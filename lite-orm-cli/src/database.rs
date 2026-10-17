use std::collections::{HashMap, HashSet};

use heck::{ToLowerCamelCase, ToUpperCamelCase};
use lite_orm::Database;

use crate::schema::{Datasource, Enum, Field, FieldType, Generator, Model, Schema};

#[derive(Default)]
struct DatabaseState {
    tables: HashSet<(String, String)>,
    columns: HashMap<(String, String, String), ColumnState>,
    constraints: HashSet<(String, String, String)>,
}

struct ColumnState {
    data_type: String,
}

pub async fn connect(root: &std::path::Path) -> Result<Database, String> {
    let _ = dotenvy::from_path(root.join(".env"));
    let url = std::env::var("DATABASE_URL_UNPOOLED")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| {
            format!(
                "DATABASE_URL (or DATABASE_URL_UNPOOLED) is missing from {}",
                root.join(".env").display()
            )
        })?;
    Database::connect(&url)
        .await
        .map_err(|error| error.to_string())
}

pub async fn diff(db: &Database, schema: &Schema) -> Result<String, String> {
    let state = inspect(db).await?;
    let mut statements = vec![];
    let schemas: HashSet<_> = schema
        .models
        .iter()
        .map(|model| model.schema.as_str())
        .collect();
    for namespace in schemas {
        let missing_table = schema.models.iter().any(|model| {
            model.schema == namespace
                && !state
                    .tables
                    .contains(&(model.schema.clone(), model.table.clone()))
        });
        if namespace != "public" && missing_table {
            statements.push(format!("CREATE SCHEMA IF NOT EXISTS {};", quote(namespace)));
        }
    }

    for model in &schema.models {
        let table_key = (model.schema.clone(), model.table.clone());
        if !state.tables.contains(&table_key) {
            let columns = model.fields.iter().filter(|field| is_scalar(field));
            let definitions = columns
                .map(column_definition)
                .collect::<Result<Vec<_>, _>>()?;
            statements.push(format!(
                "CREATE TABLE {}.{} (\n  {}\n);",
                quote(&model.schema),
                quote(&model.table),
                definitions.join(",\n  ")
            ));
        } else {
            for field in model.fields.iter().filter(|field| is_scalar(field)) {
                let key = (
                    model.schema.clone(),
                    model.table.clone(),
                    field.column.clone(),
                );
                if let Some(existing) = state.columns.get(&key) {
                    let expected = postgres_information_type(&field.kind)?;
                    if existing.data_type != expected {
                        return Err(format!(
                            "{}.{}.{} is `{}` in PostgreSQL but `{}` in liteorm.schema; automatic destructive type changes are blocked",
                            model.schema, model.table, field.column, existing.data_type, expected
                        ));
                    }
                } else {
                    statements.push(format!(
                        "ALTER TABLE {}.{} ADD COLUMN {};",
                        quote(&model.schema),
                        quote(&model.table),
                        column_definition(field)?
                    ));
                }
            }
        }
    }

    for model in &schema.models {
        for field in model.fields.iter().filter(|field| is_scalar(field)) {
            if field.unique && !field.id {
                let name = constraint_name(&model.table, &field.column, "key");
                let key = (model.schema.clone(), model.table.clone(), name.clone());
                if !state.constraints.contains(&key) {
                    statements.push(format!(
                        "ALTER TABLE {}.{} ADD CONSTRAINT {} UNIQUE ({});",
                        quote(&model.schema),
                        quote(&model.table),
                        quote(&name),
                        quote(&field.column)
                    ));
                }
            }
        }
        for relation in model
            .fields
            .iter()
            .filter(|field| matches!(field.kind, FieldType::Relation { .. }))
        {
            let FieldType::Relation {
                model: target_name,
                many,
            } = &relation.kind
            else {
                unreachable!()
            };
            if *many || relation.relation_fields.is_empty() {
                continue;
            }
            let target = schema
                .models
                .iter()
                .find(|item| item.name == *target_name)
                .ok_or_else(|| format!("unknown relation target `{target_name}`"))?;
            let name = constraint_name(&model.table, &relation.name, "fkey");
            let key = (model.schema.clone(), model.table.clone(), name.clone());
            if !state.constraints.contains(&key) {
                let local = relation
                    .relation_fields
                    .iter()
                    .map(|value| {
                        model
                            .fields
                            .iter()
                            .find(|field| field.name == *value)
                            .map(|field| quote(&field.column))
                            .ok_or_else(|| format!("unknown relation field `{value}`"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let remote = relation
                    .relation_references
                    .iter()
                    .map(|value| {
                        target
                            .fields
                            .iter()
                            .find(|field| field.name == *value)
                            .map(|field| quote(&field.column))
                            .ok_or_else(|| format!("unknown relation reference `{value}`"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                statements.push(format!(
                    "ALTER TABLE {}.{} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}.{} ({});",
                    quote(&model.schema),
                    quote(&model.table),
                    quote(&name),
                    local.join(", "),
                    quote(&target.schema),
                    quote(&target.table),
                    remote.join(", ")
                ));
            }
        }
    }

    Ok(statements.join("\n\n"))
}

pub async fn pull(db: &Database, output: Option<String>) -> Result<Schema, String> {
    let enum_rows = db.client().query(
        "SELECT t.typname, e.enumlabel
         FROM pg_type t
         JOIN pg_enum e ON t.oid = e.enumtypid
         JOIN pg_namespace n ON t.typnamespace = n.oid
         WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
         ORDER BY t.typname, e.enumsortorder",
        &[],
    ).await.map_err(|error| error.to_string())?;
    let mut enums: Vec<Enum> = vec![];
    for row in enum_rows {
        let typname: String = row.get(0);
        let label: String = row.get(1);
        if let Some(existing) = enums.iter_mut().find(|e| e.name == typname) {
            existing.variants.push(label);
        } else {
            enums.push(Enum {
                name: typname,
                variants: vec![label],
            });
        }
    }

    let rows = db.client().query(
        "SELECT table_schema, table_name, column_name, data_type, is_nullable, column_default, is_identity, udt_name
         FROM information_schema.columns
         WHERE table_schema NOT IN ('pg_catalog', 'information_schema')
           AND table_name <> '_lite_orm_migrations'
         ORDER BY table_schema, table_name, ordinal_position",
        &[],
    ).await.map_err(|error| error.to_string())?;
    let constraint_rows = db.client().query(
        "SELECT tc.table_schema, tc.table_name, tc.constraint_name, tc.constraint_type, kcu.column_name
         FROM information_schema.table_constraints tc
         JOIN information_schema.key_column_usage kcu
           ON tc.constraint_catalog = kcu.constraint_catalog
          AND tc.constraint_schema = kcu.constraint_schema
          AND tc.constraint_name = kcu.constraint_name
         WHERE tc.table_schema NOT IN ('pg_catalog', 'information_schema')",
        &[],
    ).await.map_err(|error| error.to_string())?;
    let mut flags: HashMap<(String, String, String), (bool, bool)> = HashMap::new();
    for row in constraint_rows {
        let key = (row.get(0), row.get(1), row.get(4));
        let entry = flags.entry(key).or_default();
        match row.get::<_, String>(3).as_str() {
            "PRIMARY KEY" => entry.0 = true,
            "UNIQUE" => entry.1 = true,
            _ => {}
        }
    }
    let mut models: Vec<Model> = vec![];
    for row in rows {
        let namespace: String = row.get(0);
        let table: String = row.get(1);
        let column: String = row.get(2);
        let data_type: String = row.get(3);
        let model_index = models
            .iter()
            .position(|model| model.schema == namespace && model.table == table);
        let index = match model_index {
            Some(index) => index,
            None => {
                models.push(Model {
                    name: singular(&table).to_upper_camel_case(),
                    table: table.clone(),
                    schema: namespace.clone(),
                    fields: vec![],
                });
                models.len() - 1
            }
        };
        let (id, unique) = flags
            .get(&(namespace, table, column.clone()))
            .copied()
            .unwrap_or_default();
        let default: Option<String> = row.get(5);
        let identity: String = row.get(6);
        let udt_name: String = row.get(7);
        models[index].fields.push(Field {
            name: column.to_lower_camel_case(),
            column,
            kind: map_postgres_type(&data_type, &udt_name, &enums)?,
            optional: row.get::<_, String>(4) == "YES",
            id,
            unique,
            auto_increment: identity == "YES"
                || default
                    .as_deref()
                    .is_some_and(|value| value.starts_with("nextval(")),
            default: postgres_default(default),
            relation_fields: vec![],
            relation_references: vec![],
        });
    }
    let foreign_keys = db
        .client()
        .query(
            "SELECT tc.table_schema, tc.table_name, kcu.column_name,
                    ccu.table_schema, ccu.table_name, ccu.column_name
             FROM information_schema.table_constraints tc
             JOIN information_schema.key_column_usage kcu
               ON tc.constraint_catalog = kcu.constraint_catalog
              AND tc.constraint_schema = kcu.constraint_schema
              AND tc.constraint_name = kcu.constraint_name
             JOIN information_schema.constraint_column_usage ccu
               ON tc.constraint_catalog = ccu.constraint_catalog
              AND tc.constraint_schema = ccu.constraint_schema
              AND tc.constraint_name = ccu.constraint_name
             WHERE tc.constraint_type = 'FOREIGN KEY'
               AND tc.table_schema NOT IN ('pg_catalog', 'information_schema')",
            &[],
        )
        .await
        .map_err(|error| error.to_string())?;
    for row in foreign_keys {
        let source_schema: String = row.get(0);
        let source_table: String = row.get(1);
        let source_column: String = row.get(2);
        let target_schema: String = row.get(3);
        let target_table: String = row.get(4);
        let target_column: String = row.get(5);
        let Some(source_index) = models
            .iter()
            .position(|model| model.schema == source_schema && model.table == source_table)
        else {
            continue;
        };
        let Some(target_index) = models
            .iter()
            .position(|model| model.schema == target_schema && model.table == target_table)
        else {
            continue;
        };
        let target_name = models[target_index].name.clone();
        let target_field = models[target_index]
            .fields
            .iter()
            .find(|field| field.column == target_column)
            .map(|field| field.name.clone())
            .unwrap_or_else(|| target_column.to_lower_camel_case());
        let source_field = models[source_index]
            .fields
            .iter()
            .find(|field| field.column == source_column)
            .map(|field| field.name.clone())
            .unwrap_or_else(|| source_column.to_lower_camel_case());
        let mut relation_name = target_name.to_lower_camel_case();
        if models[source_index]
            .fields
            .iter()
            .any(|field| field.name == relation_name)
        {
            relation_name.push_str("Relation");
        }
        models[source_index].fields.push(Field {
            name: relation_name.clone(),
            column: relation_name,
            kind: FieldType::Relation {
                model: target_name,
                many: false,
            },
            optional: false,
            id: false,
            unique: false,
            auto_increment: false,
            default: None,
            relation_fields: vec![source_field],
            relation_references: vec![target_field],
        });
        if source_index != target_index {
            let source_name = models[source_index].name.clone();
            let mut reverse_name = models[source_index].table.to_lower_camel_case();
            if models[target_index]
                .fields
                .iter()
                .any(|field| field.name == reverse_name)
            {
                reverse_name.push_str("Relations");
            }
            models[target_index].fields.push(Field {
                name: reverse_name.clone(),
                column: reverse_name,
                kind: FieldType::Relation {
                    model: source_name,
                    many: true,
                },
                optional: false,
                id: false,
                unique: false,
                auto_increment: false,
                default: None,
                relation_fields: vec![],
                relation_references: vec![],
            });
        }
    }
    Ok(Schema {
        datasource: Datasource {
            provider: Some("postgresql".into()),
            url_env: Some("DATABASE_URL".into()),
            direct_url_env: None,
        },
        generator: Generator { output },
        enums,
        models,
    })
}

async fn inspect(db: &Database) -> Result<DatabaseState, String> {
    let mut state = DatabaseState::default();
    for row in db.client().query(
        "SELECT table_schema, table_name FROM information_schema.tables
         WHERE table_type = 'BASE TABLE' AND table_schema NOT IN ('pg_catalog', 'information_schema')",
        &[],
    ).await.map_err(|error| error.to_string())? {
        state.tables.insert((row.get(0), row.get(1)));
    }
    for row in db.client().query(
        "SELECT table_schema, table_name, column_name, data_type FROM information_schema.columns
         WHERE table_schema NOT IN ('pg_catalog', 'information_schema')",
        &[],
    ).await.map_err(|error| error.to_string())? {
        state.columns.insert((row.get(0), row.get(1), row.get(2)), ColumnState { data_type: row.get(3) });
    }
    for row in db.client().query(
        "SELECT table_schema, table_name, constraint_name FROM information_schema.table_constraints
         WHERE table_schema NOT IN ('pg_catalog', 'information_schema')",
        &[],
    ).await.map_err(|error| error.to_string())? {
        state.constraints.insert((row.get(0), row.get(1), row.get(2)));
    }
    Ok(state)
}

fn column_definition(field: &Field) -> Result<String, String> {
    let mut sql = format!("{} {}", quote(&field.column), postgres_type(&field.kind)?);
    if field.auto_increment {
        sql.push_str(" GENERATED BY DEFAULT AS IDENTITY");
    }
    if !field.optional {
        sql.push_str(" NOT NULL");
    }
    if let Some(default) = &field.default
        && !field.auto_increment
    {
        sql.push_str(" DEFAULT ");
        sql.push_str(&sql_default(default)?);
    }
    if field.id {
        sql.push_str(" PRIMARY KEY");
    }
    Ok(sql)
}

fn postgres_type(kind: &FieldType) -> Result<String, String> {
    match kind {
        FieldType::Int32 => Ok("INTEGER".into()),
        FieldType::Int64 => Ok("BIGINT".into()),
        FieldType::Float => Ok("DOUBLE PRECISION".into()),
        FieldType::Boolean => Ok("BOOLEAN".into()),
        FieldType::String => Ok("TEXT".into()),
        FieldType::Bytes => Ok("BYTEA".into()),
        FieldType::Json => Ok("JSONB".into()),
        FieldType::Uuid => Ok("UUID".into()),
        FieldType::Date => Ok("DATE".into()),
        FieldType::Time => Ok("TIME".into()),
        FieldType::DateTime => Ok("TIMESTAMPTZ".into()),
        FieldType::Enum(name) => Ok(quote(name)),
        FieldType::Array(inner) => Ok(format!("{}[]", postgres_type(inner)?)),
        FieldType::Unsupported(name) => Ok(quote(name)),
        FieldType::Relation { .. } => Err("relation fields are not PostgreSQL columns".into()),
    }
}

fn postgres_information_type(kind: &FieldType) -> Result<String, String> {
    Ok(match kind {
        FieldType::Int32 => "integer",
        FieldType::Int64 => "bigint",
        FieldType::Float => "double precision",
        FieldType::Boolean => "boolean",
        FieldType::String => "text",
        FieldType::Bytes => "bytea",
        FieldType::Json => "jsonb",
        FieldType::Uuid => "uuid",
        FieldType::Date => "date",
        FieldType::Time => "time without time zone",
        FieldType::DateTime => "timestamp with time zone",
        FieldType::Enum(_) => "USER-DEFINED",
        FieldType::Array(_) => "ARRAY",
        FieldType::Unsupported(_) => "USER-DEFINED",
        FieldType::Relation { .. } => {
            return Err("relation fields are not PostgreSQL columns".into());
        }
    }
    .into())
}

fn map_postgres_type(
    data_type: &str,
    udt_name: &str,
    enums: &[crate::schema::Enum],
) -> Result<FieldType, String> {
    match data_type {
        "smallint" | "integer" => Ok(FieldType::Int32),
        "bigint" => Ok(FieldType::Int64),
        "real" | "double precision" | "numeric" => Ok(FieldType::Float),
        "boolean" => Ok(FieldType::Boolean),
        "text" | "character varying" | "character" => Ok(FieldType::String),
        "bytea" => Ok(FieldType::Bytes),
        "json" | "jsonb" => Ok(FieldType::Json),
        "uuid" => Ok(FieldType::Uuid),
        "date" => Ok(FieldType::Date),
        "time without time zone" | "time with time zone" => Ok(FieldType::Time),
        "timestamp without time zone" | "timestamp with time zone" => Ok(FieldType::DateTime),
        "USER-DEFINED" => {
            if enums.iter().any(|e| e.name == udt_name) {
                Ok(FieldType::Enum(udt_name.to_string()))
            } else if udt_name == "citext" {
                Ok(FieldType::String)
            } else {
                Ok(FieldType::Unsupported(udt_name.to_string()))
            }
        }
        "ARRAY" => {
            let element_udt = udt_name.strip_prefix('_').unwrap_or(udt_name);
            let element_type = match element_udt {
                "int2" | "int4" => FieldType::Int32,
                "int8" => FieldType::Int64,
                "float4" | "float8" | "numeric" => FieldType::Float,
                "bool" => FieldType::Boolean,
                "text" | "varchar" | "bpchar" => FieldType::String,
                "bytea" => FieldType::Bytes,
                "json" | "jsonb" => FieldType::Json,
                "uuid" => FieldType::Uuid,
                "date" => FieldType::Date,
                "time" | "timetz" => FieldType::Time,
                "timestamp" | "timestamptz" => FieldType::DateTime,
                other => {
                    if enums.iter().any(|e| e.name == other) {
                        FieldType::Enum(other.to_string())
                    } else {
                        FieldType::Unsupported(other.to_string())
                    }
                }
            };
            Ok(FieldType::Array(Box::new(element_type)))
        }
        _ => Err(format!(
            "PostgreSQL type `{data_type}` is not supported by lite-orm yet"
        )),
    }
}

fn postgres_default(value: Option<String>) -> Option<String> {
    let value = value?;
    if value.starts_with("nextval(") {
        Some("autoincrement()".into())
    } else if value.eq_ignore_ascii_case("CURRENT_TIMESTAMP") || value.eq_ignore_ascii_case("now()")
    {
        Some("now()".into())
    } else if value == "true" || value == "false" {
        Some(value)
    } else {
        None
    }
}

fn sql_default(value: &str) -> Result<String, String> {
    match value {
        "now()" => Ok("CURRENT_TIMESTAMP".into()),
        "true" | "false" => Ok(value.into()),
        value if value.starts_with('"') && value.ends_with('"') => Ok(format!(
            "'{}'",
            value[1..value.len() - 1].replace('\'', "''")
        )),
        value if value.parse::<f64>().is_ok() => Ok(value.into()),
        "autoincrement()" => Err("autoincrement() is only valid on integer identity fields".into()),
        _ => Err(format!("unsupported default `{value}`")),
    }
}

fn is_scalar(field: &Field) -> bool {
    !matches!(field.kind, FieldType::Relation { .. })
}
fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
fn constraint_name(table: &str, field: &str, suffix: &str) -> String {
    format!("liteorm_{table}_{field}_{suffix}")
}

fn singular(value: &str) -> String {
    if let Some(stem) = value.strip_suffix("ies") {
        format!("{stem}y")
    } else if value.ends_with("ss") {
        value.to_owned()
    } else if let Some(stem) = value.strip_suffix('s') {
        stem.to_owned()
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn singularizes_common_table_names() {
        assert_eq!(super::singular("categories"), "category");
        assert_eq!(super::singular("users"), "user");
        assert_eq!(super::singular("posts"), "post");
        assert_eq!(super::singular("address"), "address");
    }
}
