---
id: "0026"
title: "Fail-closed library router (drop Development default)"
status: todo
priority: p2
size: s
created_at: "2026-07-11T05:26:19Z"
blocked_by: []
gh_issue: []
area:
  - "server"
tags:
  - "security"
---


## Why

`create_router()` does `SecurityConfig::from_env().unwrap_or_else(|_| SecurityConfig::default())` (`fido-server/src/lib.rs:33-38`). The default is `Environment::Development`, and `lib.rs:69-73` mounts the passwordless `/auth/login` and `/users/test` credential-free login routes whenever the environment is not production. So any consumer of this `pub` library function that runs without a valid `ENVIRONMENT` var silently gets the login bypass mounted.

The shipped binary is safe (`main.rs:42-52` uses `create_router_with_security_config` and hard-exits on config failure), but this fail-open path directly contradicts the fail-closed design used everywhere else — `is_production_runtime` (`security/mod.rs:80-87`) treats unknown environments as production. The only current guard is a doc comment saying it is "for testing".

## Done When

- `create_router` no longer defaults to `Development` on config error: it either returns `Result<Router>` and propagates the error, or is gated behind `#[cfg(test)]` / a test-only feature so it cannot be reached in a normal build.
- No non-test entry point can mount the passwordless login routes outside an explicitly-development environment.
- `cargo test -p fido-server` passes; `cargo clippy --workspace --all-targets -- -D warnings` clean.

## References

- Security audit 2026-07-11 (auth), finding: library router falls back to Development security config (fail-open).
- `fido-server/src/lib.rs:33-38`, test-route mount `fido-server/src/lib.rs:69-73`, fail-closed contrast `fido-server/src/security/mod.rs:80-87`.
