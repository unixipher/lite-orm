#[path = "../src/liteorm.rs"]
#[allow(dead_code)]
mod models;

use lite_orm::{Database, Model};
use models::User;

#[tokio::main]
async fn main() -> lite_orm::Result<()> {
    let db = Database::connect_env().await?;
    let users = db.all(User::select().limit(20)).await?;
    println!("loaded {} users", users.len());
    Ok(())
}
