# Directory-Scoped Communities: TUI Reimagining

Date: 2026-07-02
Status: approved (Ian, in-session)

## Decision

The directory you launch `fido` from decides the community. No picker, no
persistent community rail.

- Launched inside a git repo with a GitHub `origin` remote: the TUI opens
  directly to that repo's community board, auto-joining (which lazily creates
  the community server-side on first visit).
- Launched anywhere else: the TUI opens to a Home view listing the user's
  joined communities (Enter opens one) plus DMs/Profile/Settings as today.
  Empty state tells the user to launch fido inside a repo.

This supersedes the persistent-left-rail layout in `.stint/tasks/0012` and
replaces the hardcoded default-community bootstrap in the current WIP.

## Scope

1. **Repo context detection (TUI)** — new `fido-tui/src/repo_context.rs`.
   At startup: `git -C <cwd> rev-parse --show-toplevel` (git walks up to the
   nearest repo itself), then `git -C <top> remote get-url origin`, parse
   `owner/name` from GitHub https/ssh remote forms. No repo, no origin, or a
   non-GitHub remote all yield `None` (Home mode) — detection, not config, so
   absence is a valid state, logged at info.

2. **Join by owner/name (server)** — `POST /communities/join` request becomes
   `{ owner, name }`; `github_repo_id` is removed from the request. The server
   resolves the repo via GitHub `GET /repos/{owner}/{name}` — with the caller's
   stored token when present, otherwise unauthenticated (public repos) — and
   uses the canonical id/owner/name from the response. Unresolvable repo → 404
   naming the repo. New `GithubService::get_repo`.

3. **Community context state (TUI)** — replace
   `current_community_id`/`current_community_label` with
   `community: Option<CommunityContext>` carrying id, owner/name, the user's
   `MembershipRole`, member_count, claimed flag, and
   `require_thread_approval`. Populated from the join/get response after
   login and after session restore.

4. **UI**
   - Posts tab is the Board. Repo mode: title `owner/name · role · N members`.
   - Role badge renders admin/contributor/member from membership.
   - Claim: `C` on the board when the community is unclaimed; server verifies
     GitHub admin/maintain. Success updates the badge; Forbidden shows the
     server's message inline.
   - Home mode: Posts tab renders the joined-communities list; Enter loads
     that community's board (`GET /communities/:id`), Esc returns to the list.
   - DMs/Profile/Settings tabs unchanged. Hashtag-follow UI untouched this
     pass (dies with stint 0014).

5. **E2E TUI harness** — `just e2e-tui` → `scripts/e2e_tui.sh`:
   - Starts a GitHub API stub (fixture responses for `/repos/...`,
     `/repos/.../contributors`) and `fido-server` with a temp DB and
     `GITHUB_API_BASE` pointed at the stub.
   - Creates a temp git repo with a fake GitHub origin, launches the real TUI
     binary inside it under tmux, logs in as a seeded test user, and asserts
     via `tmux capture-pane` + server API + TUI log file: board title shows
     the repo community, posting works, and launching outside a repo lands on
     Home.

## Non-Scope

- Chat channels and WebSocket-driven state (stints 0011/0013).
- Admin approval queue, member management UI (stint 0014).
- Starred-repo browsing UI (`/communities/browse` stays server-only for now;
  discovery on Home can come later).
- Leave-community UI (endpoint exists; no keybind this pass).

## Error Handling

- Join failure in repo mode (server down, repo 404, private repo without
  token): board pane shows the categorized error with a retry keybind; the
  app stays usable (DMs/Profile/Settings still work).
- No silent fallback from repo mode to Home mode: if the directory names a
  repo, the app is that repo's community or an explicit error.

## Testing

- Unit: remote-URL parsing table (https, ssh, `.git` suffix, non-GitHub).
- Server e2e (`e2e_community_rewrite.rs`): update join tests for the new
  request shape; add resolve-failure case.
- TUI e2e: harness above, run in CI-able script form.
