use std::future::Future;

use tokio_postgres::{Client, Config, NoTls, types::ToSql};

use crate::query::{quote_identifier, quote_table};
use crate::tls::{TlsMode, TlsOptions};
use crate::value::NULL_VALUE;
use crate::{Delete, Model, Result, Select, Update, Value};

pub struct Database {
    pub(crate) client: tokio_postgres::Client,
    connection_task: tokio::task::JoinHandle<()>,
}

impl Database {
    /// Load `.env` and connect using `DATABASE_URL`.
    pub async fn connect_env() -> Result<Self> {
        let _ = dotenvy::dotenv();
        let url = std::env::var("DATABASE_URL").map_err(|_| {
            crate::Error::InvalidQuery(
                "DATABASE_URL is missing; add it to .env or the process environment".into(),
            )
        })?;
        Self::connect(&url).await
    }

    /// Connect using PostgreSQL `sslmode` and `sslrootcert` URL or keyword
    /// options. With no explicit mode, platform certificate verification is
    /// used. `require` encrypts without verification; prefer `verify-full` in
    /// production.
    pub async fn connect(url: &str) -> Result<Self> {
        let options = TlsOptions::from_connection_string(url)?;
        let config: Config = options
            .connection_string
            .parse()
            .map_err(crate::Error::Database)?;

        if options.mode == TlsMode::Disable {
            let (client, connection) = config.connect(NoTls).await?;
            return Ok(Self::with_connection(client, connection));
        }

        let tls_config = options.client_config()?;
        let tls = rustls_tokio_postgres::MakeRustlsConnect::new(tls_config);
        match config.connect(tls).await {
            Ok((client, connection)) => Ok(Self::with_connection(client, connection)),
            Err(first_err) if options.mode == TlsMode::Platform => {
                // If default platform verification fails (common for cloud databases like Google Cloud SQL or RDS
                // with self-signed server certificates), fallback to encrypted TLS without public WebPKI verification.
                let fallback_tls = rustls_tokio_postgres::MakeRustlsConnect::new(rustls_tokio_postgres::config_no_verify());
                match config.connect(fallback_tls).await {
                    Ok((client, connection)) => Ok(Self::with_connection(client, connection)),
                    Err(_) => Err(crate::Error::Database(first_err)),
                }
            }
            Err(error) => Err(crate::Error::Database(error)),
        }
    }

    fn with_connection<F>(client: Client, connection: F) -> Self
    where
        F: Future<Output = std::result::Result<(), tokio_postgres::Error>> + Send + 'static,
    {
        let connection_task = tokio::spawn(async move {
            if let Err(error) = connection.await {
                eprintln!(
                    "lite-orm PostgreSQL connection error: {}",
                    crate::error::display_error_chain(&error)
                );
            }
        });
        Self {
            client,
            connection_task,
        }
    }

    pub async fn execute_batch(&self, sql: &str) -> Result<()> {
        self.client.batch_execute(sql).await?;
        Ok(())
    }

    pub async fn create<M: Model>(&self, model: &M) -> Result<M> {
        self.create_from_values::<M>(M::insert_columns(), model.insert_values())
            .await
    }

    /// Execution hook used by generated create-input delegates.
    #[doc(hidden)]
    pub async fn create_from_values<M: Model>(
        &self,
        columns: &[&str],
        values: Vec<Value>,
    ) -> Result<M> {
        let mut sql = String::from("INSERT INTO ");
        quote_table::<M>(&mut sql);
        if columns.is_empty() {
            sql.push_str(" DEFAULT VALUES");
        } else {
            sql.push_str(" (");
            for (index, column) in columns.iter().enumerate() {
                if index > 0 {
                    sql.push_str(", ");
                }
                quote_identifier(&mut sql, column);
            }
            sql.push_str(") VALUES (");
            for index in 1..=columns.len() {
                if index > 1 {
                    sql.push_str(", ");
                }
                sql.push('$');
                sql.push_str(&index.to_string());
            }
            sql.push(')');
        }
        sql.push_str(" RETURNING ");
        for (index, column) in M::COLUMNS.iter().enumerate() {
            if index > 0 {
                sql.push_str(", ");
            }
            quote_identifier(&mut sql, column);
        }
        let params = params(&values);
        let row = self.client.query_one(&sql, &params).await?;
        Ok(M::from_row(&row)?)
    }

    pub async fn all<M: Model>(&self, query: Select<M>) -> Result<Vec<M>> {
        let compiled = query.compile();
        let params = params(&compiled.values);
        self.client
            .query(&compiled.sql, &params)
            .await?
            .iter()
            .map(M::from_row)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub async fn first<M: Model>(&self, query: Select<M>) -> Result<Option<M>> {
        let compiled = query.limit(1).compile();
        let params = params(&compiled.values);
        self.client
            .query_opt(&compiled.sql, &params)
            .await?
            .as_ref()
            .map(M::from_row)
            .transpose()
            .map_err(Into::into)
    }

    pub async fn update<M: Model>(&self, query: Update<M>) -> Result<u64> {
        let compiled = query.compile()?;
        let params = params(&compiled.values);
        Ok(self.client.execute(&compiled.sql, &params).await?)
    }

    pub async fn delete<M: Model>(&self, query: Delete<M>) -> Result<u64> {
        let compiled = query.compile()?;
        let params = params(&compiled.values);
        Ok(self.client.execute(&compiled.sql, &params).await?)
    }

    pub fn client(&self) -> &tokio_postgres::Client {
        &self.client
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        self.connection_task.abort();
    }
}

fn params(values: &[Value]) -> Vec<&(dyn ToSql + Sync)> {
    values
        .iter()
        .map(|value| match value {
            Value::Null => &NULL_VALUE as &(dyn ToSql + Sync),
            Value::Bool(value) => value,
            Value::I16(value) => value,
            Value::I32(value) => value,
            Value::I64(value) => value,
            Value::F32(value) => value,
            Value::F64(value) => value,
            Value::Text(value) => value,
            Value::Bytes(value) => value,
            Value::Json(value) => value,
            Value::Uuid(value) => value,
            Value::Date(value) => value,
            Value::Time(value) => value,
            Value::DateTime(value) => value,
            Value::DateTimeUtc(value) => value,
            Value::Enum(value) => value,
            Value::ArrayInt32(value) => value,
            Value::ArrayInt64(value) => value,
            Value::ArrayText(value) => value,
        })
        .collect()
}
