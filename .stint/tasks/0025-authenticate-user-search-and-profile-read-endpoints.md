---
id: "0025"
title: "Authenticate user search and profile-read endpoints"
status: todo
priority: p2
size: s
created_at: "2026-07-11T05:26:19Z"
blocked_by: []
gh_issue: []
area:
  - "server/api"
tags:
  - "security"
---


## Why

Authentication is enforced per-handler via the `AuthenticatedUser` extractor; there is no global auth middleware (`fido-server/src/lib.rs:59-217`). Two read handlers forgot the extractor and are therefore fully anonymous:

- `search_users` takes only `State` + `Query` (`fido-server/src/api/friends.rs:28`, routed `lib.rs:191`). `GET /users/search?q=a` substring-matches over `users.list_all()` (`services/friends.rs:52-78`) and returns id+username for every match. An anonymous client can dump the entire user table by iterating letters.
- `get_profile` takes only `State` + `Path` (`fido-server/src/api/profile.rs:19`, routed `lib.rs:129`) and returns username, bio, karma, post_count, join_date for any user id with no login.

Every other read endpoint (posts/chat/dms) enforces `AuthenticatedUser` plus membership. The deliberately-public profile variant already exists as `/users/:id/profile-view` using `OptionalUser` (`friends.rs:74`), which is what confirms these two were meant to be authenticated.

## Done When

- `search_users` requires `AuthenticatedUser`.
- `get_profile` either requires `AuthenticatedUser`, or is deleted in favor of the existing `OptionalUser` `profile-view` handler (pick one; do not keep two profile endpoints with different auth).
- A regression test asserts both endpoints return 401 without a valid `X-Session-Token`.
- `cargo test -p fido-server --features sqlite-tests` passes.

## References

- Security audit 2026-07-11 (access-control + fido-server sweep), finding: unauthenticated user enumeration / profile disclosure.
- `fido-server/src/api/friends.rs:28`, `fido-server/src/api/profile.rs:19`, routes at `fido-server/src/lib.rs:129,191`.
