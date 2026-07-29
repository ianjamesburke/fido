---
id: "0034"
title: "Harden TUI session-token handling"
status: todo
priority: p3
size: s
created_at: "2026-07-11T05:26:20Z"
blocked_by: []
gh_issue: []
area:
  - "tui"
tags:
  - "security"
---


## Why

The terminal client leaks or under-protects the session token in three ways:

- Temp-file permission window. `session.rs:save` creates the temp file with `fs::File::create` (process umask, typically 0644), writes the plaintext token, and only sets 0600 before the atomic rename (`fido-tui/src/session.rs:145-170`). On a shared host the token is briefly world-readable.
- No HTTPS enforcement. The token is sent in `X-Session-Token` over whatever scheme the base URL uses; the built-in default is HTTPS (`api/client.rs:70`) but `FIDO_SERVER_URL` / `~/.fido/server_url` can point at plaintext `http://` with no guard (`api/client.rs:155-161`), leaking the token on the wire.
- Verbose logging writes the token to a plaintext file. With `--verbose`, tungstenite's TRACE handshake dump includes `x-session-token: <uuid>` written to `fido_debug.log` in the cwd (`fido-tui/src/logging.rs:39`). A stray `fido_debug.log` containing a real token already exists in the repo working dir (untracked).

## Done When

- The session temp file is created with `OpenOptions::new().write(true).create_new(true).mode(0o600)` so it is never group/other-readable.
- Attaching the token to a non-loopback, non-`https://` base URL is refused (or warns loudly and requires opt-in).
- Third-party crates are capped below TRACE (e.g. `tungstenite=info`) or the auth header is redacted; the log file is created 0600.
- The stray working-dir `fido_debug.log` is deleted (it is gitignored, so cleanup only).
- `cargo test -p fido-tui` passes.

## References

- Security audit 2026-07-11 (access-control + deploy), findings: session-token temp file window; no HTTPS enforcement; verbose logging leaks token.
- `fido-tui/src/session.rs:145-170`, `fido-tui/src/api/client.rs:70,155-161`, `fido-tui/src/logging.rs:39`.
