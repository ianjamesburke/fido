# fido v2: Discord for GitHub

Design doc for the v2 rebuild. Every schema and API task in the v2 sprint implements against this document; where a task file and this doc disagree, this doc wins.

## Product shape

fido v1 was a global Reddit-style board with hashtags-as-topics, follows, and DMs. v2 replaces the global feed with **communities keyed to GitHub repositories**. A user browses their starred repos, joins a repo's community, chats in its channels, posts on its threads board, and DMs other members. Repo maintainers claim their community and get admin tools. Realtime delivery is a single authenticated WebSocket.

There is no production data to preserve. The schema is rewritten fresh; no migrations from v1.

## Entities

All tables SQLite. `TEXT` ids are UUIDv4 strings unless noted. Timestamps are `TEXT` ISO-8601 UTC (matches existing convention in `users`/`posts`).

### communities

| column | type | notes |
|---|---|---|
| id | TEXT PK | |
| github_repo_id | INTEGER UNIQUE NOT NULL | GitHub's numeric repo id — the stable key; survives renames |
| owner | TEXT NOT NULL | GitHub owner login at creation/last sync |
| name | TEXT NOT NULL | repo name at creation/last sync |
| claimed_by | TEXT FK→users NULL | user who claimed; NULL = unclaimed |
| require_thread_approval | INTEGER NOT NULL DEFAULT 0 | admin-gated top-level threads |
| created_at | TEXT NOT NULL | |

Communities are **lazily created**: the first `POST /communities/join` for a repo creates the row plus a default `#general` channel. Public repos only for MVP. `owner`/`name` are display data; `github_repo_id` is identity.

### channels

| column | type | notes |
|---|---|---|
| id | TEXT PK | |
| community_id | TEXT FK→communities NOT NULL | |
| name | TEXT NOT NULL | unique per community |
| created_at | TEXT NOT NULL | |

MVP creates only `#general` per community, but every chat API is channel-scoped from day one. No channel-management endpoints in MVP.

### messages

Chat messages. Append-only: no edits, deletes, reactions, or attachments in MVP.

| column | type | notes |
|---|---|---|
| id | TEXT PK | |
| channel_id | TEXT FK→channels NOT NULL | |
| author_id | TEXT FK→users NOT NULL | |
| content | TEXT NOT NULL | validated via `security/validation.rs`; same length policy as posts |
| created_at | TEXT NOT NULL | |

Index `(channel_id, id)` for cursor pagination (before/after message id + limit).

### memberships

| column | type | notes |
|---|---|---|
| community_id | TEXT FK→communities | composite PK with user_id |
| user_id | TEXT FK→users | |
| role | TEXT CHECK IN ('admin','contributor','member') NOT NULL | |
| created_at | TEXT NOT NULL | |

Role semantics:
- **admin** — claimed the community with verified `admin`/`maintain` GitHub permission. Moderation rights: approval queue, future channel management.
- **contributor** — appears in the repo's public contributors list at join time. Badge only in MVP; no extra rights.
- **member** — everyone else.

### posts (altered, not new)

The existing Reddit-style posts/votes/reply-tree stack survives, rescoped:
- add `community_id` TEXT FK→communities NOT NULL — every thread belongs to a community
- add `approved` INTEGER NOT NULL DEFAULT 1 — set 0 on create when the community has `require_thread_approval` and author isn't admin; replies are always approved
- drop hashtag junction tables: `hashtags`, `post_hashtags`, `user_hashtag_follows`, `user_hashtag_activity`; remove hashtag extraction from the post pipeline

`votes`, karma, recursive reply CTE, and sort orders (Newest/Popular/Controversial) are unchanged.

### dm_conversations

Server-enforced message requests. Today the mutual-friends DM gate lives only in the TUI; v2 moves the authority to the server (`services/dms.rs::send_message`).

| column | type | notes |
|---|---|---|
| user_a | TEXT FK→users | composite PK (user_a, user_b), ordered pair: user_a < user_b lexicographically |
| user_b | TEXT FK→users | |
| state | TEXT CHECK IN ('pending','accepted','declined') NOT NULL | |
| initiator_id | TEXT FK→users NOT NULL | who sent the first message |
| created_at | TEXT NOT NULL | |

