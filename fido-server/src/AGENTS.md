# fido-server/src

## Purpose

Production server modules, shared router construction, startup wiring, and application state.

## Ownership

- `lib.rs` exports server modules and builds the API router used by production and tests.
- `main.rs` loads configuration, initializes the database and state, then starts the HTTP process.
- `state.rs` wires repositories, services, sessions, and runtime state.

## Local Contracts

- Keep routes, middleware, security configuration, and request limits consistent between production and test router construction.
- Handlers should translate HTTP input and output. Services should contain domain behavior. Repositories should contain SQL.
- Preserve explicit production checks for test-only routes, token encryption, cookies, and environment configuration.

## Verification

```bash
cargo test -p fido-server --features sqlite-tests
```

## Child DOX Index

- [`api/AGENTS.md`](api/AGENTS.md): request handlers and API errors.
- [`db/AGENTS.md`](db/AGENTS.md): SQLite connection, schema, rows, and repositories.
- [`http/AGENTS.md`](http/AGENTS.md): request metadata and authentication extractors.
- [`security/AGENTS.md`](security/AGENTS.md): security configuration and middleware.
- [`services/AGENTS.md`](services/AGENTS.md): business logic and GitHub integration.
