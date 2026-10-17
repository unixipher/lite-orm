use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use rustls_tokio_postgres::rustls::{
    self, ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
    client::{
        danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
        verify_server_cert_signed_by_trust_anchor,
    },
    crypto::{
        CryptoProvider, WebPkiSupportedAlgorithms, verify_tls12_signature, verify_tls13_signature,
    },
    pki_types::{CertificateDer, ServerName, UnixTime, pem::PemObject},
    server::ParsedCertificate,
};

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TlsMode {
    Platform,
    Prefer,
    Disable,
    Require,
    VerifyCa,
    VerifyFull,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TlsOptions {
    pub connection_string: String,
    pub mode: TlsMode,
    pub root_cert: Option<PathBuf>,
}

impl TlsOptions {
    pub fn from_connection_string(input: &str) -> Result<Self> {
        if !input.starts_with("postgres://") && !input.starts_with("postgresql://") {
            return Self::from_keyword_string(input);
        }

        let (base, query) = input.split_once('?').unwrap_or((input, ""));
        let mut mode = TlsMode::Platform;
        let mut root_cert = None;
        let mut output = form_urlencoded::Serializer::new(String::new());

        for (key, value) in form_urlencoded::parse(query.as_bytes()) {
            match key.as_ref() {
                "sslmode" => {
                    mode = parse_mode(&value)?;
                    output.append_pair("sslmode", postgres_mode(mode));
                }
                "sslrootcert" => root_cert = Some(PathBuf::from(value.as_ref())),
                _ => {
                    output.append_pair(&key, &value);
                }
            }
        }

        let query = output.finish();
        let connection_string = if query.is_empty() {
            base.to_owned()
        } else {
            format!("{base}?{query}")
        };
        if root_cert.is_some() && mode == TlsMode::Platform {
            mode = TlsMode::VerifyFull;
        }
        Ok(Self {
            connection_string,
            mode,
            root_cert,
        })
    }

    fn from_keyword_string(input: &str) -> Result<Self> {
        let tokens = keyword_tokens(input)?;
        let mut mode = TlsMode::Platform;
        let mut root_cert = None;
        let mut kept = vec![];

        for token in tokens {
            let (key, value) = token.split_once('=').ok_or_else(|| {
                Error::Tls(format!("invalid PostgreSQL connection option `{token}`"))
            })?;
            let unquoted = unquote_keyword_value(value)?;
            match key {
                "sslmode" => {
                    mode = parse_mode(&unquoted)?;
                    kept.push(format!("sslmode={}", postgres_mode(mode)));
                }
                "sslrootcert" => root_cert = Some(PathBuf::from(unquoted)),
                _ => kept.push(token),
            }
        }
        if root_cert.is_some() && mode == TlsMode::Platform {
            mode = TlsMode::VerifyFull;
        }
        Ok(Self {
            connection_string: kept.join(" "),
            mode,
            root_cert,
        })
    }

    pub fn client_config(&self) -> Result<ClientConfig> {
        match self.mode {
            TlsMode::Disable => Err(Error::Tls(
                "internal error: TLS config requested for sslmode=disable".into(),
            )),
            TlsMode::Platform => rustls_tokio_postgres::config_platform_verifier()
                .map_err(|error| Error::Tls(error.to_string())),
            TlsMode::Prefer | TlsMode::Require if self.root_cert.is_none() => {
                Ok(rustls_tokio_postgres::config_no_verify())
            }
            TlsMode::Prefer | TlsMode::Require | TlsMode::VerifyCa => {
                let path = self.required_root_cert()?;
                chain_only_config(path)
            }
            TlsMode::VerifyFull => match &self.root_cert {
                Some(path) => rustls_tokio_postgres::config_from_ca_cert(path)
                    .map_err(|error| Error::Tls(format_ca_error(path, &error))),
                None => rustls_tokio_postgres::config_platform_verifier()
                    .map_err(|error| Error::Tls(error.to_string())),
            },
        }
    }

    fn required_root_cert(&self) -> Result<&Path> {
        self.root_cert.as_deref().ok_or_else(|| {
            Error::Tls("sslmode=verify-ca requires sslrootcert=/path/to/server-ca.pem".into())
        })
    }
}

fn parse_mode(value: &str) -> Result<TlsMode> {
    match value {
        "disable" => Ok(TlsMode::Disable),
        "prefer" => Ok(TlsMode::Prefer),
        "require" => Ok(TlsMode::Require),
        "verify-ca" => Ok(TlsMode::VerifyCa),
        "verify-full" => Ok(TlsMode::VerifyFull),
        _ => Err(Error::Tls(format!(
            "unsupported sslmode `{value}`; expected disable, prefer, require, verify-ca, or verify-full"
        ))),
    }
}

fn postgres_mode(mode: TlsMode) -> &'static str {
    match mode {
        TlsMode::Disable => "disable",
        TlsMode::Prefer | TlsMode::Platform => "prefer",
        TlsMode::Require | TlsMode::VerifyCa | TlsMode::VerifyFull => "require",
    }
}

