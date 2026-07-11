# fido-tui/src

## Purpose

Terminal client implementation: startup, terminal lifecycle, app state, input dispatch, HTTP and WebSocket API access, and rendering.

## Ownership

- `main.rs` owns CLI parsing, startup sequencing, terminal initialization, and the main event loop handoff.
- `event_loop.rs` drives events and redraws.
- `repo_context.rs` detects GitHub repository context for directory-scoped communities.
- `terminal.rs` owns terminal setup and restoration.
- `session.rs` owns local session persistence.

## Local Contracts

- Preserve first-frame-before-network behavior in startup.
- Keep side effects out of render functions. Keep state transitions and key dispatch in `app/`.
- Send HTTP and WebSocket work through `api/`; do not construct client requests from UI modules.
- Always restore the terminal on exit or error paths.

## Verification

```bash
cargo test -p fido
just e2e-tui-startup
just e2e-tui
```

## Child DOX Index

- [`api/AGENTS.md`](api/AGENTS.md): server client and realtime connection.
- [`app/AGENTS.md`](app/AGENTS.md): application state and input behavior.
- [`ui/AGENTS.md`](ui/AGENTS.md): Ratatui rendering and visual components.
