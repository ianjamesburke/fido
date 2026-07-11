---
id: "0031"
title: "Eliminate feed and DM N+1 query patterns"
status: todo
priority: p2
size: m
created_at: "2026-07-11T05:26:19Z"
blocked_by: []
gh_issue: []
area:
  - "server/db"
tags:
  - "performance"
---


## Why

List endpoints issue one query per row:

- Feed vote lookups: `populate_posts` loops `get_vote(uid, post.id)` per post (`fido-server/src/services/posts.rs:445-450` over `vote_repository.rs:47-72`). A 50-post feed issues 50 queries, each checking out its own pooled connection. Same for reply trees.
- DM conversation list: `get_conversation_summaries` returns only `other_user_id`, then the service loops `users.get_by_id` + `dm_conversations.get` per conversation (`services/dms.rs:44-57` over `dm_repository.rs:110-172`) — 2 extra queries each.

## Done When

- A batch vote method (`get_votes_for_posts(user_id, &[Uuid])`, one query with a generated `IN (?, ?, ...)` placeholder list, values bound) or a LEFT JOIN on the authenticated user replaces the per-post loop in `populate_posts` (and reply trees).
- `get_conversation_summaries` JOINs `users.username` and `dm_conversations.state` so the service no longer loops.
- A test asserts feed/conversation-list render issues O(1) queries, not O(n).
- `cargo test -p fido-server --features sqlite-tests` passes.

## Gotchas

- Keep placeholders generated but values bound — do not interpolate ids into SQL.
- Batchable with the index task (both touch the DB layer); can land in one PR.

## References

- Performance audit 2026-07-11 (DB), findings: N+1 vote lookups on feed; N+1 in DM conversation list.
- `fido-server/src/services/posts.rs:445-450`, `fido-server/src/services/dms.rs:44-57`.
