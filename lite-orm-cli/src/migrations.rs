use std::path::{Path, PathBuf};

use chrono::Utc;
use lite_orm::Database;

use crate::{database, schema::Schema};

pub async fn dev(db: &Database, schema: &Schema, name: &str, root: &Path) -> Result<(), String> {
    let sql = database::diff(db, schema).await?;
    if sql.trim().is_empty() {
        println!("Database is already in sync; no migration created");
        return Ok(());
    }
    let safe_name: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let version = format!(
        "{}_{}",
        Utc::now().format("%Y%m%d%H%M%S"),
        safe_name.trim_matches('_')
    );
    let directory = root.join("migrations").join(&version);
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    std::fs::write(directory.join("migration.sql"), format!("{sql}\n"))
        .map_err(|error| error.to_string())?;
    deploy(db, root, history_schema(schema)).await?;
    println!("Created and applied migration {version}");
    Ok(())
}

pub async fn deploy(db: &Database, root: &Path, namespace: &str) -> Result<(), String> {
    ensure_table(db, namespace).await?;
    let lock_key = format!("lite_orm_migrations:{namespace}");
    lock(db, &lock_key).await?;
    let result = deploy_locked(db, root, namespace).await;
    let unlock_result = unlock(db, &lock_key).await;
    match (result, unlock_result) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

async fn deploy_locked(db: &Database, root: &Path, namespace: &str) -> Result<(), String> {
    // This read must happen after the session-level lock is acquired. Otherwise a
    // waiting deployer can replay the plan computed before the winner committed.
    let applied = applied_versions(db, namespace).await?;
    let migrations = migration_files(root)?;
    let mut count = 0;
    for (version, path) in migrations {
        if applied.contains(&version) {
            continue;
        }
        let sql = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let statement = format!(
            "BEGIN;\n{sql}\nINSERT INTO {}._lite_orm_migrations (version, description) VALUES ({}, {});\nCOMMIT;",
            quote(namespace),
            literal(&version),
            literal(&version)
        );
        db.execute_batch(&statement)
            .await
            .map_err(|error| format!("{}: {error}", path.display()))?;
        println!("Applied {version}");
        count += 1;
    }
    if count == 0 {
        println!("No pending migrations");
    }
    Ok(())
}

async fn lock(db: &Database, key: &str) -> Result<(), String> {
    db.client()
        .query_one("SELECT pg_advisory_lock(hashtext($1))", &[&key])
        .await
        .map(|_| ())
        .map_err(|error| format!("could not acquire migration lock: {error}"))
}

async fn unlock(db: &Database, key: &str) -> Result<(), String> {
    let row = db
        .client()
        .query_one("SELECT pg_advisory_unlock(hashtext($1))", &[&key])
        .await
        .map_err(|error| format!("could not release migration lock: {error}"))?;
    if row.get::<_, bool>(0) {
        Ok(())
    } else {
        Err("migration lock was not held by this connection".into())
    }
}

pub async fn status(db: &Database, root: &Path, namespace: &str) -> Result<(), String> {
    ensure_table(db, namespace).await?;
    let applied = applied_versions(db, namespace).await?;
    let migrations = migration_files(root)?;
    if migrations.is_empty() {
        println!("No migrations found");
        return Ok(());
    }
    let mut pending = 0;
    for (version, _) in migrations {
        if applied.contains(&version) {
            println!("applied  {version}");
        } else {
            println!("pending  {version}");
            pending += 1;
        }
    }
    println!("{pending} pending migration(s)");
    Ok(())
}

async fn ensure_table(db: &Database, namespace: &str) -> Result<(), String> {
    db.execute_batch(&format!(
        "CREATE SCHEMA IF NOT EXISTS {};
        CREATE TABLE IF NOT EXISTS {}._lite_orm_migrations (
            version TEXT PRIMARY KEY,
            description TEXT NOT NULL,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );",
        quote(namespace),
        quote(namespace)
    ))
    .await
    .map_err(|error| error.to_string())
}

async fn applied_versions(
    db: &Database,
    namespace: &str,
) -> Result<std::collections::HashSet<String>, String> {
    let rows = db
        .client()
        .query(
            &format!(
                "SELECT version FROM {}._lite_orm_migrations",
                quote(namespace)
            ),
            &[],
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(rows.into_iter().map(|row| row.get(0)).collect())
}

fn migration_files(root: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let directory = root.join("migrations");
    if !directory.exists() {
        return Ok(vec![]);
    }
    let mut migrations = vec![];
    for entry in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let path = entry.path().join("migration.sql");
        if path.exists() {
            migrations.push((entry.file_name().to_string_lossy().into_owned(), path));
        }
    }
    migrations.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(migrations)
}

fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub fn history_schema(schema: &Schema) -> &str {
    schema
        .models
        .first()
        .map_or("public", |model| model.schema.as_str())
}

fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
