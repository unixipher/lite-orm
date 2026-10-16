use std::marker::PhantomData;

use crate::{Error, IntoValue, Model, Result, Value};

pub struct Column<M, T> {
    pub(crate) name: &'static str,
    _marker: PhantomData<fn() -> (M, T)>,
}
impl<M, T> Copy for Column<M, T> {}
impl<M, T> Clone for Column<M, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M, T> Column<M, T> {
    #[doc(hidden)]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _marker: PhantomData,
        }
    }
    pub fn eq(self, value: impl IntoValue) -> Expr<M> {
        Expr::comparison(self.name, "=", value)
    }
    pub fn ne(self, value: impl IntoValue) -> Expr<M> {
        Expr::comparison(self.name, "<>", value)
    }
    pub fn gt(self, value: impl IntoValue) -> Expr<M> {
        Expr::comparison(self.name, ">", value)
    }
    pub fn gte(self, value: impl IntoValue) -> Expr<M> {
        Expr::comparison(self.name, ">=", value)
    }
    pub fn lt(self, value: impl IntoValue) -> Expr<M> {
        Expr::comparison(self.name, "<", value)
    }
    pub fn lte(self, value: impl IntoValue) -> Expr<M> {
        Expr::comparison(self.name, "<=", value)
    }
    pub fn between(self, start: impl IntoValue, end: impl IntoValue) -> Expr<M> {
        Expr::Between {
            column: self.name,
            start: start.into_value(),
            end: end.into_value(),
            _model: PhantomData,
        }
    }
    pub fn in_list<I, V>(self, values: I) -> Expr<M>
    where
        I: IntoIterator<Item = V>,
        V: IntoValue,
    {
        Expr::In {
            column: self.name,
            values: values.into_iter().map(IntoValue::into_value).collect(),
            negated: false,
            _model: PhantomData,
        }
    }
    pub fn not_in<I, V>(self, values: I) -> Expr<M>
    where
        I: IntoIterator<Item = V>,
        V: IntoValue,
    {
        Expr::In {
            column: self.name,
            values: values.into_iter().map(IntoValue::into_value).collect(),
            negated: true,
            _model: PhantomData,
        }
    }
    pub fn is_null(self) -> Expr<M> {
        Expr::Null {
            column: self.name,
            negated: false,
            _model: PhantomData,
        }
    }
    pub fn is_not_null(self) -> Expr<M> {
        Expr::Null {
            column: self.name,
            negated: true,
            _model: PhantomData,
        }
    }
    pub fn asc(self) -> Order<M> {
        Order {
            column: self.name,
            descending: false,
            _model: PhantomData,
        }
    }
    pub fn desc(self) -> Order<M> {
        Order {
            column: self.name,
            descending: true,
            _model: PhantomData,
        }
    }
}

impl<M> Column<M, String> {
    pub fn like(self, pattern: impl IntoValue) -> Expr<M> {
        Expr::comparison(self.name, "LIKE", pattern)
    }
    pub fn ilike(self, pattern: impl IntoValue) -> Expr<M> {
        Expr::comparison(self.name, "ILIKE", pattern)
    }
}

pub enum Expr<M> {
    Comparison {
        column: &'static str,
        operator: &'static str,
        value: Value,
        _model: PhantomData<M>,
    },
    Null {
        column: &'static str,
        negated: bool,
        _model: PhantomData<M>,
    },
    Between {
        column: &'static str,
        start: Value,
        end: Value,
        _model: PhantomData<M>,
    },
    In {
        column: &'static str,
        values: Vec<Value>,
        negated: bool,
        _model: PhantomData<M>,
    },
    Not(Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
}

impl<M> Expr<M> {
    fn comparison(column: &'static str, operator: &'static str, value: impl IntoValue) -> Self {
        Self::Comparison {
            column,
            operator,
            value: value.into_value(),
            _model: PhantomData,
        }
    }
    pub fn and(self, other: Self) -> Self {
        Self::And(Box::new(self), Box::new(other))
    }
    pub fn or(self, other: Self) -> Self {
        Self::Or(Box::new(self), Box::new(other))
    }
    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Self {
        Self::Not(Box::new(self))
    }

