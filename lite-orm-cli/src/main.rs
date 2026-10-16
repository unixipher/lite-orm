mod database;
mod generate;
mod migrations;
mod schema;
mod studio;

use std::path::{Path, PathBuf};

struct Arguments {
    values: Vec<String>,
    schema: PathBuf,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("liteorm: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let arguments = arguments()?;
    let words: Vec<_> = arguments.values.iter().map(String::as_str).collect();
    match words.as_slice() {
        [] | ["help"] | ["--help"] | ["-h"] => {
            print_help();
            Ok(())
        }
        ["init"] => init_command(&arguments.schema),
        ["generate"] => generate_command(&arguments.schema, None),
        ["generate", "--output", output] => {
            generate_command(&arguments.schema, Some(Path::new(output)))
        }
        ["check"] | ["schema", "check"] => check_command(&arguments.schema),
        ["format"] | ["schema", "format"] => format_command(&arguments.schema),
        ["migrate", "dev"] => migrate_dev(&arguments.schema, "migration").await,
        ["migrate", "dev", "--name", name] => migrate_dev(&arguments.schema, name).await,
        ["migrate", "deploy"] => migrate_deploy(&arguments.schema).await,
        ["migrate", "status"] => migrate_status(&arguments.schema).await,
        ["db", "push"] => db_push(&arguments.schema).await,
        ["db", "pull"] => db_pull(&arguments.schema).await,
        ["db", "seed"] => db_seed(&arguments.schema, Path::new("seed.sql")).await,
        ["db", "seed", "--file", file] => db_seed(&arguments.schema, Path::new(file)).await,
        ["studio"] | ["studio", ..] => studio::launch(&arguments.schema),
        _ => Err(format!(
            "unknown command `{}`; run `liteorm help`",
            words.join(" ")
        )),
    }
}

fn arguments() -> Result<Arguments, String> {
    let mut input = std::env::args().skip(1);
    let mut values = vec![];
    let mut schema = PathBuf::from("liteorm.schema");
    while let Some(argument) = input.next() {
        if argument == "--schema" {
            schema = PathBuf::from(input.next().ok_or("--schema requires a path")?);
        } else {
            values.push(argument);
        }
    }
    Ok(Arguments { values, schema })
}

fn read_schema(path: &Path) -> Result<schema::Schema, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    schema::parse(&source).map_err(|errors| errors.join("\n"))
}

fn generate_command(path: &Path, output_override: Option<&Path>) -> Result<(), String> {
    let schema = read_schema(path)?;
    let output = output_override
        .map(Path::to_path_buf)
        .or_else(|| schema.generator.output.clone().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("src/liteorm.rs"));
    let output = project_root(path).join(output);
    let code = generate::rust(&schema);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&output, code).map_err(|error| error.to_string())?;
    println!(
        "Generated {} structs into {}",
        schema.models.len(),
        output.display()
    );
    Ok(())
}

fn check_command(path: &Path) -> Result<(), String> {
    let schema = read_schema(path)?;
    println!(
        "{} is valid ({} structs)",
        path.display(),
        schema.models.len()
    );
    Ok(())
}

fn format_command(path: &Path) -> Result<(), String> {
    let parsed = read_schema(path)?;
    std::fs::write(path, schema::format(&parsed)).map_err(|error| error.to_string())?;
    println!("Formatted {}", path.display());
    Ok(())
}

fn init_command(path: &Path) -> Result<(), String> {
    if path.exists() {
        return Err(format!("{} already exists", path.display()));
    }
    std::fs::write(path, include_str!("../starter.schema")).map_err(|error| error.to_string())?;
    let env_path = project_root(path).join(".env");
    if !env_path.exists() {
        std::fs::write(
            &env_path,
            "DATABASE_URL=postgresql://postgres:postgres@localhost:5432/postgres\n",
        )
        .map_err(|error| error.to_string())?;
    }
    println!("Created {} and {}", path.display(), env_path.display());
    Ok(())
}

async fn migrate_dev(path: &Path, name: &str) -> Result<(), String> {
    let parsed = read_schema(path)?;
    let db = database::connect(project_root(path)).await?;
    migrations::dev(&db, &parsed, name, project_root(path)).await
}

async fn migrate_deploy(path: &Path) -> Result<(), String> {
    let parsed = read_schema(path)?;
    let db = database::connect(project_root(path)).await?;
    migrations::deploy(&db, project_root(path), migrations::history_schema(&parsed)).await
}

async fn migrate_status(path: &Path) -> Result<(), String> {
    let parsed = read_schema(path)?;
    let db = database::connect(project_root(path)).await?;
    migrations::status(&db, project_root(path), migrations::history_schema(&parsed)).await
}

async fn db_push(path: &Path) -> Result<(), String> {
    let parsed = read_schema(path)?;
    let db = database::connect(project_root(path)).await?;
    let sql = database::diff(&db, &parsed).await?;
    if sql.trim().is_empty() {
        println!("Database is already in sync");
        return Ok(());
    }
    db.execute_batch(&sql)
        .await
        .map_err(|error| error.to_string())?;
    println!("Database schema updated");
    Ok(())
}

async fn db_pull(path: &Path) -> Result<(), String> {
    let existing = if path.exists() {
        Some(read_schema(path)?)
    } else {
        None
    };
    let output = existing
        .as_ref()
        .and_then(|schema| schema.generator.output.clone());
    let db = database::connect(project_root(path)).await?;
    let pulled = database::pull(&db, output).await?;
    std::fs::write(path, schema::format(&pulled)).map_err(|error| error.to_string())?;
    println!(
        "Introspected {} structs into {}",
        pulled.models.len(),
        path.display()
    );
    Ok(())
}

async fn db_seed(schema_path: &Path, seed: &Path) -> Result<(), String> {
    let _ = read_schema(schema_path)?;
    let seed = project_root(schema_path).join(seed);
    let sql = std::fs::read_to_string(&seed)
        .map_err(|error| format!("could not read {}: {error}", seed.display()))?;
    let db = database::connect(project_root(schema_path)).await?;
    db.execute_batch(&sql)
        .await
        .map_err(|error| error.to_string())?;
    println!("Executed {}", seed.display());
    Ok(())
}

pub(crate) fn project_root(path: &Path) -> &Path {
    path.parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

fn print_help() {
    println!(
        r#"liteorm — schema-first PostgreSQL ORM

  liteorm init
  liteorm generate
  liteorm generate --output <path>
  liteorm schema check
  liteorm schema format
  liteorm migrate dev [--name NAME]
  liteorm migrate deploy
  liteorm migrate status
  liteorm db push
  liteorm db pull
  liteorm db seed [--file FILE]
  liteorm studio

Global option: --schema PATH"#
    );
}
