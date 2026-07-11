# fido-types

## Purpose

Shared serializable models, events, and enums used by the server and terminal client.

## Ownership

- `src/models.rs` owns domain entities and API payload types.
- `src/events.rs` owns realtime event types.
- `src/enums.rs` owns shared enum types.
- `src/lib.rs` defines the public export surface.

## Local Contracts

- This crate is a public contract for both workspace consumers and crates.io users.
- Preserve serde field names and enum encodings unless the server and client migration is deliberate and tested.
- Prefer additive, backward-compatible changes. Document or version any unavoidable wire-format break.
- Keep transport types free of server, terminal, database, or UI dependencies.

## Verification

```bash
cargo test -p fido-types
cargo package -p fido-types
```

## Child DOX Index

- [`src/AGENTS.md`](src/AGENTS.md): source-level public type contracts.
