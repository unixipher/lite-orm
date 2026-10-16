use std::collections::HashSet;

use crate::{Database, Error, Result};

#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub version: &'static str,
    pub description: &'static str,
    pub sql: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub applied: Vec<String>,
    pub already_applied: usize,
}

impl Database {
    pub async fn migrate(&mut self, migrations: &[Migration]) -> Result<MigrationReport> {
        validate_migrations(migrations)?;
        let transaction = self.client.transaction().await?;
        transaction
            .batch_execute(
                "SELECT pg_advisory_xact_lock(hashtext('lite_orm_migrations'));
             CREATE TABLE IF NOT EXISTS public._lite_orm_migrations (
                version TEXT PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
             );",
            )
            .await?;
        let rows = transaction
            .query("SELECT version FROM public._lite_orm_migrations", &[])
            .await?;
        let existing: HashSet<String> = rows.into_iter().map(|row| row.get(0)).collect();
        let mut applied = vec![];
        for migration in migrations {
            if existing.contains(migration.version) {
                continue;
            }
            transaction.batch_execute(migration.sql).await?;
            transaction.execute(
                "INSERT INTO public._lite_orm_migrations (version, description) VALUES ($1, $2)",
                &[&migration.version, &migration.description],
            ).await?;
            applied.push(migration.version.to_owned());
        }
        transaction.commit().await?;
        Ok(MigrationReport {
            already_applied: migrations.len() - applied.len(),
            applied,
        })
    }
}

fn validate_migrations(migrations: &[Migration]) -> Result<()> {
    let mut versions = HashSet::new();
    for migration in migrations {
        if migration.version.trim().is_empty() {
            return Err(Error::Migration("migration version cannot be empty".into()));
        }
        if !versions.insert(migration.version) {
            return Err(Error::Migration(format!(
                "duplicate migration version: {}",
                migration.version
            )));
        }
    }
    Ok(())
}
