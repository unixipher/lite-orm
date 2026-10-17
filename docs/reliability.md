# Reliability

LiteORM is an independent, Rust-first PostgreSQL ORM inspired by the developer
experience of mature ORM ecosystems such as Prisma and Drizzle.

A reliability guarantee is considered complete only when it has explicit
behavior and a regression test.

## Current guarantees

| Area | LiteORM behavior |
| --- | --- |
| Concurrent migrations | A schema-scoped session advisory lock is acquired before migration metadata is created or read. Waiting deployers re-read history after obtaining the lock. |
| Atomic migrations | Every generated migration and its history insert run in one PostgreSQL transaction. |
| Schema isolation | Generated DDL, runtime queries, and migration metadata use schema-qualified identifiers without changing `search_path`. |
| Deterministic ordering | Migration versions use UTC rather than the machine timezone. |
| Hosted PostgreSQL | Connections honor `sslmode` and `sslrootcert`, including private CAs and `NoTls`; the secure default uses rustls platform verification. CLI operations prefer `DATABASE_URL_UNPOOLED` and fall back to `DATABASE_URL`. |
| Parameter safety | Query values use PostgreSQL bound parameters and remain outside generated SQL strings. |
| Mutation safety | Update and delete operations require a filter unless `.all_rows()` is explicitly selected. |
| Schema-change safety | Inferred destructive type changes stop with an error instead of silently changing columns. |
| Nullable text filtering | Nullable and required text columns both support `LIKE` and `ILIKE`. |

## Engineering roadmap

- migration checksums and edited-migration drift detection;
- recovery metadata for failed or manually repaired migrations;
- opt-in non-transactional migrations such as `CREATE INDEX CONCURRENTLY`;
- column and table rename detection with ordered migration plans;
- PostgreSQL enums, arrays, domains, composite types, generated columns,
  expression and partial indexes, check constraints, views, and row-level
  security;
- exact round-tripping of PostgreSQL default expressions;
- nested relation reads and writes, eager loading, aggregates, cursor
  pagination, upsert, batch operations, and transaction APIs;
- connection pooling with acquisition timeouts and retry policies;
- large-schema performance and cross-platform release tests.

New behavior moves from the roadmap into the guarantees table only after its
implementation and regression tests pass.
