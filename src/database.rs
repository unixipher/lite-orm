use tokio_postgres::types::ToSql;

use crate::query::{quote_identifier, quote_table};
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

    pub async fn connect(url: &str) -> Result<Self> {
        let tls_config = rustls_tokio_postgres::config_platform_verifier()
            .map_err(|error| crate::Error::Tls(error.to_string()))?;
        let tls = rustls_tokio_postgres::MakeRustlsConnect::new(tls_config);
        let (client, connection) = tokio_postgres::connect(url, tls).await?;
        let connection_task = tokio::spawn(async move {
            if let Err(error) = connection.await {
                eprintln!("lite-orm PostgreSQL connection error: {error}");
            }
        });
        Ok(Self {
            client,
            connection_task,
        })
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
        })
        .collect()
}
