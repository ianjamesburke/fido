---
id: "0033"
title: "Reject self-votes and scope mention notifications to members"
status: todo
priority: p3
size: s
created_at: "2026-07-11T05:26:20Z"
blocked_by: []
gh_issue: []
area:
  - "server/posts"
tags:
  - "security"
---


## Why

Two content-integrity gaps in the posts service:

- Self-voting inflates karma. `record_vote` checks visibility/membership but never compares `post.author_id` to the voter (`fido-server/src/services/posts.rs:357-374`); `calculate_karma` counts all up-votes on a user's posts (`vote_repository.rs:75-86`), so a user can upvote their own posts to inflate karma. (Counts cannot be forged and multi-vote is deduped by the upsert.)
- Mentions notify arbitrary users. `notify_mentions` resolves every `@username` to any existing user regardless of community membership (`services/posts.rs:507-521`, called from `create_post` `posts.rs:343`), so a member can @-mention users outside the community and generate cross-community notification spam.

## Done When

- `record_vote` rejects a vote where `post.author_id == user_id` (BadRequest), or `calculate_karma` excludes self-votes.
- `notify_mentions` only notifies mentioned users who are members of `post.community_id`.
- Tests cover: self-upvote rejected/uncounted; mention of a non-member produces no notification.
- `cargo test -p fido-server --features sqlite-tests` passes.

## References

- Security audit 2026-07-11 (access-control), findings: self-voting inflates karma; mention notifications not scoped to community membership.
- `fido-server/src/services/posts.rs:357-374,507-521`.
