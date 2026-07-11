# fido-types/src

## Purpose

Public shared domain types for HTTP payloads, persisted data projections, and realtime events.

## Ownership

- `models.rs` defines core entities and request or response payloads.
- `events.rs` defines shared realtime messages.
- `enums.rs` defines reusable enum values.
- `lib.rs` re-exports the supported public surface.

## Local Contracts

- Derive and maintain the serialization traits required by both clients.
- Keep identifiers, timestamps, optionals, and collection fields semantically consistent across server and TUI.
- Add tests for changed serde behavior or enum representation.
- Avoid exposing internal server persistence details as public types.

## Verification

```bash
cargo test -p fido-types
cargo package -p fido-types
```

## Child DOX Index

No child instruction files.
