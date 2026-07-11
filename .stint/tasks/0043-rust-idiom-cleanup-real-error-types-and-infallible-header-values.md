---
id: "0043"
title: "Rust idiom cleanup: real error types and infallible header values"
status: todo
priority: p4
size: s
created_at: "2026-07-11T05:29:08Z"
blocked_by: []
gh_issue: []
area:
  - "server"
  - "tui"
tags:
  - "code-quality"
---


## Why

A few non-idiomatic spots flagged by the Rust audit (no crashes today, but they cost composability and route infallible work through fallible paths):

- Server `ApiError` is not a real error type. `pub enum ApiError` derives only `Debug` (`fido-server/src/api/error.rs:15`) — no `Display`/`std::error::Error` — so it cannot compose with `?` / `Box<dyn Error>`. The TUI's `ApiError` already uses `thiserror` correctly, so this is inconsistent.
- Stringly-typed error. `session_restore_task: Option<JoinHandle<Result<Option<RestoredSession>, String>>>` (`fido-tui/src/event_loop.rs:21`) uses `String` as the error type instead of the existing error enum.
- Infallible conversions via `.parse().unwrap()`. Static security-header values are built with `"DENY".parse().unwrap()` etc. (`fido-server/src/security/headers.rs:54,58,62,70,77,85`).

## Done When

- Server `ApiError` derives `#[derive(Debug, thiserror::Error)]` with a `#[error(...)]` per variant, and `#[non_exhaustive]`; existing conversions/usages still compile.
- `event_loop.rs` session-restore error type is the real error enum, not `String`.
- The static header inserts use `HeaderValue::from_static(...)` (no `parse`, no `unwrap`).
- `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace` passes.

## References

- Rust conventions audit 2026-07-11, findings: server ApiError not a real error type; stringly-typed error; static header values via `.parse().unwrap()`.
- `fido-server/src/api/error.rs:15`, `fido-tui/src/event_loop.rs:21`, `fido-server/src/security/headers.rs:54-85`.
