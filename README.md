<p align="center">
  <img src="assets/logo.png" width="110" height="110" alt="LiteORM Logo" style="border-radius: 18px;" />
</p>

<h1 align="center">LiteORM</h1>

<p align="center">
  <strong>Schema-first, type-safe PostgreSQL ORM and Native Desktop Studio for Rust.</strong>
</p>

<p align="center">
  <a href="#quickstart">Quickstart</a> •
  <a href="#schema-definition">Schema</a> •
  <a href="#cli-commands">CLI Reference</a> •
  <a href="#querying-with-rust">Querying in Rust</a> •
  <a href="#native-desktop-studio">Desktop Studio</a> •
  <a href="docs/reliability.md">Reliability Guarantees</a>
</p>

---

## Highlights

- **Schema-First Workflow:** Declare your models, relations, and data types once in `liteorm.schema`. Generate strictly typed Rust structs and compile-time safe queries.
- **Transactional Migrations:** Safe schema diffing, atomic migrations, and advisory locking for concurrent deployment safety.
- **Native Desktop Studio:** Fast, 100% native GUI built with Rust & Dioxus Desktop (macOS, Linux, Windows). No browser tabs, no Node.js, and zero JavaScript runtime.
- **Relational Navigation:** Inspect connected records inline via interactive relation pills without losing your place.
- **PostgreSQL-Native:** Full support for parameterized queries, scalar types, platform TLS via rustls, connection pooling, and introspective schema reverse-engineering (`db pull`).
- **Minimalist Aesthetic:** High-contrast duo-tone monochrome interface (`#FFFEFA` and `#000000`).

---

## Quickstart

### 1. Installation

Install the CLI and Native Desktop Studio via Cargo:

```bash
cargo install lite-orm-cli lite-orm-studio
```

And add LiteORM to your backend project:

```bash
cargo add lite-orm
```

### 2. Initialize Project

Initialize a new LiteORM configuration in your repository:

```bash
liteorm init
```

This creates a starter `liteorm.schema` and a `.env` file template.

### 3. Configure Database Connection

Set your PostgreSQL connection string in `.env`:

```dotenv
# .env
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/my_app
```

TLS behavior follows PostgreSQL connection-string options:

| Option | Behavior |
|---|---|
| `sslmode=disable` | Plain PostgreSQL using `NoTls` |
| `sslmode=require` | TLS encryption without certificate or hostname verification |
| `sslmode=verify-ca&sslrootcert=/path/to/server-ca.pem` | Verifies the certificate chain against the supplied private CA |
| `sslmode=verify-full&sslrootcert=/path/to/server-ca.pem` | Verifies both the private CA chain and the connection hostname |
| `sslmode=verify-full` | Verifies the hostname using the platform trust store |

When `sslmode` is omitted, LiteORM preserves its secure default: TLS uses the
platform certificate verifier. For `verify-full`, connect with the hostname
listed on the server certificate; use PostgreSQL's `hostaddr` separately when
the network connection must target a specific IP address.

Paths in URL-style connection strings should be URL-encoded when necessary,
for example `sslrootcert=%2Fsecrets%2Fserver-ca.pem`. Keyword-style connection
strings are also supported, including quoted paths containing spaces.

### 3. Define the Schema

Define models, fields, and relationships in `liteorm.schema`:

```prisma
datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

struct User {
  id        Int32    @id @default(autoincrement())
  email     String   @unique
  name      String
  active    Boolean  @default(true)
  createdAt DateTime @default(now()) @map("created_at")
  posts     Post[]
}

struct Post {
  id        Int32    @id @default(autoincrement())
  title     String
  content   String?
  userId    Int32    @map("user_id")
  user      User     @relation(fields: [userId], references: [id])
}
```

### 4. Push Schema or Create Migrations

Synchronize your schema changes with PostgreSQL:

```bash
# Direct additive sync for rapid development
liteorm db push

# Or generate and run a versioned SQL migration
liteorm migrate dev --name init_tables
```

### 5. Generate Type-Safe Rust Client

```bash
liteorm generate
```

This compiles your schema into type-safe Rust structs and query builders in `src/liteorm.rs` (customizable via `--output <path>`).

### 6. Query from Rust

Add `lite-orm` to your `Cargo.toml` dependencies, then query your database:

```rust
mod liteorm;
use lite_orm::{Database, Model};
use liteorm::{User, Post};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect using DATABASE_URL from .env, including its TLS options
    let db = Database::connect_env().await?;

    // Fetch up to 25 users
    let users = db
        .all(User::select().limit(25))
        .await?;

    for user in users {
        println!("User #{}: {} ({})", user.id, user.name, user.email);
    }

    Ok(())
}
```

### 7. Launch Native Desktop Studio

Manage your database visually with the built-in desktop studio:

```bash
liteorm studio
```

---

## Schema Definition