State machine:
- First DM between two strangers → row created `pending`; initiator may send **exactly one** message while pending.
- Recipient accepts → `accepted`, normal conversation. Declines → `declined`, further sends rejected.
- Auto-accept (no request gate): the pair shares a community membership, or mutual follows exist, at first-send time.

### notifications

One generic pipeline for every "something happened to you" event.

| column | type | notes |
|---|---|---|
| id | TEXT PK | |
| user_id | TEXT FK→users NOT NULL | recipient |
| type | TEXT NOT NULL | 'mention' \| 'reply' \| 'dm_request' \| 'thread_approved' \| 'thread_rejected' |
| actor_id | TEXT FK→users NOT NULL | who caused it |
| subject_type | TEXT NOT NULL | 'message' \| 'post' \| 'dm_conversation' |
| subject_id | TEXT NOT NULL | id in the subject table; TUI uses (subject_type, subject_id) for jump-to-source |
| read | INTEGER NOT NULL DEFAULT 0 | |
| created_at | TEXT NOT NULL | |

Triggers: @mention in chat message, reply to your post, DM request received, your pending thread approved/rejected. Self-notifications suppressed. Unread counts are queryable grouped by community/DM for rail badges.

### github_tokens

The device-flow OAuth token is currently used once and discarded (`oauth.rs`); v2 persists it so the server can list starred repos and verify permissions.

| column | type | notes |
|---|---|---|
| user_id | TEXT PK FK→users | |
| token_ciphertext | BLOB NOT NULL | encrypted at rest |
| created_at | TEXT NOT NULL | |

Encryption key comes from a **required** env var (`FIDO_TOKEN_KEY`); the server throws at startup if missing — no silent fallback. Token is stored on device-flow completion and deleted (best-effort revoked) on logout.

### Surviving v1 tables

`users` (+ github_id/github_login), `follows`, `direct_messages`, `sessions`, `user_configs`, `post_rate_limits`, `dm_rate_limits`, `audit_logs`. The legacy `friendships` table is dropped (dead: created but never read or written; `follows` is the live social graph).

## Claim flow

Verification uses the claimant's own stored GitHub token via an internal `GithubService`:

- `starred_repos(user)` — `GET /user/starred` with the user's token; powers `GET /communities/browse`.
- `repo_permission(user, owner, name)` — `GET /repos/{owner}/{name}` with the user's token; reads the `permissions` object (`admin`, `maintain`, `push`, ...). Returned only when the token holder has access — exactly what claim verification needs.
- `is_contributor(user, owner, name)` — `GET /repos/{owner}/{name}/contributors` (public), checks the user's github_login.

Flows:
1. **Join** (`POST /communities/join`): lazy-create community + `#general` if absent; insert membership. Role is `contributor` if `is_contributor`, else `member`.
2. **Claim** (`POST /communities/:id/claim`): call `repo_permission`; if `admin` or `maintain` is true, set `communities.claimed_by` and upgrade the membership role to `admin`. Otherwise 403 with the permission level found. Multiple admins allowed (each maintainer can claim); `claimed_by` records the first.

All GitHub calls are try-caught with operation name + inputs logged; failures surface as errors, never silently degrade.

## API surface (v2 additions)

Defined here so tasks 0005–0009 don't invent divergent shapes. Existing conventions apply: session-token auth middleware, rate limiting, `ErrorResponse`.

Communities (0005):
- `GET /communities/browse` — user's starred repos annotated `{repo, community_exists, joined, member_count}`
- `POST /communities/join` — body `{github_repo_id, owner, name}`; lazy-create
- `GET /communities` — joined list with unread summaries
- `GET /communities/:id`
- `POST /communities/:id/claim`
- `DELETE /communities/:id/membership` — leave

Chat (0006):
- `GET /communities/:id/channels`
- `GET /channels/:id/messages?before=<msg_id>&after=<msg_id>&limit=<n>` — cursor pagination
- `POST /channels/:id/messages` — membership-gated

Threads board (0007): existing post endpoints gain required `community_id`; plus `GET /communities/:id/posts/pending` (admin) and `POST /posts/:id/approve` (admin).

