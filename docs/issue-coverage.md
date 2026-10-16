# Upstream issue coverage

LiteORM uses the Prisma and Drizzle issue trackers as failure-mode inputs, not as
a claim of drop-in compatibility. Both trackers include thousands of issues for
JavaScript runtimes, non-PostgreSQL databases, hosted products, and APIs LiteORM
does not expose. Those are **not applicable**, not “fixed.”

An upstream issue may be marked covered only when all three conditions hold:

1. It applies to PostgreSQL and an API LiteORM exposes.
2. LiteORM has an explicit behavioral guarantee for the failure mode.
3. A regression test exercises that guarantee.

## Current guarantees

| Failure mode | LiteORM behavior | Status |
| --- | --- | --- |
| Concurrent migration replay | A schema-scoped session advisory lock is acquired before migration history is read. Waiting deployers re-read history after obtaining the lock. | Covered |
| Partially applied PostgreSQL migration | Every generated migration and its history insert run in one transaction. | Covered |
| Migration history changes `search_path` | Generated DDL and the history table are schema-qualified; LiteORM does not mutate `search_path`. | Covered |
| Machine timezone changes migration ordering | Migration versions use UTC. | Covered |
| TLS and hosted PostgreSQL poolers | Runtime connections use rustls with platform certificate verification; CLI prefers `DATABASE_URL_UNPOOLED` and falls back to `DATABASE_URL`. | Covered |
| Bound values leaked through generated SQL | Query SQL uses PostgreSQL parameters and keeps values outside the SQL string. | Covered |
| Unscoped update/delete | The typed builder rejects update and delete operations without a filter. | Covered |
| Destructive inferred type changes | Schema diff stops with an error rather than silently changing an existing column type. | Covered |
| Schema-qualified models | Runtime queries, migrations, and introspection preserve PostgreSQL schema names. | Covered |

## Open coverage gaps

These areas must not be advertised as solved yet:

- migration checksums and edited-migration drift detection;
- recovery metadata for failed or manually repaired migrations;
- opt-in non-transactional migrations such as `CREATE INDEX CONCURRENTLY`;
- column/table rename detection and ordered destructive migration plans;
- PostgreSQL enums, arrays, domains, composite types, generated columns,
  expression/partial indexes, check constraints, views, and row-level security;
- exact round-tripping of arbitrary PostgreSQL default expressions;
- nested relation reads/writes, eager loading, aggregates, pagination cursors,
  upsert, batch operations, and transaction APIs;
- a connection pool with acquisition timeouts and retry policy;
- large-schema performance and cross-platform release testing.

## Tracker references

- Drizzle open issues: <https://github.com/drizzle-team/drizzle-orm/issues>
- Prisma open issues: <https://github.com/prisma/orm/issues>
- Prisma concurrent migration replay: <https://github.com/prisma/orm/issues/30658>
- Prisma PostgreSQL migration transaction semantics: <https://github.com/prisma/orm/issues/22922>
- Drizzle migration schema/search-path bug: <https://github.com/drizzle-team/drizzle-orm/issues/5889>

Review this matrix before each release. When an upstream report exposes a
relevant new failure mode, add the regression first and then change its status.
