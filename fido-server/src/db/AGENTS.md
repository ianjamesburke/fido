# fido-server/src/db

## Purpose

SQLite connection pooling, schema setup, row conversion, and repository exports.

## Ownership

- `connection.rs` owns `Database` and the pool.
- `schema.rs` owns schema creation, migrations, seed data, and indexes.
- `row.rs` owns database-to-domain conversions.
- `repositories/` owns entity-specific SQL.

## Local Contracts

- Use parameterized SQL only.
- Keep schema changes compatible with existing persisted databases and add indexes for new query paths.
- Use transactions when a domain operation changes multiple rows.
- Repositories are cheap `DbPool` wrappers. Add them to `Repositories` when a new domain service needs shared access.

## Verification

```bash
cargo test -p fido-server --features sqlite-tests
```

## Child DOX Index

- [`repositories/AGENTS.md`](repositories/AGENTS.md): entity repositories and the shared repository bundle.
