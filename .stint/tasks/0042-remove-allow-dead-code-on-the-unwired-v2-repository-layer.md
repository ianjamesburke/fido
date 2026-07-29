---
id: "0042"
title: "Remove allow(dead_code) on the unwired v2 repository layer"
status: todo
priority: p3
size: s
created_at: "2026-07-11T05:29:08Z"
blocked_by: []
gh_issue: []
area:
  - "server/db"
tags:
  - "code-quality"
---


## Why

Seven repository files crate-suppress dead-code warnings for an unwired "v2 repository" API:

`#![allow(dead_code)]` at line 1 of `fido-server/src/db/repositories/{mod,community,membership,dm_conversation,channel,message,notification}_repository.rs`.

This directly violates the project's own convention (no `#[allow(dead_code)]`: delete, wire up, or feature-flag). The suppression hides how much of the v2 layer is actually reachable and lets genuinely-dead code accumulate silently.

## Done When

- Every `#![allow(dead_code)]` in those seven files is removed.
- Each item is resolved: wired into a real call path, deleted, or gated behind an explicit feature flag (e.g. `#[cfg(feature = "v2-repos")]`) that is documented.
- `cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` are clean with no dead-code allow.
- `cargo test --workspace` passes.

## References

- Rust conventions audit 2026-07-11, finding: `#![allow(dead_code)]` papering over unfinished v2 layer.
- `fido-server/src/db/repositories/mod.rs:1` and six sibling `_repository.rs` files.
