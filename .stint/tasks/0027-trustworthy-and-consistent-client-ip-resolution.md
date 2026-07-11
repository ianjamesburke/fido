---
id: "0027"
title: "Trustworthy and consistent client-IP resolution"
status: todo
priority: p2
size: m
created_at: "2026-07-11T05:26:19Z"
blocked_by: []
gh_issue: []
area:
  - "server/http"
  - "infra"
tags:
  - "security"
---


## Why

The deploy path is client -> Railway edge -> nginx -> fido-server (two proxy hops), but IP handling assumes one, and two different extractors disagree:

- nginx `limit_req_zone $binary_remote_addr` (`nginx.conf:17`) sees the Railway edge IP, so every external visitor shares one 10r/s bucket: a single abuser trips 503s site-wide, and there is no real per-client edge limiting.
- `http::extract_client_ip` takes the right-most XFF entry (`fido-server/src/http/headers.rs:17-42`), which under two hops is the nginx-appended Railway edge IP. So all anonymous app-level rate keys and audit IPs collapse to the proxy IP. The "single trusted proxy" comment is wrong.
- `security/admin.rs:19-30` takes the left-most XFF entry, which is fully attacker-forgeable, and it feeds admin-endpoint audit logs, so admin-action IP attribution can be spoofed.
- If the app process is ever directly reachable, XFF is attacker-forgeable end to end and the anonymous IP rate limit is bypassable by rotating the header.

## Done When

- nginx resolves the real client IP from Railway's XFF (`set_real_ip_from <railway range>`, `real_ip_header X-Forwarded-For`, `real_ip_recursive on`) and keys `limit_req_zone` on the resolved IP.
- All in-app IP extraction goes through one helper (`http::extract_client_ip`); `security/admin.rs` no longer uses the spoofable left-most entry. The helper reads only the nginx-validated hop (e.g. `X-Real-IP`) or counts hops against a trusted-proxy allowlist.
- The single-instance assumption of the in-process HTTP limiter (`rate_limit.rs`) is documented, or the limiter is moved behind nginx `limit_req`; note that with >1 replica the in-process limit multiplies.
- Verified the app port is not publicly reachable (only nginx listens externally).
- Tests cover: forged left-most XFF does not change the resolved IP; audit log records the real client IP.

## References

- Security audit 2026-07-11 (deploy + HTTP + fido-server sweep), findings: rate limiting keyed to wrong hop; inconsistent/spoofable client-IP extraction; in-process limiter scope.
- `nginx.conf:17`, `fido-server/src/http/headers.rs:17-42`, `fido-server/src/security/admin.rs:19-30`, `fido-server/src/rate_limit.rs:26-53`.
