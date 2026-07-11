---
id: "0030"
title: "Add missing DB indexes on hot query paths"
status: todo
priority: p2
size: s
created_at: "2026-07-11T05:26:19Z"
blocked_by: []
gh_issue: []
area:
  - "server/db"
tags:
  - "performance"
---


## Why

Three hot queries full-scan as data grows because the supporting index is missing:

- memberships: PK is `(community_id, user_id)`, so `WHERE user_id = ?` cannot use it. `list_for_user` and `users_share_community` full-scan `memberships` (`fido-server/src/db/repositories/membership_repository.rs:100-101,113-116`); the latter runs on every DM send (`services/dms.rs:287`).
- posts: `get_post_count` and `calculate_karma` filter on `author_id` with no index (`post_repository.rs:207`, `vote_repository.rs:78-81`); both run per profile view. `posts` is the largest table and every FK is indexed except `author_id`.
- notifications: pagination `ORDER BY created_at DESC LIMIT ? OFFSET ?` (`notification_repository.rs:52-55`) has only `idx_notifications_user_read (user_id, read)`, so every page temp-sorts the user's notifications; deep OFFSET pages get progressively worse.

## Done When

- `SCHEMA` adds: `idx_memberships_user_id ON memberships(user_id)`, `idx_posts_author_id ON posts(author_id)`, `idx_notifications_user_created ON notifications(user_id, created_at DESC)`.
- An `EXPLAIN QUERY PLAN` test (matching the existing feed test at `post_repository.rs:492-535`) asserts these queries use the new indexes and do no temp B-tree sort.
- `cargo test -p fido-server --features sqlite-tests` passes.

## Gotchas

- Consider cursor pagination for notifications (as `MessageRepository` already does) instead of OFFSET, but the index is the minimum fix.

## References

- Performance audit 2026-07-11 (DB), findings: missing indexes on memberships(user_id), posts(author_id), notifications ordering.
- `fido-server/src/db/schema.rs:93-101,118`.
