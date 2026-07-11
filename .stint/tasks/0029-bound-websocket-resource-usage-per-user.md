---
id: "0029"
title: "Bound WebSocket resource usage per user"
status: todo
priority: p2
size: m
created_at: "2026-07-11T05:26:19Z"
blocked_by: []
gh_issue: []
area:
  - "server/realtime"
tags:
  - "security"
---


## Why

An authenticated user can exhaust server resources over WebSockets:

- No per-user connection cap. `ConnectionRegistry::connect` only increments a counter (`fido-server/src/realtime.rs:48-72`); nothing rejects a user opening many simultaneous `/ws` connections. Each connection spawns a task plus a 256-slot broadcast receiver (`api/ws.rs:60-61`), so thousands of sockets exhaust memory/FDs. The global 100/min HTTP limiter counts the upgrade GET but not concurrent long-lived sockets.
- No inbound message throttle or size bound. The server is send-only and silently discards inbound frames (`api/ws.rs:94-109`), but there is no rate limit or per-message size cap, so a client can stream large/rapid frames that the runtime still buffers and decodes (CPU/bandwidth DoS) before discarding them.

## Done When

- `ConnectionRegistry::connect` enforces a max concurrent connections per user (e.g. 5-10); over the limit, the upgrade is rejected / the socket closed with a policy-violation frame.
- The upgrade sets `max_message_size`/`max_frame_size`, and the read loop closes the connection if inbound frame rate exceeds a threshold.
- Tests cover: connection cap rejection; oversized/flooding inbound frames close the socket without unbounded buffering.
- `cargo test -p fido-server --features sqlite-tests` passes.

## References

- Security audit 2026-07-11 (HTTP), findings: no per-connection cap on WebSocket clients; no inbound WS message throttle or size bound.
- `fido-server/src/realtime.rs:48-72`, `fido-server/src/api/ws.rs:60-61,94-109`.
