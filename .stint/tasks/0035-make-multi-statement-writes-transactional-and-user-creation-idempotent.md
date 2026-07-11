---
id: "0035"
title: "Make multi-statement writes transactional and user creation idempotent"
status: todo
priority: p3
size: m
created_at: "2026-07-11T05:26:20Z"
blocked_by: []
gh_issue: []
area:
  - "server/db"
tags:
  - "reliability"
---


## Why

No repository exposes a transaction; every multi-step write runs on separately-checked-out pool connections, so a crash mid-sequence corrupts state:

- Vote write: `upsert_vote` then `update_vote_counts` on two connections (`fido-server/src/services/posts.rs:370-371`). A crash between them leaves `posts.upvotes/downvotes` permanently stale — nothing reconciles.
- DM soft-delete: two sequential `UPDATE`s (`dm_repository.rs:188-211`); failure between them leaves the conversation half-hidden.
- User creation: `get_or_create_system_user` and `create_or_update_from_github` do SELECT-then-INSERT on separate connections (`user_repository.rs:99-114,147-184`). Concurrent OAuth callbacks for a new user hit the `username UNIQUE` / `github_id` constraint and 500 instead of returning the existing user; separately, a new GitHub login that collides with an existing (stale) username can never sign up.

## Done When

- A repository transaction primitive exists (`conn.transaction()`), and the vote write and DM soft-delete each run both statements in one transaction.
- User creation uses `INSERT ... ON CONFLICT DO NOTHING` then re-select (matching the existing upsert pattern for votes/configs/tokens); username collisions are disambiguated instead of 500ing.
- The misleading DM request-state error message ("Only the request recipient...") at `services/dms.rs:231-235` is corrected to match the actual guard.
- Tests cover: crash/rollback leaves no half-written vote or DM state; concurrent new-user creation returns one row, no 500.
- `cargo test -p fido-server --features sqlite-tests` passes.

## References

- Security/DB audit 2026-07-11, findings: non-atomic vote write; non-atomic DM soft-delete; check-then-insert user races; signup 500 on username collision; DM error message mismatch.
- `fido-server/src/services/posts.rs:370-371`, `fido-server/src/db/repositories/dm_repository.rs:188-211`, `fido-server/src/db/repositories/user_repository.rs:99-114,147-184`.