fn keyword_tokens(input: &str) -> Result<Vec<String>> {
    let mut tokens = vec![];
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for character in input.chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else {
            match character {
                '\\' => {
                    current.push(character);
                    escaped = true;
                }
                '\'' => {
                    current.push(character);
                    quoted = !quoted;
                }
                value if value.is_whitespace() && !quoted => {
                    if !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(character),
            }
        }
    }
    if quoted || escaped {
        return Err(Error::Tls(
            "unterminated quote or escape in PostgreSQL connection string".into(),
        ));
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

fn unquote_keyword_value(value: &str) -> Result<String> {
    if let Some(value) = value.strip_prefix('\'') {
        let value = value
            .strip_suffix('\'')
            .ok_or_else(|| Error::Tls("unterminated quoted PostgreSQL option".into()))?;
        let mut output = String::new();
        let mut escaped = false;
        for character in value.chars() {
            if escaped {
                output.push(character);
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else {
                output.push(character);
            }
        }
        Ok(output)
    } else {
        Ok(value.to_owned())
    }
}

fn chain_only_config(path: &Path) -> Result<ClientConfig> {
    let roots = roots_from_file(path)?;
    let provider = CryptoProvider::get_default()
        .cloned()
        .unwrap_or_else(|| Arc::new(rustls::crypto::aws_lc_rs::default_provider()));
    let supported = provider.signature_verification_algorithms;
    let verifier = ChainOnlyVerifier { roots, supported };
    ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|error| Error::Tls(error.to_string()))
        .map(|builder| {
            builder
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(verifier))
                .with_no_client_auth()
        })
}

fn roots_from_file(path: &Path) -> Result<Arc<RootCertStore>> {
    let mut roots = RootCertStore::empty();
    let certificates = CertificateDer::pem_file_iter(path).map_err(|error| {
        Error::Tls(format!(
            "could not read sslrootcert {}: {error}",
            path.display()
        ))
    })?;
    let mut count = 0;
    for certificate in certificates {
        roots
            .add(certificate.map_err(|error| {
                Error::Tls(format!(
                    "could not parse sslrootcert {}: {error}",
                    path.display()
                ))
            })?)
            .map_err(|error| {
                Error::Tls(format!(
                    "invalid certificate in sslrootcert {}: {error}",
                    path.display()
                ))
            })?;
        count += 1;
    }
    if count == 0 {
        return Err(Error::Tls(format!(
            "sslrootcert {} contains no PEM certificates",
            path.display()
        )));
    }
    Ok(Arc::new(roots))
}

fn format_ca_error(path: &Path, error: &dyn std::error::Error) -> String {
    format!("could not load sslrootcert {}: {error}", path.display())
}

struct ChainOnlyVerifier {
    roots: Arc<RootCertStore>,
    supported: WebPkiSupportedAlgorithms,
}

impl fmt::Debug for ChainOnlyVerifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("ChainOnlyVerifier").finish()
    }
}

impl ServerCertVerifier for ChainOnlyVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        let certificate = ParsedCertificate::try_from(end_entity)?;
        verify_server_cert_signed_by_trust_anchor(
            &certificate,
            &self.roots,
            intermediates,
            now,
            self.supported.all,
        )?;
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, certificate, signature, &self.supported)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, certificate, signature, &self.supported)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.supported.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_url_tls_options() {
        let options = TlsOptions::from_connection_string(
            "postgresql://user@db.example/test?connect_timeout=5&sslmode=verify-full&sslrootcert=%2Ftmp%2Fserver-ca.pem",
        )
        .unwrap();
        assert_eq!(options.mode, TlsMode::VerifyFull);
        assert_eq!(options.root_cert, Some(PathBuf::from("/tmp/server-ca.pem")));
        assert!(options.connection_string.contains("sslmode=require"));
        assert!(options.connection_string.contains("connect_timeout=5"));
        assert!(!options.connection_string.contains("sslrootcert"));
    }

    #[test]
    fn supports_disable_and_require() {
        let disabled =
            TlsOptions::from_connection_string("postgresql://localhost/test?sslmode=disable")
                .unwrap();
        assert_eq!(disabled.mode, TlsMode::Disable);
        let required =
            TlsOptions::from_connection_string("postgresql://localhost/test?sslmode=require")
                .unwrap();
        assert_eq!(required.mode, TlsMode::Require);
    }

    #[test]
    fn supports_keyword_connection_strings() {
        let options = TlsOptions::from_connection_string(
            "host=db.example user=me sslmode=verify-ca sslrootcert='/tmp/private ca.pem'",
        )
        .unwrap();
        assert_eq!(options.mode, TlsMode::VerifyCa);
        assert_eq!(
            options.root_cert,
            Some(PathBuf::from("/tmp/private ca.pem"))
        );
        assert_eq!(
            options.connection_string,
            "host=db.example user=me sslmode=require"
        );
    }

    #[test]
    fn verify_ca_requires_a_root_certificate() {
        let options =
            TlsOptions::from_connection_string("postgresql://localhost/test?sslmode=verify-ca")
                .unwrap();
        let error = options.client_config().unwrap_err().to_string();
        assert!(error.contains("requires sslrootcert"));
    }

    #[test]
    fn parses_url_with_percent_encoded_credentials() {
        let url = "postgresql://user:p%3E%5Dss%2Cword%231@localhost:5432/mydb";
        let options = TlsOptions::from_connection_string(url).unwrap();
        assert_eq!(options.mode, TlsMode::Platform);
        let config: tokio_postgres::Config = options.connection_string.parse().unwrap();
        assert_eq!(config.get_user(), Some("user"));
        assert_eq!(config.get_password(), Some(b"p>]ss,word#1" as &[u8]));
        assert_eq!(config.get_dbname(), Some("mydb"));
    }
}
