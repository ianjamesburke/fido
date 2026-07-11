# fido-tui/src/app

## Purpose

Application state, feature methods, navigation, and keyboard event dispatch.

## Ownership

- `state.rs` owns `App` and the durable UI state types.
- `mod.rs` exposes state and composes feature modules.
- Feature modules own state transitions for their area.
- `handlers/` owns ordered keyboard dispatch and modal routing.

## Local Contracts

- Treat `App` as the single owner of UI state.
- Keep input priority explicit. Global quit, help, overlays, modals, detail views, and screen-specific keys must continue to resolve in their current order.
- Keep user-visible async failures in app state so rendering can report them.
- Preserve Home behavior outside a repository and board behavior inside a GitHub-backed repository.
- Add or adjust tests in `tests.rs` when a state transition can be checked without a terminal.

## Verification

```bash
cargo test -p fido
just e2e-tui
```

## Child DOX Index

- [`handlers/AGENTS.md`](handlers/AGENTS.md): keyboard event priority and feature dispatch.