    fn write_sql(&self, sql: &mut String, values: &mut Vec<Value>) {
        match self {
            Self::Comparison {
                column,
                operator,
                value,
                ..
            } => {
                quote_identifier(sql, column);
                sql.push(' ');
                sql.push_str(operator);
                push_value(sql, values, value.clone());
            }
            Self::Null {
                column, negated, ..
            } => {
                quote_identifier(sql, column);
                sql.push_str(if *negated { " IS NOT NULL" } else { " IS NULL" });
            }
            Self::Between {
                column, start, end, ..
            } => {
                quote_identifier(sql, column);
                sql.push_str(" BETWEEN");
                push_value(sql, values, start.clone());
                sql.push_str(" AND");
                push_value(sql, values, end.clone());
            }
            Self::In {
                column,
                values: list,
                negated,
                ..
            } => {
                if list.is_empty() {
                    sql.push_str(if *negated { "TRUE" } else { "FALSE" });
                    return;
                }
                quote_identifier(sql, column);
                sql.push_str(if *negated { " NOT IN (" } else { " IN (" });
                for (index, value) in list.iter().enumerate() {
                    if index > 0 {
                        sql.push_str(", ");
                    }
                    values.push(value.clone());
                    sql.push('$');
                    sql.push_str(&values.len().to_string());
                }
                sql.push(')');
            }
            Self::Not(inner) => {
                sql.push_str("NOT (");
                inner.write_sql(sql, values);
                sql.push(')');
            }
            Self::And(left, right) | Self::Or(left, right) => {
                sql.push('(');
                left.write_sql(sql, values);
                sql.push_str(if matches!(self, Self::And(_, _)) {
                    " AND "
                } else {
                    " OR "
                });
                right.write_sql(sql, values);
                sql.push(')');
            }
        }
    }
}

pub struct Order<M> {
    column: &'static str,
    descending: bool,
    _model: PhantomData<M>,
}
pub struct Select<M> {
    filter: Option<Expr<M>>,
    orders: Vec<Order<M>>,
    limit: Option<u64>,
    offset: Option<u64>,
}

impl<M: Model> Select<M> {
    pub(crate) fn new() -> Self {
        Self {
            filter: None,
            orders: vec![],
            limit: None,
            offset: None,
        }
    }
    pub fn filter(mut self, expression: Expr<M>) -> Self {
        self.filter = merge_filter(self.filter, expression);
        self
    }
    pub fn order_by(mut self, order: Order<M>) -> Self {
        self.orders.push(order);
        self
    }
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }
    pub fn compile(&self) -> CompiledQuery {
        let mut sql = String::from("SELECT ");
        write_columns::<M>(&mut sql);
        sql.push_str(" FROM ");
        quote_table::<M>(&mut sql);
        let mut values = vec![];
        write_filter(&mut sql, &mut values, self.filter.as_ref());
        if !self.orders.is_empty() {
            sql.push_str(" ORDER BY ");
            for (index, order) in self.orders.iter().enumerate() {
                if index > 0 {
                    sql.push_str(", ");
                }
                quote_identifier(&mut sql, order.column);
                sql.push_str(if order.descending { " DESC" } else { " ASC" });
            }
        }
        if let Some(limit) = self.limit {
            sql.push_str(" LIMIT ");
            sql.push_str(&limit.to_string());
        }
        if let Some(offset) = self.offset {
            sql.push_str(" OFFSET ");
            sql.push_str(&offset.to_string());
        }
        CompiledQuery { sql, values }
    }
}

