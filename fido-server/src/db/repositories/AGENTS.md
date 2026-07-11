# fido-server/src/db/repositories

## Purpose

Entity-specific SQLite access and the `Repositories` bundle injected into application services.

## Ownership

Each `*_repository.rs` file owns the SQL for one entity or relation. `mod.rs` exports repositories and constructs `Repositories` from a shared pool.

## Local Contracts

- Bind every SQL value as a parameter.
- Keep row conversion and null handling explicit. Return domain types or repository errors, not HTTP responses.
- When adding a repository, export it and wire it into `Repositories::new` if other layers need it.
- Add repository tests beside the affected code when query behavior, ordering, constraints, or migrations change.

## Verification

```bash
cargo test -p fido-server --features sqlite-tests
```

## Child DOX Index

No child instruction files.
