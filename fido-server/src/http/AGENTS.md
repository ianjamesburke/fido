# fido-server/src/http

## Purpose

Shared HTTP extractors for authenticated users and request metadata.

## Ownership

- `auth.rs` owns required and optional session authentication extractors.
- `headers.rs` owns client IP and user-agent extraction.

## Local Contracts

- Keep authentication decisions centralized in extractors and security middleware.
- Treat forwarded headers as untrusted unless the deployment boundary explicitly guarantees them.
- Preserve error behavior used by handlers and WebSocket setup.

## Verification

```bash
cargo test -p fido-server --features sqlite-tests
```

## Child DOX Index

No child instruction files.
