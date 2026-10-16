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
            Self::Database(error) => write!(f, "database error: {error}"),
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
