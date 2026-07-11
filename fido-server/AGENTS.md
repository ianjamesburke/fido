# fido-server

## Purpose

This crate runs Fido's Axum API, SQLite persistence, GitHub integration, and web-terminal server process.

## Ownership

- `src/` owns production server code and the shared router.
- `tests/` owns black-box server coverage, including the GitHub fixture-backed community test.
- `settings.toml` defines local default settings. Environment variables remain the deployment boundary.

## Local Contracts

- Keep `create_router_with_security_config` in `src/lib.rs` usable by tests. Mount production-only static fallback routes in `src/main.rs`.
- Server handlers use `AppState`, services own business logic, and repositories own SQL.
- Do not mount passwordless test authentication in production.
- Treat `FIDO_TOKEN_KEY`, cookie settings, CORS, and environment parsing as security-sensitive. Production must fail closed.
- SQLite remains a single-writer store. Preserve transaction boundaries and existing indexes when changing persistence.

## Work Guidance

- Add or change an endpoint through its handler, service, repository, shared types, router registration, and relevant tests.
- Use `just server` or `just server-reset` when reproducing local startup behavior. Read `logs/fido-server.log` before guessing.

## Verification

```bash
cargo test -p fido-server --features sqlite-tests
cargo test --test e2e_community_rewrite -p fido-server
```

Run the focused integration test for routes, auth, GitHub, or database changes. Follow with the workspace checks required by the root guide.

## Child DOX Index

- [`src/AGENTS.md`](src/AGENTS.md): server module boundaries.
