use std::sync::Arc;
use lite_orm::Database;
use serde_json::{Map, Value};
use tokio_postgres::{Row, SimpleQueryMessage};

use crate::schema::{Field, FieldType, Model};

pub fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub fn table_name(model: &Model) -> String {
    format!("{}.{}", quote(&model.schema), quote(&model.table))
}

pub fn scalar(field: &Field) -> bool {
    !matches!(field.kind, FieldType::Relation { .. })
}

pub async fn connect_db() -> Result<Arc<Database>, String> {
    let _ = dotenvy::dotenv();
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost:5432/liteorm_demo".to_string());
    Database::connect(&url).await.map(Arc::new).map_err(|e| e.to_string())
}

#[derive(Clone, Debug, PartialEq)]
pub struct FilterRule {
    pub field: String,
    pub op: FilterOp,
    pub value: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FilterOp {
    Contains,
    Equals,
    StartsWith,
    EndsWith,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    IsNull,
    IsNotNull,
    IsTrue,
    IsFalse,
}

impl FilterOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            FilterOp::Contains => "contains",
            FilterOp::Equals => "equals",
            FilterOp::StartsWith => "starts_with",
            FilterOp::EndsWith => "ends_with",
            FilterOp::GreaterThan => "gt",
            FilterOp::LessThan => "lt",
            FilterOp::GreaterOrEqual => "gte",
            FilterOp::LessOrEqual => "lte",
            FilterOp::IsNull => "is_null",
            FilterOp::IsNotNull => "is_not_null",
            FilterOp::IsTrue => "is_true",
            FilterOp::IsFalse => "is_false",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "equals" => FilterOp::Equals,
            "starts_with" => FilterOp::StartsWith,
            "ends_with" => FilterOp::EndsWith,
            "gt" => FilterOp::GreaterThan,
            "lt" => FilterOp::LessThan,
            "gte" => FilterOp::GreaterOrEqual,
            "lte" => FilterOp::LessOrEqual,
            "is_null" => FilterOp::IsNull,
            "is_not_null" => FilterOp::IsNotNull,
            "is_true" => FilterOp::IsTrue,
            "is_false" => FilterOp::IsFalse,
            _ => FilterOp::Contains,
        }
    }

    pub fn requires_value(&self) -> bool {
        !matches!(
            self,
            FilterOp::IsNull | FilterOp::IsNotNull | FilterOp::IsTrue | FilterOp::IsFalse
        )
    }
}