pub struct Update<M> {
    sets: Vec<(&'static str, Value)>,
    filter: Option<Expr<M>>,
    allow_all: bool,
}
impl<M: Model> Update<M> {
    pub(crate) fn new() -> Self {
        Self {
            sets: vec![],
            filter: None,
            allow_all: false,
        }
    }
    pub fn set<T>(mut self, column: Column<M, T>, value: impl IntoValue) -> Self {
        self.sets.push((column.name, value.into_value()));
        self
    }
    pub fn filter(mut self, expression: Expr<M>) -> Self {
        self.filter = merge_filter(self.filter, expression);
        self
    }
    pub fn all_rows(mut self) -> Self {
        self.allow_all = true;
        self
    }
    pub fn compile(&self) -> Result<CompiledQuery> {
        if self.sets.is_empty() {
            return Err(Error::InvalidQuery(
                "UPDATE requires at least one field".into(),
            ));
        }
        require_filter(self.filter.is_some(), self.allow_all, "UPDATE")?;
        let mut sql = String::from("UPDATE ");
        quote_table::<M>(&mut sql);
        sql.push_str(" SET ");
        let mut values = vec![];
        for (index, (column, value)) in self.sets.iter().enumerate() {
            if index > 0 {
                sql.push_str(", ");
            }
            quote_identifier(&mut sql, column);
            sql.push_str(" =");
            push_value(&mut sql, &mut values, value.clone());
        }
        write_filter(&mut sql, &mut values, self.filter.as_ref());
        Ok(CompiledQuery { sql, values })
    }
}

pub struct Delete<M> {
    filter: Option<Expr<M>>,
    allow_all: bool,
}
impl<M: Model> Delete<M> {
    pub(crate) fn new() -> Self {
        Self {
            filter: None,
            allow_all: false,
        }
    }
    pub fn filter(mut self, expression: Expr<M>) -> Self {
        self.filter = merge_filter(self.filter, expression);
        self
    }
    pub fn all_rows(mut self) -> Self {
        self.allow_all = true;
        self
    }
    pub fn compile(&self) -> Result<CompiledQuery> {
        require_filter(self.filter.is_some(), self.allow_all, "DELETE")?;
        let mut sql = String::from("DELETE FROM ");
        quote_table::<M>(&mut sql);
        let mut values = vec![];
        write_filter(&mut sql, &mut values, self.filter.as_ref());
        Ok(CompiledQuery { sql, values })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledQuery {
    pub sql: String,
    pub values: Vec<Value>,
}

fn merge_filter<M>(filter: Option<Expr<M>>, expression: Expr<M>) -> Option<Expr<M>> {
    Some(match filter {
        Some(existing) => existing.and(expression),
        None => expression,
    })
}
fn require_filter(has_filter: bool, allow_all: bool, operation: &str) -> Result<()> {
    if has_filter || allow_all {
        Ok(())
    } else {
        Err(Error::InvalidQuery(format!(
            "{operation} without a filter is blocked; call .all_rows() explicitly"
        )))
    }
}
fn write_filter<M>(sql: &mut String, values: &mut Vec<Value>, filter: Option<&Expr<M>>) {
    if let Some(filter) = filter {
        sql.push_str(" WHERE ");
        filter.write_sql(sql, values);
    }
}
fn push_value(sql: &mut String, values: &mut Vec<Value>, value: Value) {
    values.push(value);
    sql.push_str(" $");
    sql.push_str(&values.len().to_string());
}
fn write_columns<M: Model>(sql: &mut String) {
    for (index, column) in M::COLUMNS.iter().enumerate() {
        if index > 0 {
            sql.push_str(", ");
        }
        quote_identifier(sql, column);
    }
}
pub(crate) fn quote_table<M: Model>(sql: &mut String) {
    quote_identifier(sql, M::SCHEMA);
    sql.push('.');
    quote_identifier(sql, M::TABLE);
}
pub(crate) fn quote_identifier(sql: &mut String, identifier: &str) {
    sql.push('"');
    for character in identifier.chars() {
        if character == '"' {
            sql.push('"');
        }
        sql.push(character);
    }
    sql.push('"');
}
