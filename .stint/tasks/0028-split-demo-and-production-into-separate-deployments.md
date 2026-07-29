---
id: "0028"
title: "Split demo and production into separate deployments"
status: todo
priority: p2
size: l
created_at: "2026-07-11T05:26:19Z"
blocked_by: []
gh_issue: []
area:
  - "infra"
tags:
  - "security"
---


## Why

The installed TUI default server URL is `https://fido-web-production.up.railway.app` (`fido-tui/src/api/client.rs:70`) — the same deployment that runs the anonymous web-terminal demo. `start.sh:123-127` deletes the DB on every boot unless `FIDO_DEMO_EPHEMERAL=false`. So either:

- Railway keeps the ephemeral default, and real users' accounts plus encrypted GitHub tokens are wiped on every deploy; or
- Railway sets it false, and anonymous ttyd visitors drive a TUI against the persistent production DB.

Both are wrong, and which one is live depends on unverifiable Railway env vars. The demo and the production API must not be the same service.

## Scope

- Demo deployment: hardcoded ephemeral DB, web mode, ttyd + nginx, no real user data.
- Production API deployment: persistent volume, no ttyd, no anonymous terminal.
- Point `DEFAULT_PUBLIC_SERVER_URL` / `client.rs` default at the API service, not the demo host.

## Done When

- Two distinct Railway services (or equivalent) exist: demo (ephemeral, ttyd) and production API (persistent `/data`, no ttyd).
- Installed TUI default server URL points at the persistent API service.
- Anonymous web-terminal visitors provably cannot reach the persistent production DB.
- Deploy of the API service does not wipe the DB.
- README/QUICKSTART/deploy docs updated to describe the two-service topology.

## References

- Security audit 2026-07-11 (deploy), finding: demo host doubles as production API; DB persistence env-dependent.
- `start.sh:123-127`, `fido-tui/src/api/client.rs:70`, `start.sh:285-292` (ttyd).
