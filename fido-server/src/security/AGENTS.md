# fido-server/src/security

## Purpose

Environment-aware security configuration, CORS, cookies, headers, admin access, validation, and audit records.

## Ownership

`mod.rs` owns `SecurityConfig` and `Environment`; child modules own their named policy surface.

## Local Contracts

- Production configuration must fail closed. Do not default an invalid or missing environment to development.
- Do not weaken secure-cookie, CORS, admin, header, or token requirements to make a local test pass.
- Make policy changes explicit in tests for development and production behavior.
- Keep secrets out of logs and error messages.
- `validation::contains_dangerous_patterns` is a non-authoritative input denylist (defense-in-depth), NOT the XSS boundary. XSS safety comes from contextual output encoding at each render sink. Do NOT add a server- or client-side path that renders stored user content into HTML/SVG while trusting this filter; encode at the sink instead.

## Verification

```bash
cargo test -p fido-server --features sqlite-tests
cargo test --test test_https_cookies -p fido-server
```

## Child DOX Index

No child instruction files.
