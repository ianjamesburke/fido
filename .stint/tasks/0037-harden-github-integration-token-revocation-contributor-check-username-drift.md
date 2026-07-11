---
id: "0037"
title: "Harden GitHub integration: token revocation, contributor check, username drift"
status: todo
priority: p3
size: s
created_at: "2026-07-11T05:26:20Z"
blocked_by: []
gh_issue: []
area:
  - "server"
tags:
  - "security"
---


## Why

Three GitHub-identity gaps:

- Token revocation silently skipped. `try_revoke_remote_token` returns early with only a `tracing::debug!` if `GITHUB_CLIENT_ID`/`GITHUB_CLIENT_SECRET` are missing (`fido-server/src/services/github.rs:373-383`). Production startup never requires the client secret, so logout deletes the local ciphertext but leaves the GitHub grant live indefinitely, with nothing above debug level saying so.
- Contributor check on stale, case-sensitive username. Community-join Contributor role compares the repo contributors list against the local `users.username` (`github.rs:257-274`, `user_repository.rs:156-165`), but `username` is frozen at first signup while only `github_login` updates on re-login, and the compare is case-sensitive though GitHub logins are not. After a rename+recycle the wrong user can match.
- `username` never updated on GitHub rename (`user_repository.rs:147-165`), the root cause of the above and of stale display identity.

## Done When

- Production startup requires `GITHUB_CLIENT_SECRET` (same fail-fast block as `validate_token_key`), or at minimum the revocation skip logs at warn level.
- The contributor check compares against the current `github_login` case-insensitively (or verifies via an authenticated GitHub collaborators call).
- `create_or_update_from_github` updates `username`/display identity on GitHub rename.
- Tests cover: revocation attempted when secret present; case-insensitive contributor match; rename updates the stored login.
- `cargo test -p fido-server` passes.

## References

- Security audit 2026-07-11 (auth + fido-server sweep), findings: token revocation skipped when secret unset; is_contributor trusts stale case-sensitive username; username drift on rename.
- `fido-server/src/services/github.rs:257-274,373-383`, `fido-server/src/db/repositories/user_repository.rs:147-165`.
