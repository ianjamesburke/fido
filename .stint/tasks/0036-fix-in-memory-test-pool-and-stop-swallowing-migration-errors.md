---
id: "0036"
title: "Fix in-memory test pool and stop swallowing migration errors"
status: todo
priority: p3
size: s
created_at: "2026-07-11T05:26:20Z"
blocked_by: []
gh_issue: []
area:
  - "server/db"
tags:
  - "reliability"
---


## Why

Two connection-setup defects in `fido-server/src/db/connection.rs`:

- `SqliteConnectionManager::memory()` (`connection.rs:130-131`) opens an independent `:memory:` database per pooled connection. `initialize()` runs the schema on one connection; tests pass only because single-threaded r2d2 hands back the same idle connection. Any concurrent use of `Database::in_memory()` sees empty databases.
- Every migration is `let _ = conn.execute(...)` to tolerate "duplicate column" on re-run (`connection.rs:151-218`), which also masks genuine failures (disk full, locked DB, malformed schema), surfacing later as confusing errors far from the cause.

## Done When

- The in-memory pool shares one database across connections (`file:fido_mem?mode=memory&cache=shared` with URI flags) or is capped at `max_size(1)`.
- Migrations inspect the error and ignore only `duplicate column name`, propagating everything else; longer term gate on `PRAGMA user_version`.
- A test exercises the in-memory pool across >1 connection and sees the schema.
- `cargo test -p fido-server --features sqlite-tests` passes.

## References

- DB audit 2026-07-11, findings: in-memory pool gives each connection a distinct database; migration errors silently swallowed.
- `fido-server/src/db/connection.rs:130-131,151-218`.