pub async fn query_rows(
    db: &Database,
    model: &Model,
    page: usize,
    page_size: usize,
    sort: Option<&str>,
    direction: Option<&str>,
    search: Option<&str>,
    filters: &[FilterRule],
) -> Result<(Vec<Map<String, Value>>, i64), String> {
    let fields: Vec<_> = model.fields.iter().filter(|f| scalar(f)).collect();
    let mut where_conditions: Vec<String> = Vec::new();
    let mut param_strings: Vec<String> = Vec::new();

    let search_pattern = search
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!("%{}%", s.trim()));

    if let Some(pattern) = search_pattern {
        let searchable: Vec<_> = fields
            .iter()
            .filter(|f| matches!(f.kind, FieldType::String))
            .collect();
        if !searchable.is_empty() {
            param_strings.push(pattern);
            let p_idx = param_strings.len();
            let search_clause = searchable
                .iter()
                .map(|f| format!("{} ILIKE ${p_idx}", quote(&f.column)))
                .collect::<Vec<_>>()
                .join(" OR ");
            where_conditions.push(format!("({search_clause})"));
        }
    }

    for rule in filters {
        if let Some(f) = fields.iter().find(|field| field.name == rule.field) {
            let col = quote(&f.column);
            match rule.op {
                FilterOp::IsNull => {
                    where_conditions.push(format!("{col} IS NULL"));
                }
                FilterOp::IsNotNull => {
                    where_conditions.push(format!("{col} IS NOT NULL"));
                }
                FilterOp::IsTrue => {
                    where_conditions.push(format!("{col} IS TRUE"));
                }
                FilterOp::IsFalse => {
                    where_conditions.push(format!("{col} IS FALSE"));
                }
                _ => {
                    let trimmed = rule.value.trim();
                    if !trimmed.is_empty() {
                        match rule.op {
                            FilterOp::Contains => {
                                param_strings.push(format!("%{trimmed}%"));
                                let p_idx = param_strings.len();
                                where_conditions.push(format!("{col}::text ILIKE ${p_idx}"));
                            }
                            FilterOp::StartsWith => {
                                param_strings.push(format!("{trimmed}%"));
                                let p_idx = param_strings.len();
                                where_conditions.push(format!("{col}::text ILIKE ${p_idx}"));
                            }
                            FilterOp::EndsWith => {
                                param_strings.push(format!("%{trimmed}"));
                                let p_idx = param_strings.len();
                                where_conditions.push(format!("{col}::text ILIKE ${p_idx}"));
                            }
                            FilterOp::Equals => {
                                param_strings.push(trimmed.to_string());
                                let p_idx = param_strings.len();
                                where_conditions.push(format!("{col}::text = ${p_idx}"));
                            }
                            FilterOp::GreaterThan => {
                                param_strings.push(trimmed.to_string());
                                let p_idx = param_strings.len();
                                match f.kind {
                                    FieldType::Int32 | FieldType::Int64 | FieldType::Float => {
                                        where_conditions.push(format!("{col} > ${p_idx}::numeric"));
                                    }
                                    FieldType::DateTime | FieldType::Date => {
                                        where_conditions.push(format!("{col} > ${p_idx}::timestamptz"));
                                    }
                                    _ => {
                                        where_conditions.push(format!("{col} > ${p_idx}"));
                                    }
                                }
                            }
                            FilterOp::LessThan => {
                                param_strings.push(trimmed.to_string());
                                let p_idx = param_strings.len();
                                match f.kind {
                                    FieldType::Int32 | FieldType::Int64 | FieldType::Float => {
                                        where_conditions.push(format!("{col} < ${p_idx}::numeric"));
                                    }
                                    FieldType::DateTime | FieldType::Date => {
                                        where_conditions.push(format!("{col} < ${p_idx}::timestamptz"));
                                    }
                                    _ => {
                                        where_conditions.push(format!("{col} < ${p_idx}"));
                                    }
                                }
                            }
                            FilterOp::GreaterOrEqual => {
                                param_strings.push(trimmed.to_string());
                                let p_idx = param_strings.len();
                                match f.kind {
                                    FieldType::Int32 | FieldType::Int64 | FieldType::Float => {
                                        where_conditions.push(format!("{col} >= ${p_idx}::numeric"));
                                    }
                                    FieldType::DateTime | FieldType::Date => {
                                        where_conditions.push(format!("{col} >= ${p_idx}::timestamptz"));
                                    }
                                    _ => {
                                        where_conditions.push(format!("{col} >= ${p_idx}"));
                                    }
                                }
                            }
                            FilterOp::LessOrEqual => {
                                param_strings.push(trimmed.to_string());
                                let p_idx = param_strings.len();
                                match f.kind {
                                    FieldType::Int32 | FieldType::Int64 | FieldType::Float => {
                                        where_conditions.push(format!("{col} <= ${p_idx}::numeric"));
                                    }
                                    FieldType::DateTime | FieldType::Date => {
                                        where_conditions.push(format!("{col} <= ${p_idx}::timestamptz"));
                                    }
                                    _ => {
                                        where_conditions.push(format!("{col} <= ${p_idx}"));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    let where_sql = if where_conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_conditions.join(" AND "))
    };

    let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = param_strings
        .iter()
        .map(|s| s as &(dyn tokio_postgres::types::ToSql + Sync))
        .collect();

    let table = table_name(model);
    let count_sql = format!("SELECT COUNT(*) FROM {table}{where_sql}");
    let total: i64 = db
        .client()
        .query_one(&count_sql, &params)
        .await
        .map_err(|e| e.to_string())?
        .get(0);

    let columns = fields
        .iter()
        .map(|f| quote(&f.column))
        .collect::<Vec<_>>()
        .join(", ");

    let sort_col = sort
        .and_then(|name| fields.iter().find(|f| f.name == name))
        .map(|f| {
            let dir = if direction == Some("desc") { "DESC" } else { "ASC" };
            format!(" ORDER BY {} {dir}", quote(&f.column))
        })
        .or_else(|| {
            fields
                .iter()
                .find(|f| f.id)
                .map(|f| format!(" ORDER BY {} ASC", quote(&f.column)))
        })
        .unwrap_or_default();

    let offset = (page.saturating_sub(1)) * page_size;
    let sql = format!("SELECT {columns} FROM {table}{where_sql}{sort_col} LIMIT {page_size} OFFSET {offset}");

    let rows = db
        .client()
        .query(&sql, &params)
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        let mut map = Map::new();
        for (index, field) in fields.iter().enumerate() {
            let val = row_field_value(&row, index, &field.kind)?;
            map.insert(field.name.clone(), val);
        }
        result.push(map);
    }

    Ok((result, total))
}

pub async fn insert_row(
    db: &Database,
    model: &Model,
    input_strings: &std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let scalar_fields: Vec<_> = model.fields.iter().filter(|f| scalar(f)).collect();
    let mut cols = Vec::new();
    let mut placeholders = Vec::new();
    let mut params: Vec<Value> = Vec::new();

    for field in &scalar_fields {
        let raw_val = input_strings.get(&field.name).map(|s| s.trim()).unwrap_or("");
        if raw_val.is_empty() {
            // If field is auto_increment or optional or has default, omit it so Postgres supplies default / serial
            if field.auto_increment || field.optional || field.default.is_some() {
                continue;
            }
        }

        // Parse to proper serde_json::Value according to field.kind
        let val = match field.kind {
            FieldType::Int32 | FieldType::Int64 => {
                if raw_val.is_empty() {
                    Value::Null
                } else {
                    let num: i64 = raw_val.parse().map_err(|_| format!("Field '{}' must be a valid integer", field.name))?;
                    Value::Number(num.into())
                }
            }
            FieldType::Float => {
                if raw_val.is_empty() {
                    Value::Null
                } else {
                    let num: f64 = raw_val.parse().map_err(|_| format!("Field '{}' must be a valid float", field.name))?;
                    serde_json::Number::from_f64(num)
                        .map(Value::Number)
                        .unwrap_or(Value::Null)
                }
            }
            FieldType::Boolean => {
                match raw_val.to_lowercase().as_str() {
                    "true" | "1" | "yes" => Value::Bool(true),
                    "false" | "0" | "no" => Value::Bool(false),
                    _ => if field.optional { Value::Null } else { Value::Bool(false) },
                }
            }
            FieldType::Json => {
                if raw_val.is_empty() {
                    Value::Null
                } else {
                    serde_json::from_str(raw_val).map_err(|e| format!("Field '{}' has invalid JSON: {e}", field.name))?
                }
            }
            _ => {
                if raw_val.is_empty() && field.optional {
                    Value::Null
                } else {
                    Value::String(raw_val.to_string())
                }
            }
        };

        if val.is_null() && !field.optional && !field.auto_increment && field.default.is_none() {
            return Err(format!("Field '{}' is required", field.name));
        }

        cols.push(quote(&field.column));
        params.push(val);
        let p_idx = params.len();
        placeholders.push(json_expression(p_idx, field));
    }

    let table = table_name(model);
    let sql = if cols.is_empty() {
        format!("INSERT INTO {table} DEFAULT VALUES")
    } else {
        let cols_sql = cols.join(", ");
        let placeholders_sql = placeholders.join(", ");
        format!("INSERT INTO {table} ({cols_sql}) VALUES ({placeholders_sql})")
    };

    let to_sql_params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
        params.iter().map(|v| v as &(dyn tokio_postgres::types::ToSql + Sync)).collect();

    db.client().execute(&sql, &to_sql_params).await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_row_by_id(
    db: &Database,
    model: &Model,
    id_field_name: &str,
    id_value: &Value,
) -> Result<u64, String> {
    let id_field = model
        .fields
        .iter()
        .find(|f| f.name == id_field_name)
        .ok_or_else(|| format!("Field {id_field_name} not found"))?;

    let table = table_name(model);
    let col = quote(&id_field.column);
    let expr = json_expression(1, id_field);
    let sql = format!("DELETE FROM {table} WHERE {col} IS NOT DISTINCT FROM {expr}");

    let val = id_value.clone();
    let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = vec![&val];

    let changed = db
        .client()
        .execute(&sql, &params)
        .await
        .map_err(|e| e.to_string())?;

    Ok(changed)
}

pub async fn execute_sql(
    db: &Database,
    sql: &str,
) -> Result<(Vec<String>, Vec<Vec<String>>, u64), String> {
    let messages = db
        .client()
        .simple_query(sql)
        .await
        .map_err(|e| e.to_string())?;

    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut affected = 0;

    for msg in messages {
        match msg {
            SimpleQueryMessage::Row(row) => {
                if columns.is_empty() {
                    columns = row.columns().iter().map(|c| c.name().to_string()).collect();
                }
                let mut vals = Vec::new();
                for i in 0..row.columns().len() {
                    vals.push(row.get(i).unwrap_or("NULL").to_string());
                }
                rows.push(vals);
            }
            SimpleQueryMessage::CommandComplete(count) => affected += count,
            _ => {}
        }
    }

    Ok((columns, rows, affected))
}

fn row_field_value(row: &Row, index: usize, kind: &FieldType) -> Result<Value, String> {
    macro_rules! opt {
        ($t:ty) => {
            row.try_get::<_, Option<$t>>(index)
                .map(|v| serde_json::to_value(v).unwrap_or(Value::Null))
                .map_err(|e| e.to_string())
        };
    }

    match kind {
        FieldType::Int32 => opt!(i32),
        FieldType::Int64 => row
            .try_get::<_, Option<i64>>(index)
            .map(|v| v.map(|n| Value::String(n.to_string())).unwrap_or(Value::Null))
            .map_err(|e| e.to_string()),
        FieldType::Float => opt!(f64),
        FieldType::Boolean => opt!(bool),
        FieldType::String => opt!(String),
        FieldType::Json => opt!(Value),
        FieldType::Uuid => opt!(uuid::Uuid),
        FieldType::Date => opt!(chrono::NaiveDate),
        FieldType::Time => opt!(chrono::NaiveTime),
        FieldType::DateTime => opt!(chrono::DateTime<chrono::Utc>),
        FieldType::Bytes => Ok(Value::String("<bytes>".into())),
        FieldType::Relation { .. } => Ok(Value::Null),
    }
}

fn json_expression(index: usize, field: &Field) -> String {
    let value = format!("${index}::jsonb");
    match field.kind {
        FieldType::Json => value,
        _ => format!(
            "CASE WHEN {value} = 'null'::jsonb THEN NULL ELSE ({value} #>> '{{}}')::{} END",
            pg_type(&field.kind)
        ),
    }
}

fn pg_type(kind: &FieldType) -> &'static str {
    match kind {
        FieldType::Int32 => "integer",
        FieldType::Int64 => "bigint",
        FieldType::Float => "double precision",
        FieldType::Boolean => "boolean",
        FieldType::String => "text",
        FieldType::Uuid => "uuid",
        FieldType::Date => "date",
        FieldType::Time => "time",
        FieldType::DateTime => "timestamptz",
        FieldType::Bytes => "bytea",
        FieldType::Json => "jsonb",
        FieldType::Relation { .. } => "text",
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedRelation {
    pub field_name: String,
    pub target_model: String,
    pub target_table: String,
    pub many: bool,
    pub source_field: String,
    pub target_field: String,
    pub target_column: String,
}

pub fn resolve_relations(current_model: &Model, all_models: &[Model]) -> Vec<ResolvedRelation> {
    let mut resolved = Vec::new();
    for f in &current_model.fields {
        if let FieldType::Relation { ref model, many } = f.kind {
            if let Some(target) = all_models.iter().find(|m| &m.name == model) {
                if !many {
                    let source_field = f
                        .relation_fields
                        .first()
                        .cloned()
                        .unwrap_or_else(|| f.name.clone());
                    let target_field = f
                        .relation_references
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "id".to_string());
                    let target_col = target
                        .fields
                        .iter()
                        .find(|tf| tf.name == target_field)
                        .map(|tf| tf.column.clone())
                        .unwrap_or_else(|| target_field.clone());

                    resolved.push(ResolvedRelation {
                        field_name: f.name.clone(),
                        target_model: target.name.clone(),
                        target_table: table_name(target),
                        many: false,
                        source_field,
                        target_field,
                        target_column: target_col,
                    });
                } else {
                    let back_rel = target.fields.iter().find(|tf| {
                        matches!(&tf.kind, FieldType::Relation { model: m, .. } if m == &current_model.name)
                    });

                    if let Some(back_f) = back_rel {
                        let source_field = back_f
                            .relation_references
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "id".to_string());
                        let target_field = back_f
                            .relation_fields
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "id".to_string());
                        let target_col = target
                            .fields
                            .iter()
                            .find(|tf| tf.name == target_field)
                            .map(|tf| tf.column.clone())
                            .unwrap_or_else(|| target_field.clone());

                        resolved.push(ResolvedRelation {
                            field_name: f.name.clone(),
                            target_model: target.name.clone(),
                            target_table: table_name(target),
                            many: true,
                            source_field,
                            target_field,
                            target_column: target_col,
                        });
                    }
                }
            }
        }
    }
    resolved
}

pub async fn query_relation_counts(
    db: &Database,
    relation: &ResolvedRelation,
    source_ids: &[String],
) -> Result<std::collections::HashMap<String, i64>, String> {
    if source_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let target_table = &relation.target_table;
    let target_col = quote(&relation.target_column);
    let sql = format!(
        "SELECT {target_col}::text AS ref_id, COUNT(*)::bigint AS cnt FROM {target_table} WHERE {target_col}::text = ANY($1) GROUP BY {target_col}::text"
    );

    let rows = db
        .client()
        .query(&sql, &[&source_ids])
        .await
        .map_err(|e| e.to_string())?;

    let mut map = std::collections::HashMap::new();
    for row in rows {
        let ref_id: String = row.get(0);
        let cnt: i64 = row.get(1);
        map.insert(ref_id, cnt);
    }

    Ok(map)
}

pub async fn query_connected_records(
    db: &Database,
    target_model: &Model,
    target_column: &str,
    source_value: &str,
) -> Result<Vec<Map<String, Value>>, String> {
    let fields: Vec<_> = target_model.fields.iter().filter(|f| scalar(f)).collect();
    let columns = fields
        .iter()
        .map(|f| quote(&f.column))
        .collect::<Vec<_>>()
        .join(", ");
    let table = table_name(target_model);
    let target_col = quote(target_column);
    let sql = format!(
        "SELECT {columns} FROM {table} WHERE {target_col}::text = $1 LIMIT 50"
    );

    let rows = db
        .client()
        .query(&sql, &[&source_value])
        .await
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        let mut map = Map::new();
        for (index, field) in fields.iter().enumerate() {
            let val = row_field_value(&row, index, &field.kind)?;
            map.insert(field.name.clone(), val);
        }
        result.push(map);
    }

    Ok(result)
}
