#[path = "../src/liteorm.rs"]
#[allow(dead_code)]
mod models;

use lite_orm::{Database, Model};
use models::Challenge;

#[tokio::main]
async fn main() -> lite_orm::Result<()> {
    let db = Database::connect_env().await?;
    let challenges = db.all(Challenge::select().limit(20)).await?;
    println!("loaded {} challenges", challenges.len());
    Ok(())
}
