mod liteorm;

use chrono::Utc;
use lite_orm::{Database, Model};
use liteorm::{Post, User};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to PostgreSQL via LiteORM...");
    let db = Database::connect_env().await?;
    println!("[OK] Connected successfully!");

    // Clean up existing demo data
    println!("\nCleaning previous demo data...");
    let deleted_posts = db.delete(Post::delete().all_rows()).await?;
    let deleted_users = db.delete(User::delete().all_rows()).await?;
    println!("[OK] Cleared {deleted_users} users and {deleted_posts} posts");

    // 1. Create Users
    println!("\n1. Inserting Users...");
    let alice = db
        .create(&User {
            id: 0,
            email: "alice@example.com".into(),
            name: Some("Alice Developer".into()),
            created_at: Utc::now(),
        })
        .await?;
    println!(
        "[OK] Created User: #{} - {} ({})",
        alice.id,
        alice.name.as_deref().unwrap_or(""),
        alice.email
    );

    let bob = db
        .create(&User {
            id: 0,
            email: "bob@rustlang.org".into(),
            name: Some("Bob Hacker".into()),
            created_at: Utc::now(),
        })
        .await?;
    println!(
        "[OK] Created User: #{} - {} ({})",
        bob.id,
        bob.name.as_deref().unwrap_or(""),
        bob.email
    );

    // 2. Create Posts
    println!("\n2. Inserting Posts...");
    let post1 = db
        .create(&Post {
            id: 0,
            title: "Introducing LiteORM for Rust".into(),
            content: "A schema-first, lightweight async ORM with Studio UI.".into(),
            published: true,
            author_id: alice.id,
        })
        .await?;
    println!(
        "[OK] Created Post: #{} - \"{}\" (by user #{})",
        post1.id, post1.title, post1.author_id
    );

    let post2 = db
        .create(&Post {
            id: 0,
            title: "Building High-Performance Services".into(),
            content: "Exploring asynchronous I/O and typed queries.".into(),
            published: false,
            author_id: bob.id,
        })
        .await?;
    println!(
        "[OK] Created Post: #{} - \"{}\" (by user #{})",
        post2.id, post2.title, post2.author_id
    );

    // 3. Query records with filters
    println!("\n3. Querying published posts...");
    let published_posts = db
        .all(Post::select().filter(Post::published.eq(true)))
        .await?;
    for p in &published_posts {
        println!("  - [{}] {} (Published: {})", p.id, p.title, p.published);
    }

    // 4. Update record
    println!("\n4. Publishing Bob's draft post...");
    let updated = db
        .update(
            Post::update()
                .set(Post::published, true)
                .filter(Post::id.eq(post2.id)),
        )
        .await?;
    println!("[OK] Updated {updated} post(s)");

    // 5. Query after update
    println!("\n5. Fetching all posts ordered by title...");
    let all_posts = db.all(Post::select().order_by(Post::title.asc())).await?;
    for p in &all_posts {
        println!(
            "  - [{}] \"{}\" | Published: {} | Author: #{}",
            p.id, p.title, p.published, p.author_id
        );
    }

    println!("\n[SUCCESS] LiteORM demo executed successfully!");
    Ok(())
}
