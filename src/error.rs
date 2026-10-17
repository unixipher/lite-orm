use std::fmt;

/// Errors produced by lite-orm.
#[derive(Debug)]
pub enum Error {
    Database(tokio_postgres::Error),
    Tls(String),
    InvalidQuery(String),
    Migration(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => {
                write!(f, "database error: {}", display_error_chain(error))
            }
            Self::Tls(message) => write!(f, "TLS configuration error: {message}"),
            Self::InvalidQuery(message) => f.write_str(message),
            Self::Migration(message) => write!(f, "migration error: {message}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::Tls(_) | Self::InvalidQuery(_) | Self::Migration(_) => None,
        }
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(value: tokio_postgres::Error) -> Self {
        Self::Database(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn display_error_chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        let detail = cause.to_string();
        if !detail.is_empty() && !message.ends_with(&detail) {
            message.push_str(": ");
            message.push_str(&detail);
        }
        source = cause.source();
    }
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Outer(std::io::Error);

    impl fmt::Display for Outer {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("TLS handshake failed")
        }
    }

    impl std::error::Error for Outer {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    #[test]
    fn displays_nested_tls_causes() {
        let error = Outer(std::io::Error::other("unknown certificate authority"));
        assert_eq!(
            display_error_chain(&error),
            "TLS handshake failed: unknown certificate authority"
        );
    }
}