DMs (0008): existing DM endpoints, plus `GET /dms/requests`, `POST /dms/requests/:user_id/accept`, `POST /dms/requests/:user_id/decline`.

Notifications (0009): `GET /notifications` (paginated), `GET /notifications/unread-count` (grouped for rail badges), `POST /notifications/mark-read` (single id or all).

Role checks are a reusable extractor/middleware (pattern: `security/admin.rs`), parameterized by community id + minimum role.

## WebSocket event catalog

Single realtime transport: `GET /ws`, axum WebSocket upgrade.

**Auth handshake**: the client connects with its session token (`Authorization: Bearer <token>` header, or `?token=` fallback for clients that can't set headers on upgrade). The server validates via the existing session layer before upgrading; invalid token → HTTP 401, no upgrade.

**Envelope** (serde types live in `fido-types`, shared by server and TUI):

```json
{ "type": "<EventType>", "payload": { ... }, "ts": "<ISO-8601>" }
```

**Events** (server → client only in MVP; clients send nothing but pings):

| type | payload | routed to |
|---|---|---|
| `MessageCreated` | full message DTO + channel_id + community_id | members of the community |
| `ThreadCreated` | post DTO + community_id | members of the community |
| `ThreadPendingApproval` | post DTO + community_id | community admins |
| `DmRequestCreated` | conversation DTO + first message | recipient |
| `DmMessageCreated` | DM DTO | both participants |
| `NotificationCreated` | notification DTO | recipient |

**Bus**: in-process tokio broadcast. API handlers publish; one subscriber task per connection filters by the user's memberships + DM participation. Single-node SQLite deployment — the bus never needs to leave the process. Tasks 0006–0009 emit through a no-op `EventBus` trait until 0010 supplies the real one; the trait's shape (fn `publish(Event)`) is fixed here so emit sites don't churn.

**Lifecycle**: server pings every 30s, drops connections that miss 2 pongs; clean shutdown closes with a going-away frame; per-user connection registry (a user may hold multiple connections).

**Polling fallback contract**: every pushed event is also observable by re-fetching the corresponding GET endpoint — the socket is an optimization, never the only source of truth. When the TUI's socket is down it degrades to interval polling of the visible surface plus `GET /notifications/unread-count`, and surfaces connection status in the status bar. Clients must reconcile optimistic/polled state with pushed events by id (no double-apply).

## Deletions (stint 0002)

Ground cleared before any v2 code lands:

- `fido-server/src/stores/` — duplicate trait layer. Seven of ten impls are pass-throughs to `db/repositories/`; the three with real SQL (session, rate-limit, audit) move into new repositories first. `SessionRecord` and `DirectMessageConversationSummary` move to the repository layer (breaking `dm_repository.rs`'s reverse import from stores).
- `fido-tui/src/api/mock_backend.rs` (~800 lines) + `sample_data.rs` (~670 lines) + the `Backend` enum wrapper — no trait, no test coverage depends on it; `ApiClient` is used directly. `FIDO_DEMO_MODE` and `App::demo()` go with it.
- `fido-migrate` crate — hashtag backfill tooling for a model being deleted; removed from workspace and Dockerfile.
- `friendships` table — dead; `follows` is the live graph.
- `fido-tui/src/ui/modals/social_refactored_example.rs` — not in the mod tree.
- `fido-tui/src/reply_debug_log.rs` + its test — debug scaffolding with no production callers. (`social_components.rs` stays: actively used by `social.rs`, contrary to the task file's guess.)
- `web/mockup-*.html` + stale terminal easter-egg strings in `web/script.js`.

Deleted later, by their own tasks: hashtag tables/extraction/API/UI (0003/0007/0014), the TUI mutual-friends DM gate (0015), Reddit-era docs (0017).

## Known debt noted, not addressed here

- The axum router is defined twice (`lib.rs::create_app` and duplicated in `main.rs`); v2 API tasks should route through one definition and delete the copy.
- `DirectMessage` DTO lacks the `deleted_by_*` fields present on the table.
- No CI. `just test` (`cargo test --workspace`) is the gate every task must keep green.
