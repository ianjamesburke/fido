---
id: "0032"
title: "Enforce membership on community roster and metadata reads"
status: todo
priority: p3
size: s
created_at: "2026-07-11T05:26:20Z"
blocked_by: []
gh_issue: []
area:
  - "server/api"
tags:
  - "security"
---


## Why

Any authenticated user can read a community's full member roster (with roles) and metadata for a community they do not belong to:

- `list_members` binds `AuthenticatedUser(_user_id)` but discards the id and never checks membership (`fido-server/src/api/communities.rs:146`).
- `get_community` -> `get_view` returns roster + metadata + channel names with no membership check (`api/communities.rs:120` -> `services/communities.rs:99`).

Impact is bounded (communities map to public GitHub repos; channel messages still require membership via `chat.rs`), so this is roster/metadata disclosure, not message access.

## Done When

- `list_members` calls `require_membership(user_id, community_id)` (stop discarding the id).
- `get_view` (or `get_community`) checks membership before returning roster/channels.
- Tests assert a non-member gets 403 on both endpoints and a member gets 200.
- `cargo test -p fido-server --features sqlite-tests` passes.

## Gotchas

- Batchable with the "Authenticate user search and profile-read endpoints" task (same api layer, same `require_*` pattern).

## References

- Security audit 2026-07-11 (access-control), finding: community metadata and member roster exposed to non-members.
- `fido-server/src/api/communities.rs:120,146`, `fido-server/src/services/communities.rs:99`.
