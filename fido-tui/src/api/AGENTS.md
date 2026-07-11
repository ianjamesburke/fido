# fido-tui/src/api

## Purpose

Typed HTTP client, API errors, and WebSocket realtime client for the Fido server.

## Ownership

- `client.rs` owns request and response types plus HTTP calls.
- `realtime.rs` owns WebSocket connection state and event delivery.
- `error.rs` owns client-facing API error types.

## Local Contracts

- Keep server paths, request payloads, and response types aligned with `fido-server` handlers and `fido-types` models.
- Surface server failures as `ApiError`; do not silently discard failed mutations.
- Realtime work must communicate through its event channel. Do not mutate `App` directly from background tasks.
- Apply timeouts to network work and keep it off the render path.

## Verification

```bash
cargo test -p fido
just e2e-tui
```

## Child DOX Index

No child instruction files.
