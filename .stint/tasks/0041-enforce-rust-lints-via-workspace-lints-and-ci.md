---
id: "0041"
title: "Enforce Rust lints via workspace.lints and CI"
status: todo
priority: p2
size: s
created_at: "2026-07-11T05:29:08Z"
blocked_by: []
gh_issue: []
area:
  - "workspace"
  - "ci"
tags:
  - "code-quality"
---


## Why

README documents `cargo clippy --workspace --all-targets --all-features -- -D warnings`, but there is no `[workspace.lints]` table in any `Cargo.toml` and `.github/` is empty, so nothing enforces it: the "clippy-clean" state rests entirely on manual discipline, and pedantic/nursery lints never run. A README command nobody runs is not enforcement. Clippy does currently pass clean, so this locks in a good state rather than fixing a broken one.

## Done When

- Root `Cargo.toml` has `[workspace.lints.clippy]` (at least `all = "deny"`, `pedantic = "warn"`), and each crate opts in with `[lints] workspace = true`.
- The `redundant_clone` and (optionally) `unwrap_used` restriction lints are enabled at warn so real needless clones / prod unwraps surface (see the clone-density note in the Rust audit).
- A CI workflow (`.github/workflows/`) runs `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace` on push and PR.
- CI is green on `main`.

## Gotchas

- Turning on pedantic may surface existing nits; cherry-pick/allow the noisy ones rather than blanket-disabling the group.
- Pin GitHub Action versions by verified tag (per repo convention: not all actions publish major-version aliases).

## References

- Rust conventions audit 2026-07-11, finding: no lint enforcement (Cargo.toml + missing .github/).
- Updated skill: /Users/ianburke/.claude/skills/rust-best-practices/SKILL.md (Linting section).
