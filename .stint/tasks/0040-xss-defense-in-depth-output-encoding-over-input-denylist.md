---
id: "0040"
title: "XSS defense-in-depth: output encoding over input denylist"
status: todo
priority: p4
size: s
created_at: "2026-07-11T05:26:20Z"
blocked_by: []
gh_issue: []
area:
  - "server/security"
tags:
  - "security"
---


## Why

`contains_dangerous_patterns` (`fido-server/src/security/validation.rs:216-267`) is an input denylist (script tags, `on*=`, `javascript:`). Denylists are trivially bypassable (`<svg>`, `<iframe>`, `<img src=x>`, mixed encodings, HTML injection without an event handler). It is safe today only because the server never renders user content into HTML/SVG (the badge interpolates only an `i64`). The risk is false confidence: any future server- or client-side HTML render of stored content would be XSS-prone despite this check.

## Done When

- The denylist is documented in-code as non-authoritative defense-in-depth, not the XSS boundary.
- XSS safety is guaranteed by contextual output encoding at every render sink (the web client escapes on insert); this is verified for the current `../web` client.
- A short note in the security AGENTS.md states: do not add render paths that trust the input filter.

## References

- Security audit 2026-07-11 (HTTP), finding: XSS protection is an input blocklist, not output encoding.
- `fido-server/src/security/validation.rs:216-267`.
