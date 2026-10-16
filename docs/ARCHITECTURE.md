# Architecture

lite-orm is PostgreSQL-first and schema-first. The source of truth is
`liteorm.schema`; generated Rust is a rebuildable artifact. Its long-term shape
has five boundaries:

1. **Schema** — parser and validator for `liteorm.schema`.
2. **Contract/codegen** — metadata and generated Rust APIs.
3. **Plan** — typed query and migration ASTs containing no network behavior.
4. **PostgreSQL dialect** — lowers plans to SQL plus separately bound values.
5. **Runtime** — connections, transactions, prepared statements, middleware,
   telemetry, and row decoding.

The current workspace contains the first vertical slice of all five. These
boundaries should become workspace crates only when independent compilation or
third-party extension requires it; package count is not an architecture goal.

## Design commitments

- PostgreSQL behavior is first-class rather than a lowest-common-denominator SQL API.
- Every query remains inspectable through `compile()`.
- Values never enter SQL through string interpolation.
- Models remain normal Rust structs.
- Destructive unscoped mutations are rejected unless explicitly requested.
- Higher-level features compile to plans; they do not bypass the query pipeline.
- The future UI talks to stable introspection and migration APIs, not runtime internals.

## Milestones

1. Query completeness: projections, aggregates, joins, returning, upsert, batch insert.
2. Runtime completeness: pool, transaction closure, prepared query cache, TLS.
3. Relations: declared relation metadata, nested reads, explicit loading strategies.
4. Schema tooling: PostgreSQL introspection, deterministic contract, and additive schema diff. *(initial version complete)*
5. Migration CLI: generate, inspect, and apply are complete; rollback guidance, checksums, and drift detection remain.
6. Extension surface: middleware hooks, custom PostgreSQL types, query policies.
7. Separate UI integration through the contract and migration-plan formats.

The workflow is inspired by Prisma's schema-first approach while the implementation
keeps contracts, plans, PostgreSQL lowering, and runtime separate. No source
implementation is copied; the Rust API and runtime are original.