LiteORM schemas use declarative syntax for models, column mappings, attributes, and foreign relations:

### Field Attributes
- `@id`: Marks a field as primary key.
- `@default(autoincrement())`: Automatic incrementing serial or sequence.
- `@default(now())`: Default to current database timestamp.
- `@default(value)`: Constant defaults (e.g. `@default(true)` or `@default("draft")`).
- `@map("column_name")`: Maps a camelCase field to an existing snake_case database column.
- `@unique`: Enforces unique constraint on the column.
- `?`: Nullable / Optional field (e.g. `String?`, `Int32?`).

### Supported Data Types
- `Int32`, `Int64`, `Float`
- `String`, `Bytes`
- `Boolean`
- `DateTime`, `Date`, `Time`
- `Uuid`
- `Json`
- `Enum` (Custom PostgreSQL native `enum` types)
- `Array` (`String[]`, `Int32[]`, `Enum[]`, etc.)
- Relations (`User`, `Post[]`)

---

## CLI Reference

| Command | Description |
| :--- | :--- |
| `liteorm init` | Scaffolds starter `liteorm.schema` and `.env` template |
| `liteorm generate` | Generates Rust models and query builders into `src/liteorm.rs` |
| `liteorm generate --output <path>` | Generates Rust code to a specified destination file |
| `liteorm schema check` | Validates schema syntax, field types, and relation references |
| `liteorm schema format` | Formats `liteorm.schema` into canonical indentation and alignment |
| `liteorm migrate dev [--name <name>]` | Generates timestamped SQL migration from schema diff and applies it |
| `liteorm migrate deploy` | Applies pending migrations in transactional order (production & CI) |
| `liteorm migrate status` | Displays current migration status and applied history |
| `liteorm db push` | Directly applies additive schema changes without creating migrations |
| `liteorm db pull` | Introspects tables, columns, constraints, enums, and arrays from PostgreSQL |
| `liteorm db seed [--file <file>]` | Runs SQL seed script (defaults to `seed.sql`) |
| `liteorm studio` | Launches the cross-platform Native Desktop Studio application |

---

## Querying in Rust

### Basic Queries
```rust
// Fetch all records with a limit
let users = db.all(User::select().limit(10)).await?;

// Find single record by primary key
let user = db.first(User::select().where_(User::ID.equals(1))).await?;
```

### Filtering & Sorting
```rust
// Predicate filters (equals, contains, starts_with, greater_than, etc.)
let active_posts = db
    .all(
        Post::select()
            .where_(Post::USER_ID.equals(42))
            .order_by_desc(Post::ID)
            .limit(10)
    )
    .await?;
```

### Mutations
```rust
// Safe deletion (guarded against unintended full-table drops)
db.delete(User::delete().where_(User::ID.equals(15))).await?;
```

---

## Native Desktop Studio

LiteORM includes a fast, zero-JS desktop application (`lite-orm-studio`) written directly in Rust using Dioxus Desktop, Tao, and Wry:

- **Cross-Platform Native:** Native Cocoa/AppKit window on macOS, Win32/WebView2 on Windows, and GTK/WebKitGTK on Linux.
- **Live Row Count Badges:** Sidebar navigation shows real-time record counts for every table (e.g. `1,250 rows` or `1 row`) queried via ultra-fast chunked batch queries (~64ms).
- **Responsive Layout:** Elastic sidebar with text truncation, non-wrapping toolbar controls, and horizontally scrollable data grids with sticky headers.
- **Relational Navigation:** Relation pills in the grid show connected record counts (e.g. `Posts [5]`). Clicking any pill opens the connected records directly without leaving your table context.
- **Add Record Modal:** Click `+ Add Record` in the toolbar to validate and insert rows into any table, with automatic identification of auto-incrementing primary keys and type-safe inputs.
- **Column-Level Filters:** Add rules with live operators (`contains`, `equals`, `starts_with`, `ends_with`, `>`, `<`, `>=`, `<=`).
- **Full Text Search & Pagination:** Instantly search text fields across rows with pagination controls.
- **Monochrome & Light/Dark Palette:** Clean, distraction-free aesthetic strictly using `#FFFEFA` canvas and `#000000` ink with instant Light/Dark mode toggling.

---

## Reliability Guarantees

LiteORM enforces strict correctness and safety guarantees backed by automated integration tests:
- **Concurrent Migration Safety:** Schema-scoped PostgreSQL session advisory locking prevents race conditions across deployers.
- **Atomic Operations:** Migrations and migration history metadata execute inside a single transactional block.
- **Mutation Guards:** Update and delete operations strictly reject execution without explicit filtering to prevent accidental data loss.
- **Destructive Change Safety:** Dangerous column type alterations halt with actionable warnings rather than silently coercing or discarding data.

Read more in [`docs/reliability.md`](docs/reliability.md) and [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

---

## License

MIT OR Apache-2.0
