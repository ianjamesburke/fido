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

## Verification

```bash
cargo test -p fido-server --features sqlite-tests
cargo test --test test_https_cookies -p fido-server
```

## Child DOX Index

No child instruction files.
