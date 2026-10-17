use std::collections::HashSet;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Schema {
    pub datasource: Datasource,
    pub generator: Generator,
    pub enums: Vec<Enum>,
    pub models: Vec<Model>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Datasource {
    pub provider: Option<String>,
    pub url_env: Option<String>,
    pub direct_url_env: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Generator {
    pub output: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enum {
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub name: String,
    pub table: String,
    pub schema: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub column: String,
    pub kind: FieldType,
    pub optional: bool,
    pub id: bool,
    pub unique: bool,
    pub auto_increment: bool,
    pub default: Option<String>,
    pub relation_fields: Vec<String>,
    pub relation_references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    Int32,
    Int64,
    Float,
    Boolean,
    String,
    Bytes,
    Json,
    Uuid,
    Date,
    Time,
    DateTime,
    Enum(String),
    Array(Box<FieldType>),
    Unsupported(String),
    Relation { model: String, many: bool },
}

enum Block {
    Datasource,
    Generator,
    Enum(Enum),
    Model(Model),
}

pub fn parse(source: &str) -> Result<Schema, Vec<String>> {
    let mut schema = Schema::default();
    let mut current = None;
    let mut errors = vec![];

    for (index, raw) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if line == "}" {
            match current.take() {
                Some(Block::Enum(item)) => schema.enums.push(item),
                Some(Block::Model(model)) => schema.models.push(model),
                Some(_) => {}
                None => errors.push(format!("line {line_number}: unexpected `}}`")),
            }
            continue;
        }

        if current.is_none() {
            let words: Vec<_> = line.split_whitespace().collect();
            match words.as_slice() {
                ["datasource", _, "{"] => current = Some(Block::Datasource),
                ["generator", _, "{"] => current = Some(Block::Generator),
                ["enum", name, "{"] => {
                    current = Some(Block::Enum(Enum {
                        name: (*name).into(),
                        variants: vec![],
                    }))
                }
                ["struct" | "model", name, "{"] => {
                    current = Some(Block::Model(Model {
                        name: (*name).into(),
                        table: (*name).into(),
                        schema: "public".into(),
                        fields: vec![],
                    }))
                }
                _ => errors.push(format!(
                    "line {line_number}: expected datasource, generator, enum, or struct block"
                )),
            }
            continue;
        }

        let result = match current.as_mut().expect("checked") {
            Block::Datasource => parse_datasource(line, &mut schema.datasource),
            Block::Generator => parse_generator(line, &mut schema.generator),
            Block::Enum(item) => parse_enum_line(line, item),
            Block::Model(model) => parse_model_line(line, model),
        };
        if let Err(error) = result {
            errors.push(format!("line {line_number}: {error}"));
        }
    }

    if current.is_some() {
        errors.push("schema ends inside an unclosed block".into());
    }
    validate(&mut schema, &mut errors);
    if errors.is_empty() {
        Ok(schema)
    } else {
        Err(errors)
    }
}

fn parse_datasource(line: &str, datasource: &mut Datasource) -> Result<(), String> {
    let (key, value) = assignment(line)?;
    match key {
        "provider" => datasource.provider = Some(quoted(value)?.into()),
        "url" => {
            let inner = value
                .strip_prefix("env(")
                .and_then(|value| value.strip_suffix(')'))
                .ok_or("datasource url must use env(\"VARIABLE\")")?;
            datasource.url_env = Some(quoted(inner)?.into());
        }
        "directUrl" => {
            let inner = value
                .strip_prefix("env(")
                .and_then(|value| value.strip_suffix(')'))
                .ok_or("datasource directUrl must use env(\"VARIABLE\")")?;
            datasource.direct_url_env = Some(quoted(inner)?.into());
        }
        _ => return Err(format!("unknown datasource property `{key}`")),
    }
    Ok(())
}

fn parse_generator(line: &str, generator: &mut Generator) -> Result<(), String> {
    let (key, value) = assignment(line)?;
    match key {
        "output" => generator.output = Some(quoted(value)?.into()),
        _ => return Err(format!("unknown generator property `{key}`")),
    }
    Ok(())
}

fn parse_enum_line(line: &str, item: &mut Enum) -> Result<(), String> {
    let variant = line.trim();
    if !variant.is_empty() {
        item.variants.push(variant.to_owned());
    }
    Ok(())
}

fn parse_model_line(line: &str, model: &mut Model) -> Result<(), String> {
    if line.starts_with("@@") {
        if let Some(value) = attribute_argument(line, "@@map") {
            model.table = value;
            return Ok(());
        }
        if let Some(value) = attribute_argument(line, "@@schema") {
            model.schema = value;
            return Ok(());
        }
        return Err("unknown model attribute".into());
    }
    let mut pieces = line.split_whitespace();
    let name = pieces.next().ok_or("missing field name")?;
    let type_name = pieces.next().ok_or("missing field type")?;
    let attributes = pieces.collect::<Vec<_>>().join(" ");
    let optional = type_name.ends_with('?');
    let base_type = type_name.trim_end_matches('?');
    let many = base_type.ends_with("[]");
    let kind = parse_type(base_type.trim_end_matches("[]"), many)?;
    let column = attribute_argument(&attributes, "@map").unwrap_or_else(|| name.into());
    model.fields.push(Field {
        name: name.into(),
        column,
        kind,
        optional,
        id: has_attribute(&attributes, "@id"),
        unique: has_attribute(&attributes, "@unique"),
        auto_increment: attributes.contains("@default(autoincrement())"),
        default: default_argument(&attributes),
        relation_fields: relation_argument(&attributes, "fields"),
        relation_references: relation_argument(&attributes, "references"),
    });
    Ok(())
}

fn parse_type(value: &str, many: bool) -> Result<FieldType, String> {
    let scalar = match value {
        "Int" | "Int32" => Some(FieldType::Int32),
        "BigInt" | "Int64" => Some(FieldType::Int64),
        "Float" => Some(FieldType::Float),
        "Boolean" => Some(FieldType::Boolean),
        "String" => Some(FieldType::String),
        "Bytes" => Some(FieldType::Bytes),
        "Json" => Some(FieldType::Json),
        "Uuid" => Some(FieldType::Uuid),
        "Date" => Some(FieldType::Date),
        "Time" => Some(FieldType::Time),
        "DateTime" => Some(FieldType::DateTime),
        _ => None,
    };
    if let Some(inner) = scalar {
        return if many {
            Ok(FieldType::Array(Box::new(inner)))
        } else {
            Ok(inner)
        };
    }
    if value.starts_with("Unsupported(\"") && value.ends_with("\")") {
        let inner = &value["Unsupported(\"".len()..value.len() - 2];
        let unsupported = FieldType::Unsupported(inner.to_string());
        return if many {
            Ok(FieldType::Array(Box::new(unsupported)))
        } else {
            Ok(unsupported)
        };
    }
    if value
        .chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_')
    {
        Ok(FieldType::Relation {
            model: value.into(),
            many,
        })
    } else {
        Err(format!("unsupported field type `{value}`"))
    }
}

fn validate(schema: &mut Schema, errors: &mut Vec<String>) {
    if schema.datasource.provider.as_deref() != Some("postgresql") {
        errors.push("datasource provider must be \"postgresql\"".into());
    }
    if schema.datasource.url_env.as_deref() != Some("DATABASE_URL") {
        errors.push("datasource url must be env(\"DATABASE_URL\")".into());
    }
    if schema.models.is_empty() {
        errors.push("schema must define at least one model".into());
    }
    let mut enums_seen = HashSet::new();
    for item in &schema.enums {
        if !enums_seen.insert(&item.name) {
            errors.push(format!("duplicate enum `{}`", item.name));
        }
    }
    let enum_names: HashSet<String> = schema.enums.iter().map(|e| e.name.clone()).collect();
    for model in &mut schema.models {
        for field in &mut model.fields {
            if let FieldType::Relation { model: target, many } = &field.kind {
                if enum_names.contains(target) {
                    if *many {
                        field.kind = FieldType::Array(Box::new(FieldType::Enum(target.clone())));
                    } else {
                        field.kind = FieldType::Enum(target.clone());
                    }
                }
            }
        }
    }
    let mut models = HashSet::new();
    for model in &schema.models {
        if !models.insert(&model.name) {
            errors.push(format!("duplicate model `{}`", model.name));
        }
        let mut fields = HashSet::new();
        for field in &model.fields {
            if !fields.insert(&field.name) {
                errors.push(format!("duplicate field `{}.{}`", model.name, field.name));
            }
            if let FieldType::Relation { model: target, .. } = &field.kind {
                if !schema.models.iter().any(|item| item.name == *target) {
                    errors.push(format!(
                        "relation `{}.{}` targets unknown struct `{target}`",
                        model.name, field.name
                    ));
                }
                if field.relation_fields.len() != field.relation_references.len() {
                    errors.push(format!(
                        "relation `{}.{}` must have the same number of fields and references",
                        model.name, field.name
                    ));
                }
                for local in &field.relation_fields {
                    if !model
                        .fields
                        .iter()
                        .any(|candidate| candidate.name == *local)
                    {
                        errors.push(format!(
                            "relation `{}.{}` uses unknown field `{local}`",
                            model.name, field.name
                        ));
                    }
                }
            }
        }
    }
}

fn assignment(line: &str) -> Result<(&str, &str), String> {
    let (key, value) = line.split_once('=').ok_or("expected `name = value`")?;
    Ok((key.trim(), value.trim()))
}
fn quoted(value: &str) -> Result<&str, String> {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .ok_or("expected a quoted string".into())
}
fn attribute_argument(source: &str, name: &str) -> Option<String> {
    let start = source.find(&format!("{name}(\""))? + name.len() + 2;
    let rest = &source[start..];
    Some(rest[..rest.find("\")")?].into())
}
fn has_attribute(source: &str, name: &str) -> bool {
    source.split_whitespace().any(|part| part == name)
}

fn relation_argument(source: &str, name: &str) -> Vec<String> {
    let Some(relation) = source.split("@relation(").nth(1) else {
        return vec![];
    };
    let Some(start) = relation.find(&format!("{name}:")) else {
        return vec![];
    };
    let rest = &relation[start + name.len() + 1..];
    let Some(open) = rest.find('[') else {
        return vec![];
    };
    let Some(close) = rest[open + 1..].find(']') else {
        return vec![];
    };
    rest[open + 1..open + 1 + close]
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect()
}

fn default_argument(source: &str) -> Option<String> {
    let start = source.find("@default(")? + "@default(".len();
    let mut depth = 1_u32;
    let mut quoted = false;
    for (offset, character) in source[start..].char_indices() {
        match character {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => {
                depth -= 1;
                if depth == 0 {
                    return Some(source[start..start + offset].trim().to_owned());
                }
            }
            _ => {}
        }
    }
    None
}

pub fn format(schema: &Schema) -> String {
    let mut output = String::from(
        "datasource db {\n  provider = \"postgresql\"\n  url      = env(\"DATABASE_URL\")\n",
    );
    if let Some(variable) = &schema.datasource.direct_url_env {
        output.push_str(&format!("  directUrl = env({variable:?})\n"));
    }
    output.push_str("}\n");
    if let Some(path) = &schema.generator.output {
        output.push_str(&format!("\ngenerator client {{\n  output = {path:?}\n}}\n"));
    }
    for item in &schema.enums {
        output.push_str(&format!("\nenum {} {{\n", item.name));
        for variant in &item.variants {
            output.push_str(&format!("  {variant}\n"));
        }
        output.push_str("}\n");
    }
    for model in &schema.models {
        output.push_str(&format!("\nstruct {} {{\n", model.name));
        let width = model
            .fields
            .iter()
            .map(|field| field.name.len())
            .max()
            .unwrap_or(0);
        for field in &model.fields {
            let type_name = display_type(field);
            let mut attributes = vec![];
            if field.id {
                attributes.push("@id".to_owned());
            }
            if field.unique {
                attributes.push("@unique".to_owned());
            }
            if field.auto_increment {
                attributes.push("@default(autoincrement())".to_owned());
            } else if let Some(default) = &field.default {
                attributes.push(format!("@default({default})"));
            }
            if field.column != field.name {
                attributes.push(format!("@map({:?})", field.column));
            }
            if let FieldType::Relation { .. } = field.kind
                && !field.relation_fields.is_empty()
            {
                attributes.push(format!(
                    "@relation(fields: [{}], references: [{}])",
                    field.relation_fields.join(", "),
                    field.relation_references.join(", ")
                ));
            }
            let line = format!(
                "  {:width$} {:<10} {}",
                field.name,
                type_name,
                attributes.join(" ")
            );
            output.push_str(line.trim_end());
            output.push('\n');
        }
        if model.table != model.name {
            output.push_str(&format!("\n  @@map({:?})\n", model.table));
        }
        if model.schema != "public" {
            output.push_str(&format!("  @@schema({:?})\n", model.schema));
        }
        output.push_str("}\n");
    }
    output
}

fn display_type(field: &Field) -> String {
    let value = match &field.kind {
        FieldType::Int32 => "Int32".into(),
        FieldType::Int64 => "Int64".into(),
        FieldType::Float => "Float".into(),
        FieldType::Boolean => "Boolean".into(),
        FieldType::String => "String".into(),
        FieldType::Bytes => "Bytes".into(),
        FieldType::Json => "Json".into(),
        FieldType::Uuid => "Uuid".into(),
        FieldType::Date => "Date".into(),
        FieldType::Time => "Time".into(),
        FieldType::DateTime => "DateTime".into(),
        FieldType::Enum(name) => name.clone(),
        FieldType::Array(inner) => format!("{}[]", display_inner(inner)),
        FieldType::Unsupported(name) => format!("Unsupported({name:?})"),
        FieldType::Relation { model, many } => format!("{model}{}", if *many { "[]" } else { "" }),
    };
    format!("{value}{}", if field.optional { "?" } else { "" })
}

fn display_inner(kind: &FieldType) -> String {
    match kind {
        FieldType::Int32 => "Int32".into(),
        FieldType::Int64 => "Int64".into(),
        FieldType::Float => "Float".into(),
        FieldType::Boolean => "Boolean".into(),
        FieldType::String => "String".into(),
        FieldType::Bytes => "Bytes".into(),
        FieldType::Json => "Json".into(),
        FieldType::Uuid => "Uuid".into(),
        FieldType::Date => "Date".into(),
        FieldType::Time => "Time".into(),
        FieldType::DateTime => "DateTime".into(),
        FieldType::Enum(name) => name.clone(),
        FieldType::Array(inner) => format!("{}[]", display_inner(inner)),
        FieldType::Unsupported(name) => format!("Unsupported({name:?})"),
        FieldType::Relation { model, .. } => model.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_starter_schema() {
        let schema = parse(include_str!("../starter.schema")).unwrap();
        assert_eq!(schema.models.len(), 1);
        assert_eq!(schema.models[0].table, "users");
        assert_eq!(schema.models[0].fields[4].column, "created_at");
        assert_eq!(
            schema.models[0].fields[0].default.as_deref(),
            Some("autoincrement()")
        );
        assert_eq!(schema.models[0].fields[4].default.as_deref(), Some("now()"));
    }

    #[test]
    fn parses_struct_relations_and_explicit_integer_widths() {
        let schema = parse(
            r#"
            datasource db {
              provider = "postgresql"
              url = env("DATABASE_URL")
            }
            struct User {
              id Int32 @id @default(autoincrement())
            }
            struct Post {
              id Int @id @default(autoincrement())
              userId Int32
              user User @relation(fields: [userId], references: [id])
            }
            "#,
        )
        .unwrap();
        let relation = &schema.models[1].fields[2];
        assert!(matches!(relation.kind, FieldType::Relation { .. }));
        assert_eq!(relation.relation_fields, ["userId"]);
        assert_eq!(relation.relation_references, ["id"]);
    }

    #[test]
    fn formatting_is_idempotent() {
        let parsed = parse(include_str!("../../liteorm.schema")).unwrap();
        let once = format(&parsed);
        let twice = format(&parse(&once).unwrap());
        assert_eq!(once, twice);
    }

    #[test]
    fn parses_enums_and_arrays() {
        let schema = parse(
            r#"
            datasource db {
              provider = "postgresql"
              url = env("DATABASE_URL")
            }
            enum Role {
              admin
              member
            }
            struct User {
              id Int32 @id
              role Role
              tags String[]
            }
            "#,
        )
        .unwrap();
        assert_eq!(schema.enums.len(), 1);
        assert_eq!(schema.enums[0].name, "Role");
        assert_eq!(schema.enums[0].variants, vec!["admin", "member"]);
        assert_eq!(schema.models[0].fields[1].kind, FieldType::Enum("Role".into()));
        assert_eq!(
            schema.models[0].fields[2].kind,
            FieldType::Array(Box::new(FieldType::String))
        );
        let formatted = format(&schema);
        assert!(formatted.contains("enum Role"));
        let parsed_again = parse(&formatted).unwrap();
        assert_eq!(schema, parsed_again);
    }
}
