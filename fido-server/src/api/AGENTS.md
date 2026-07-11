# fido-server/src/api

## Purpose

Axum handlers for authentication, communities, chat, posts, DMs, notifications, profiles, and WebSocket connections.

## Ownership

- Each endpoint family has one module.
- `error.rs` owns the API error response shape.
- Route registration stays in `../lib.rs`.

## Local Contracts

- Authenticate and authorize before invoking stateful behavior.
- Validate all client input here or in the service boundary. Return `ApiError` rather than ad hoc response formats.
- Keep handlers thin. Call services and repositories through `AppState`; do not duplicate SQL or GitHub calls in handlers.
- Route changes require a matching update in `../lib.rs` and endpoint coverage.

## Verification

```bash
cargo test -p fido-server --features sqlite-tests
```

## Child DOX Index

No child instruction files.
