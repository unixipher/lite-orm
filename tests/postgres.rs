use lite_orm::{Error, Model, Value};

#[derive(Debug, PartialEq, Model)]
#[lite_orm(schema = "app", table = "users")]
struct User {
    #[lite_orm(primary_key, auto_increment)]
    id: i64,
    name: String,
    email: Option<String>,
    active: bool,
}

#[test]
fn compiles_postgres_selects() {
    let query = User::select()
        .filter(User::active.eq(true))
        .filter(User::name.ilike("m%"))
        .order_by(User::name.asc())
        .limit(10)
        .offset(20)
        .compile();

    assert_eq!(
        query.sql,
        "SELECT \"id\", \"name\", \"email\", \"active\" FROM \"app\".\"users\" WHERE (\"active\" = $1 AND \"name\" ILIKE $2) ORDER BY \"name\" ASC LIMIT 10 OFFSET 20"
    );
    assert_eq!(
        query.values,
        vec![Value::Bool(true), Value::Text("m%".into())]
    );
}

#[test]
fn compiles_richer_predicates() {
    let query = User::select()
        .filter(
            User::id
                .in_list([1_i64, 2, 3])
                .or(User::email.ilike("%@example.com")),
        )
        .compile();
    assert_eq!(
        query.sql,
        "SELECT \"id\", \"name\", \"email\", \"active\" FROM \"app\".\"users\" WHERE (\"id\" IN ($1, $2, $3) OR \"email\" ILIKE $4)"
    );
}

#[test]
fn compiles_updates_and_deletes() {
    let update = User::update()
        .set(User::active, false)
        .set(User::name, "Archived")
        .filter(User::id.eq(7_i64))
        .compile()
        .unwrap();
    assert_eq!(
        update.sql,
        "UPDATE \"app\".\"users\" SET \"active\" = $1, \"name\" = $2 WHERE \"id\" = $3"
    );

    let delete = User::delete().filter(User::id.eq(7_i64)).compile().unwrap();
    assert_eq!(
        delete.sql,
        "DELETE FROM \"app\".\"users\" WHERE \"id\" = $1"
    );
}

#[test]
fn blocks_unscoped_mutations() {
    assert!(matches!(
        User::delete().compile(),
        Err(Error::InvalidQuery(_))
    ));
    assert!(matches!(
        User::update().set(User::active, false).compile(),
        Err(Error::InvalidQuery(_))
    ));
    assert!(User::delete().all_rows().compile().is_ok());
}

#[derive(Debug, PartialEq, Model)]
struct AuditLog {
    #[lite_orm(column = "event_id", auto_increment)]
    id: i64,
    #[lite_orm(column = "event_name")]
    event: String,
}

#[test]
fn supports_renamed_columns() {
    let query = AuditLog::select()
        .filter(AuditLog::event.eq("signed_in"))
        .compile();
    assert_eq!(
        query.sql,
        "SELECT \"event_id\", \"event_name\" FROM \"public\".\"audit_log\" WHERE \"event_name\" = $1"
    );
}
