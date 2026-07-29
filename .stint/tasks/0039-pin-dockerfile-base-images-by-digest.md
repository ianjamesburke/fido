---
id: "0039"
title: "Pin Dockerfile base images by digest"
status: todo
priority: p3
size: s
created_at: "2026-07-11T05:26:20Z"
blocked_by: []
gh_issue: []
area:
  - "infra"
tags:
  - "security"
---


## Why

The runtime `FROM` lines use mutable tags: `rust:1.91-bookworm` and `debian:bookworm-slim` (`Dockerfile:6,20`). Builds are not reproducible and a registry-side tag change silently alters the deployed image. This is inconsistent with the care already taken to SHA-256-pin the ttyd binary in the same Dockerfile.

## Done When

- Both `FROM` lines are pinned with `@sha256:` digests.
- A comment records how to refresh the digest (or a justfile target does it).
- The image still builds and deploys.

## References

- Security audit 2026-07-11 (deploy), finding: runtime base images pinned by tag, not digest.
- `Dockerfile:6,20`.
