# M1: Launch Readiness — Design

Date: 2026-07-02
Status: awaiting review
North star: fastest way to connect with a developer (see NORTH_STAR.md)

## Goal

Get Fido stable enough to put real developers in the fido repo community and collect feedback. M1 fixes the broken connection path (profiles, DMs, community modal, search discoverability), then ships. Rich GitHub profiles and markdown rendering are M2, scoped by what launch feedback asks for.

## Findings that shaped this scope

- DM stranger/mutual gating already works server-side (`fido-server/src/services/dms.rs:233`): strangers get a one-message request; mutual follows or shared-community members auto-accept. No rewrite needed. The rough edges are TUI-side.
- User search already exists (`fido-server/src/services/friends.rs:52`, substring match over all usernames, cap 20). The gap is discoverability in the UI, not the backend.
- A UI components module already exists (`fido-tui/src/ui/components/`). No extraction refactor. The community modal's centering is one styling choice, not a systemic problem.
- `p` (view profile) is a dead stub: empty handler at `fido-tui/src/app/friends.rs:153`, and `UserProfileViewState` is never constructed anywhere. The render path and key handler for the open state exist but are unreachable.
- GitHub OAuth scope is `user:email` only. Public commit activity and profile READMEs are fetchable without a new scope, but the app has zero markdown rendering, which is the real cost of README display.

## M1 scope

### 1. Wire user profile viewing (`p`)

The connection path's missing link. From the friends list, user search results, a post author, or a DM conversation, `p` opens the profile view.

- Server: `GET /users/{username}/profile` returning `UserProfile` (username, bio, join date, follower/following/post counts, relationship to viewer: following / follower / mutual / none).
- TUI: construct `UserProfileViewState` from that response and open the existing view. Fill in the stubbed `f` (follow/unfollow toggle) and `m` (message → opens or creates the DM conversation) keys inside it.
- `p` works from every list that shows a username. Same key, same behavior, everywhere (commandment 4).

Profile → `m` → typing a message is the end-to-end connection path. That flow working smoothly is M1's definition of done.

### 2. DM polish (TUI-side)

No server rewrite. Fix the client experience:

- Pending-request state is visible and actionable: incoming requests show accept/decline, outgoing show "pending".
- Conversation list updates live (websocket events already exist server-side).
- Clear empty states: what a stranger sees ("your message will be sent as a request") vs a mutual.
- Whatever concrete bugs an e2e pass over `app/dms.rs` turns up.

### 3. Community modal rework

- Left-align content, structured layout (use `ui/components/panel.rs` patterns rather than centered wrapped text).
- Show: description, member count, your role, and the admin/owner list (membership roles are already in the db).
- Keep `c: Claim admin` where applicable.

### 4. Search discoverability

- Global "find a developer" entry point, not buried in a friends sub-tab. `/` from the friends tab already searches; make search results support `p` (profile) and `m` (message) directly.
- Server search stays as-is for M1 (username substring is fine at current scale).

### 5. Release + feedback loop

- Fix or gate anything that crashes in the e2e harness (`just e2e-tui`) and a manual pass of the main flows.
- Publish to crates.io (`cargo install fido`), verify the Railway web demo matches.
- Seed the fido repo community as the flagship: post a welcome thread, a feedback thread, and a "what should we build" thread. The app dogfoods its own feedback channel (commandment 3: the repo is the room).
- Announce where developers already are: a Show HN / r/rust / r/commandline post, plus the Kiroween audience. One announcement at a time, so feedback stays attributable.

## Explicitly out of M1 (M2 candidates, ordered)

1. Markdown renderer (markdown → ratatui). Prerequisite for both README display and md-viewing packages.
2. GitHub-rich profiles: contribution graph (colored cells, no markdown needed — could land before the renderer), profile README.
3. Group chats: not planned. Communities are the group surface; revisit only if launch feedback demands it.
4. Server-side search improvements (bio search, ranking).

## Testing

- Each M1 item gets e2e coverage in the existing tmux harness where feasible (profile open, DM request accept flow, search → profile → message).
- `cargo test`, `cargo clippy`, `cargo fmt` clean before release.

## Error handling

- Profile fetch failure: show the modal with an inline error line, not a silent no-op (a dead `p` is what we're fixing).
- DM send rejection (declined conversation, pending limit): surface the server's reason verbatim in the composer.
