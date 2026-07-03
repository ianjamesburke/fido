# Repo-committed community config (`.fido/community.toml`)

Status: idea, not scheduled. Do not build before M2. Do build M1/M2 in a way that keeps this possible (guardrails below).

## The idea

A community's settings live in a file committed to its repo, not in fido's database. Maintainers merge changes to `.fido/community.toml`; the server reads the file when syncing the community and treats it as the source of truth. The database only caches the last-parsed state.

The trust model is the whole point: only people with write access to the repo can change the file, so GitHub permissions become fido governance. Rule changes arrive as PRs and get reviewed like code. Same pattern as CODEOWNERS and ISSUE_TEMPLATE.

## What the file could hold

```toml
[roles]
# github username -> fido role, granted on top of derived roles
admins = ["octocat"]
moderators = ["alice"]

[moderation]
require_thread_approval = false

[dms]
# who can DM members of this community without a request
member_to_member = "open"   # open | request | off

[channels]
names = ["general", "releases", "help"]

[welcome]
pinned_post = "Read CONTRIBUTING.md before posting."
```

## Sync model

- Fetch `GET /repos/{owner}/{repo}/contents/.fido/community.toml` on community sync (join, claim, periodic refresh, or webhook later).
- Parse with a real TOML parser, strict: unknown keys warn, invalid values fall back to defaults with a visible notice to admins, never crash the community.
- Absent file = current behavior exactly. The file is purely additive.

## Guardrails so we don't build ourselves out of it

1. **One settings-resolution point.** All community behavior flags (`require_thread_approval`, future DM policy, channels) must be read through a single server-side resolver, not scattered column reads. When the file lands, only the resolver changes.
2. **One role-resolution point.** Role assignment (GitHub perms → MembershipRole) stays in one service function so file-granted roles can layer in.
3. **No in-app admin settings UI.** Don't build a settings-editing surface in the TUI that would compete with file-based config. The community modal displays settings; it doesn't edit them.
4. **Settings as data, not columns.** When adding new per-community options, prefer a single settings blob (or resolver-mediated fields) over one-off columns.

## Open questions for when this gets scheduled

- Refresh trigger: poll on activity vs GitHub webhook (webhook needs an app installation, not just OAuth).
- Conflict between file roles and claimed-admin: file wins, or union?
- Should the TUI render the file's existence in the community modal ("governed by .fido/community.toml @ abc123")? Probably yes — it makes governance visible.
