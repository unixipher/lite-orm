# lite-orm

A schema-first PostgreSQL ORM for Rust. Put the connection URL in `.env`,
describe the database in `liteorm.schema`, generate the Rust client, and use it.

## 1. Configure the database

```dotenv
# .env
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/my_app
```

## 2. Define the schema

```prisma
datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

struct Story {
  id    Int32  @id @default(autoincrement())
  title String
}

struct Challenge {
  id        Int32    @id @default(autoincrement())
  title     String
  createdAt DateTime @default(now()) @map("created_at")
  storyId   Int32    @map("story_id")
  audiourl  String
  imageurl  String
  reward    String?
  story     Story    @relation(fields: [storyId], references: [id])
}
```

## 3. Generate and query

```bash
cargo run -p lite-orm-cli --bin liteorm -- check
cargo run -p lite-orm-cli --bin liteorm -- generate
```

```rust
mod liteorm;
use lite_orm::{Database, Model};
use liteorm::Challenge;

# async fn run() -> lite_orm::Result<()> {
let db = Database::connect_env().await?;
let challenges = db
    .all(Challenge::select().limit(20))
    .await?;
# Ok(()) }
```

## Commands

```bash
liteorm init
liteorm generate
liteorm schema check
liteorm schema format

liteorm migrate dev --name create_challenges
liteorm migrate status
liteorm migrate deploy

liteorm db push
liteorm db pull
liteorm db seed
liteorm studio
```

`liteorm generate` writes to `src/liteorm.rs` by default. Use
`liteorm generate --output <path>` when a project needs a different location;
the output path does not need to live in `liteorm.schema`.

`migrate dev` creates `migrations/<timestamp>_<name>/migration.sql` from the
live PostgreSQL difference and applies it. `migrate deploy` applies pending
migration directories in order. `db push` synchronizes additive changes
without creating a migration, while `db pull` introspects tables, scalar
columns, keys, uniqueness, and foreign-key relations back into the schema.

Potentially destructive type changes are rejected instead of being applied
silently. `db seed` executes `seed.sql`, or another file supplied with
`--file`. Studio remains a separate UI binary; `liteorm studio` launches
`liteorm-studio`, configurable through `LITEORM_STUDIO_BIN`.

Current scope includes schema validation and deterministic model generation,
PostgreSQL scalar types, async CRUD, typed filters, safe mutation guards, and
transactional migrations, PostgreSQL introspection, and additive schema
diffing. Next are generated input types, nested relation queries, destructive
migration planning with explicit confirmation, and a Prisma-style client facade.
