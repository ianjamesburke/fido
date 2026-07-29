---
id: "0038"
title: "Resolve dead and vestigial server config and endpoints"
status: todo
priority: p3
size: s
created_at: "2026-07-11T05:26:20Z"
blocked_by: []
gh_issue: []
area:
  - "server"
tags:
  - "security"
---


## Why

Three pieces of the server are half-wired: present but unused, which is misleading attack surface and dead config.

- Session cookie set but never read. Login sets an `HttpOnly; SameSite=Strict; Secure` cookie (`fido-server/src/security/cookies.rs:62-74`, `api/auth.rs:149-150,423-424`), but every extractor authenticates only from `X-Session-Token` (`http/auth.rs`, `api/ws.rs`). The cookie is a real session token that no code trusts today — dead surface if any future handler starts reading it, and it means browser clients keep the token in JS-accessible storage anyway.
- `max_request_size` config ignored. `SecurityConfig.max_request_size` is parsed/validated (`security/mod.rs:162-165,244-249`) but the router hardcodes `RequestBodyLimitLayer::new(1024*1024)` (`lib.rs:216`). Tuning `MAX_REQUEST_SIZE` silently does nothing.
- Global-admin endpoints unreachable in prod. `is_admin` is only ever set by seed data for `alice` (`db/schema.rs:264-265`), and seeding is skipped in production, so `/auth/cleanup-sessions` and `/admin/config/validate` (`lib.rs:79-95`) have no reachable caller in prod. Fail-closed (safe) but the admin surface is dead with no supported grant path.

## Done When

- Decide per item: either wire it up or remove it, no vestigial middle state.
  - Cookie: either the server also accepts the session cookie (CSRF covered by SameSite=Strict + header requirement) so browser clients never touch the raw token, or the cookie is removed.
  - `max_request_size`: the body-limit layer uses the configured value.
  - Global admin: either a supported production grant path exists, or the dead endpoints are removed.
- Tests reflect the chosen behavior.
- `cargo test -p fido-server` passes; clippy clean.

## References

- Security audit 2026-07-11 (auth + fido-server sweep), findings: unused session cookie; ignored max_request_size; dead global-admin endpoints.
- `fido-server/src/security/cookies.rs:62-74`, `fido-server/src/lib.rs:79-95,216`, `fido-server/src/security/mod.rs:162-165`.
