# Repo Activity Feed + Community Badge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** GitHub issues/PRs from the last 14 days appear as a distinct read-only item type in the community feed (cached server-side, non-blocking TUI load, `o` opens in browser), plus a public live SVG badge endpoint showing member count.

**Architecture:** New `ActivityItem` type in fido-types. Server fetches from GitHub's issues endpoint (one request, PRs included) through the existing `GithubService`, caches the serialized payload per community in sqlite with a 10-minute TTL, and serves it at `GET /communities/:id/activity`. The TUI loads activity after the board renders (pending-load pattern, like posts), interleaves items with posts by `created_at` via a `FeedEntry` index layer, and styles them as ambient rows. Badge is an unauthenticated endpoint rendering a `format!`-templated SVG.

**Tech Stack:** Rust workspace — fido-types (serde models), fido-server (Axum + rusqlite), fido-tui (ratatui, existing `webbrowser` dep).

**Spec:** `docs/superpowers/specs/2026-07-03-repo-activity-feed-and-badge-design.md`

## Global Constraints

- Activity items are NOT posts: no votes, no replies, no post id, never written to the posts table.
- Activity fetch must never block board rendering; a dim `⊙ loading repo activity...` row shows while loading.
- One GitHub request per fetch: `GET /repos/{owner}/{name}/issues?state=all&since=<now-14d>&per_page=100&sort=created&direction=desc`, authenticated with the requesting user's stored token when present, `get_public` fallback otherwise.
- Cache TTL 10 minutes. GitHub fetch failure with stale cache: serve stale, `tracing::warn!`. Failure with no cache: propagate the error (no silent fallback).
- A key shown in a footer must work; a key that does nothing on the current selection must not be shown (M1 footer-honesty rule). `o` appears only when an activity item is selected.
- Enter, `u`, `d`, `p`, reply keys are no-ops on activity items.
- New async key behavior goes in `event_loop.rs::handle_async_key_events` as guarded arms BEFORE the final `_ =>` arm.
- New server routes are registered in BOTH `fido-server/src/lib.rs` and `fido-server/src/main.rs`.
- Badge endpoint is public (no auth), `Content-Type: image/svg+xml`, `Cache-Control: public, max-age=300`, 404 plain text when no community exists.
- Test fixtures for GitHub responses must be snapshots of real API responses (fetch with curl), never invented shapes.
- All error handling: name the operation, log inputs and original error (`tracing::warn!`/`anyhow::Context` patterns already in the codebase).
- `cargo clippy --workspace --all-targets -- -D warnings` must stay clean.

---

### Task 1: ActivityItem model in fido-types

**Files:**
- Modify: `fido-types/src/enums.rs`
- Modify: `fido-types/src/models.rs`
- Modify: `fido-types/src/lib.rs` (only if exports are listed explicitly; if it's `pub use` globs, no change)

**Interfaces:**
- Produces: `fido_types::{ActivityItem, ActivityKind, ActivityState}` — used by server (Tasks 2-4) and TUI (Tasks 5-6).

- [ ] **Step 1: Write the failing test**

Append to the existing tests in `fido-types/src/models.rs` (create a `#[cfg(test)] mod tests` at the bottom if none exists):

```rust
#[cfg(test)]
mod activity_tests {
    use super::*;
    use crate::enums::{ActivityKind, ActivityState};

    #[test]
    fn activity_item_serde_round_trip() {
        let item = ActivityItem {
            github_id: 123456,
            kind: ActivityKind::PullRequest,
            number: 42,
            title: "Add dark mode".to_string(),
            author_login: "alice".to_string(),
            state: ActivityState::Merged,
            created_at: "2026-07-01T12:00:00Z".parse().unwrap(),
            html_url: "https://github.com/o/r/pull/42".to_string(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"pull_request\""));
        assert!(json.contains("\"merged\""));
        let back: ActivityItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.github_id, 123456);
        assert_eq!(back.kind, ActivityKind::PullRequest);
        assert_eq!(back.state, ActivityState::Merged);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p fido-types activity_item_serde_round_trip`
Expected: FAIL to compile — `ActivityItem` not found.

- [ ] **Step 3: Implement**

In `fido-types/src/enums.rs`, following the file's existing derive/serde style:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Issue,
    PullRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityState {
    Open,
    Closed,
    Merged,
}
```

In `fido-types/src/models.rs` (imports: add `ActivityKind, ActivityState` to the existing `crate::enums` use):

```rust
/// A GitHub issue or PR surfaced in a community feed. Read-only ambient
/// content — never a post: no votes, no replies, no post id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityItem {
    pub github_id: i64,
    pub kind: ActivityKind,
    pub number: i64,
    pub title: String,
    pub author_login: String,
    pub state: ActivityState,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
    pub html_url: String,
}
```

Export from `lib.rs` the same way `Post`/`RelationshipStatus` are exported.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p fido-types activity_item_serde_round_trip`
Expected: PASS. Then `cargo build --workspace` — 0 errors.

- [ ] **Step 5: Commit**

```bash
git add fido-types/src
git commit -m "feat(types): ActivityItem model for repo activity feed"
```

---

### Task 2: GithubService::repo_activity

**Files:**
- Modify: `fido-server/src/services/github.rs`
- Create: `fido-server/tests/fixtures/github_issues_sample.json` (or inline fixture string in the test module — inline is fine, but the content must be a real snapshot)

**Interfaces:**
- Consumes: `fido_types::{ActivityItem, ActivityKind, ActivityState}` (Task 1); existing `get_with_token`, `get_public`, `load_token`, `log_result` helpers in `github.rs`.
- Produces: `GithubService::repo_activity(&self, user_id: Uuid, owner: &str, name: &str) -> Result<Vec<ActivityItem>>`; private pure fn `map_issue_to_activity(issue: GithubIssue) -> ActivityItem`.

- [ ] **Step 1: Capture a real fixture**

Run:
```bash
curl -s "https://api.github.com/repos/ianjamesburke/fido/issues?state=all&per_page=3" -H "Accept: application/vnd.github+json"
```
Take the real response and reduce each element to the fields the deserializer needs plus a few extras GitHub actually sends (extras prove `#[serde(default)]`-free deserialization tolerates unknown keys — serde ignores unknown fields by default, keep it that way). The fixture MUST include: one plain issue (no `pull_request` key), one open PR (`pull_request` present, `merged_at: null`), one merged PR (`pull_request.merged_at` set). If the live repo lacks one of these shapes, take the missing shape from `curl -s "https://api.github.com/repos/rust-lang/rust/issues?state=all&per_page=10"` — still a real snapshot, just a different repo.

- [ ] **Step 2: Write the failing tests**

In `github.rs` tests module:

```rust
#[test]
fn maps_issue_pr_and_merged_pr() {
    let raw = include_str!("../../tests/fixtures/github_issues_sample.json");
    let issues: Vec<GithubIssue> = serde_json::from_str(raw).expect("fixture parses");
    let items: Vec<ActivityItem> = issues.into_iter().map(map_issue_to_activity).collect();

    let issue = items.iter().find(|i| i.kind == ActivityKind::Issue).expect("has an issue");
    assert!(matches!(issue.state, ActivityState::Open | ActivityState::Closed));

    let merged = items.iter().find(|i| i.state == ActivityState::Merged).expect("has a merged PR");
    assert_eq!(merged.kind, ActivityKind::PullRequest);
    assert!(merged.html_url.starts_with("https://"));
    assert!(!merged.author_login.is_empty());
}
```

Adjust the assertions to the actual fixture contents (exact numbers/titles from the snapshot are better than `find` where practical).

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p fido-server maps_issue_pr_and_merged_pr`
Expected: FAIL to compile — `GithubIssue`/`map_issue_to_activity` not found.

- [ ] **Step 4: Implement**

In `github.rs` (module-level, near the other response structs):

```rust
#[derive(Debug, Deserialize)]
pub(crate) struct GithubIssue {
    id: i64,
    number: i64,
    title: String,
    state: String,
    html_url: String,
    created_at: chrono::DateTime<Utc>,
    user: GithubIssueUser,
    pull_request: Option<GithubIssuePullRequest>,
}

#[derive(Debug, Deserialize)]
struct GithubIssueUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GithubIssuePullRequest {
    merged_at: Option<chrono::DateTime<Utc>>,
}

pub(crate) fn map_issue_to_activity(issue: GithubIssue) -> ActivityItem {
    let (kind, state) = match &issue.pull_request {
        Some(pr) if pr.merged_at.is_some() => (ActivityKind::PullRequest, ActivityState::Merged),
        Some(_) => (
            ActivityKind::PullRequest,
            if issue.state == "open" { ActivityState::Open } else { ActivityState::Closed },
        ),
        None => (
            ActivityKind::Issue,
            if issue.state == "open" { ActivityState::Open } else { ActivityState::Closed },
        ),
    };
    ActivityItem {
        github_id: issue.id,
        kind,
        number: issue.number,
        title: issue.title,
        author_login: issue.user.login,
        state,
        created_at: issue.created_at,
        html_url: issue.html_url,
    }
}
```

And the service method (imports: `fido_types::{ActivityItem, ActivityKind, ActivityState}`, `chrono::Duration`):

```rust
/// Fetch issues + PRs updated in the last 14 days (one request; GitHub's
/// issues endpoint includes PRs). Uses the caller's stored token when
/// present, unauthenticated otherwise.
pub async fn repo_activity(
    &self,
    user_id: Uuid,
    owner: &str,
    name: &str,
) -> Result<Vec<ActivityItem>> {
    let since = (Utc::now() - Duration::days(14)).to_rfc3339();
    let url = format!(
        "{}/repos/{}/{}/issues?state=all&since={}&per_page=100&sort=created&direction=desc",
        self.api_base, owner, name, since
    );

    let result = match self.load_token(user_id)? {
        Some(_) => self.get_with_token::<Vec<GithubIssue>>(user_id, url, "repo_activity").await,
        None => self.get_public::<Vec<GithubIssue>>(url, "repo_activity").await,
    }
    .map(|issues| issues.into_iter().map(map_issue_to_activity).collect());

    let op = format!("GET /repos/{}/{}/issues", owner, name);
    self.log_result("repo_activity", user_id, Some(&op), &result);
    result
}
```

Note: GitHub's `since` filters on `updated_at`, which is what we want ("recently active"), and `created_at` still drives display ordering client-side.

- [ ] **Step 5: Run tests**

Run: `cargo test -p fido-server maps_issue_pr_and_merged_pr` — PASS. `cargo clippy -p fido-server --all-targets -- -D warnings` — clean (dead-code on `repo_activity` is fine only if it errors; it shouldn't since it's `pub`).

- [ ] **Step 6: Commit**

```bash
git add fido-server/src/services/github.rs fido-server/tests/fixtures/github_issues_sample.json
git commit -m "feat(server): fetch repo issues/PRs as activity items from GitHub"
```

---

### Task 3: Activity cache repository + ActivityService

**Files:**
- Modify: `fido-server/src/db/schema.rs` (new table)
- Create: `fido-server/src/db/repositories/activity_repository.rs`
- Modify: `fido-server/src/db/repositories/mod.rs` (register + add to `Repositories`)
- Create: `fido-server/src/services/activity.rs`
- Modify: `fido-server/src/services/mod.rs`

**Interfaces:**
- Consumes: `GithubService::repo_activity` (Task 2), `fido_types::ActivityItem` (Task 1), `Repositories` bundle, `CommunityRepository` (existing `get_by_id`-style lookup — match its actual method name when wiring).
- Produces: `ActivityRepository { get(&self, community_id: Uuid) -> Result<Option<ActivityCacheRecord>>, upsert(&self, community_id: Uuid, payload: &str, fetched_at: DateTime<Utc>) -> Result<()> }`; `ActivityService::new(repos: Repositories, github: GithubService)`; `ActivityService::get_activity(&self, user_id: Uuid, community_id: Uuid) -> Result<CommunityActivity>` where `pub struct CommunityActivity { pub items: Vec<ActivityItem>, pub fetched_at: DateTime<Utc> }`; pub const `ACTIVITY_CACHE_TTL_MINUTES: i64 = 10`.

- [ ] **Step 1: Add the table to `schema.rs`** (after the `github_tokens` table):

```sql
CREATE TABLE IF NOT EXISTS community_activity (
    community_id TEXT PRIMARY KEY,
    payload TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    FOREIGN KEY (community_id) REFERENCES communities(id) ON DELETE CASCADE
);
```

Match the exact formatting/registration mechanism the other tables in `schema.rs` use (they are executed as one batch — add this block the same way).

- [ ] **Step 2: Write the failing repository test**

`activity_repository.rs`, modeled exactly on `github_token_repository.rs` (same `DbPool` pattern, same `#[cfg(all(test, feature = "sqlite-tests"))]`):

```rust
#[test]
fn upsert_and_get_activity_cache() {
    let db = setup_db(); // like token repo's setup, but insert a community row instead of a user
    let repo = ActivityRepository::new(db.pool.clone());
    let community_id = Uuid::parse_str("650e8400-e29b-41d4-a716-446655440001").unwrap();
    let now = Utc::now();

    assert!(repo.get(community_id).unwrap().is_none());

    repo.upsert(community_id, r#"[{"fake":"payload"}]"#, now).unwrap();
    let rec = repo.get(community_id).unwrap().unwrap();
    assert_eq!(rec.payload, r#"[{"fake":"payload"}]"#);
    assert_eq!(rec.fetched_at, now);

    let later = now + chrono::Duration::minutes(11);
    repo.upsert(community_id, "[]", later).unwrap();
    let rec = repo.get(community_id).unwrap().unwrap();
    assert_eq!(rec.payload, "[]");
    assert_eq!(rec.fetched_at, later);
}
```

The `setup_db` helper inserts a row into `communities` with id `650e8400-e29b-41d4-a716-446655440001` (copy the column list from `schema.rs`'s communities table; use dummy values, `claimed_by` NULL).

- [ ] **Step 3: Run to verify it fails, then implement the repository**

Run: `cargo test -p fido-server --features sqlite-tests upsert_and_get_activity_cache` — FAIL (module missing). Then:

```rust
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use crate::db::DbPool;

#[derive(Debug, Clone)]
pub struct ActivityCacheRecord {
    pub payload: String,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct ActivityRepository {
    pool: DbPool,
}

impl ActivityRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn get(&self, community_id: Uuid) -> Result<Option<ActivityCacheRecord>> {
        let conn = self.pool.get()?;
        let row: Option<(String, String)> = conn
            .query_row(
                "SELECT payload, fetched_at FROM community_activity WHERE community_id = ?1",
                rusqlite::params![community_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        match row {
            Some((payload, fetched_at)) => Ok(Some(ActivityCacheRecord {
                payload,
                fetched_at: DateTime::parse_from_rfc3339(&fetched_at)?.with_timezone(&Utc),
            })),
            None => Ok(None),
        }
    }

    pub fn upsert(&self, community_id: Uuid, payload: &str, fetched_at: DateTime<Utc>) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO community_activity (community_id, payload, fetched_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(community_id) DO UPDATE SET
                payload = excluded.payload,
                fetched_at = excluded.fetched_at",
            rusqlite::params![community_id.to_string(), payload, fetched_at.to_rfc3339()],
        )?;
        Ok(())
    }
}
```

Register in `repositories/mod.rs`: `mod activity_repository;`, `pub use activity_repository::{ActivityCacheRecord, ActivityRepository};`, field `pub activity: ActivityRepository` on `Repositories`, constructed with `ActivityRepository::new(pool.clone())` (keep the final non-cloned pool on the last field as-is).

Run the test — PASS.

- [ ] **Step 4: Write the failing service tests (pure TTL logic)**

`services/activity.rs` — the cache decision is a pure function so it tests without network or fake clocks in the service:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn cache_is_fresh_within_ttl_and_stale_after() {
        let now = Utc::now();
        assert!(cache_is_fresh(now - Duration::minutes(9), now));
        assert!(!cache_is_fresh(now - Duration::minutes(10), now));
        assert!(!cache_is_fresh(now - Duration::hours(2), now));
    }
}
```

- [ ] **Step 5: Implement the service**

```rust
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use fido_types::ActivityItem;
use uuid::Uuid;

use crate::db::repositories::Repositories;
use crate::services::github::GithubService;

pub const ACTIVITY_CACHE_TTL_MINUTES: i64 = 10;

#[derive(Debug, Clone)]
pub struct CommunityActivity {
    pub items: Vec<ActivityItem>,
    pub fetched_at: DateTime<Utc>,
}

pub(crate) fn cache_is_fresh(fetched_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now - fetched_at < Duration::minutes(ACTIVITY_CACHE_TTL_MINUTES)
}

pub struct ActivityService {
    repos: Repositories,
    github: GithubService,
}

impl ActivityService {
    pub fn new(repos: Repositories, github: GithubService) -> Self {
        Self { repos, github }
    }

    /// Cached repo activity for a community. Fresh cache: served as-is.
    /// Stale/missing: refetch from GitHub and upsert. GitHub failure with a
    /// stale cache present: serve stale with a warning. Failure with no
    /// cache: propagate.
    pub async fn get_activity(&self, user_id: Uuid, community_id: Uuid) -> Result<CommunityActivity> {
        let now = Utc::now();
        let cached = self.repos.activity.get(community_id)?;

        if let Some(record) = &cached {
            if cache_is_fresh(record.fetched_at, now) {
                return decode(record.payload.as_str(), record.fetched_at);
            }
        }

        let community = self
            .repos
            .communities
            .get_by_id(&community_id)? // match CommunityRepository's actual lookup signature
            .ok_or_else(|| anyhow!("Community {} not found", community_id))?;

        match self.github.repo_activity(user_id, &community.owner, &community.name).await {
            Ok(items) => {
                let payload = serde_json::to_string(&items)
                    .context("Failed to serialize activity payload")?;
                self.repos.activity.upsert(community_id, &payload, now)?;
                Ok(CommunityActivity { items, fetched_at: now })
            }
            Err(error) => {
                if let Some(record) = cached {
                    tracing::warn!(%community_id, %error, "Serving stale activity cache after GitHub fetch failure");
                    return decode(record.payload.as_str(), record.fetched_at);
                }
                Err(error).with_context(|| {
                    format!("Failed to fetch repo activity for community {}", community_id)
                })
            }
        }
    }
}

fn decode(payload: &str, fetched_at: DateTime<Utc>) -> Result<CommunityActivity> {
    let items: Vec<ActivityItem> =
        serde_json::from_str(payload).context("Failed to decode cached activity payload")?;
    Ok(CommunityActivity { items, fetched_at })
}
```

Check `CommunityRepository` for the real by-id lookup method name and adapt that one line. Register `pub mod activity;` in `services/mod.rs`.

- [ ] **Step 6: Run and commit**

Run: `cargo test -p fido-server --features sqlite-tests activity` and `cargo clippy -p fido-server --all-targets -- -D warnings`.
Expected: PASS / clean.

```bash
git add fido-server/src/db fido-server/src/services
git commit -m "feat(server): community activity cache with 10-minute TTL"
```

---

### Task 4: GET /communities/:id/activity endpoint

**Files:**
- Modify: `fido-server/src/api/communities.rs`
- Modify: `fido-server/src/lib.rs` (route)
- Modify: `fido-server/src/main.rs` (route — the router is duplicated there)

**Interfaces:**
- Consumes: `ActivityService::get_activity` (Task 3), `AuthenticatedUser`, `AppState` (has `repos` and `github_service`).
- Produces: `GET /communities/:id/activity` → `200 {"items": [ActivityItem...], "fetched_at": "<rfc3339>"}`; handler `pub async fn get_activity` in `api/communities.rs`.

- [ ] **Step 1: Write the failing test**

Follow the existing endpoint-test pattern in the server test suite (the `/communities/:id/members` tests from M1 are the closest model — find them with `grep -rn "communities.*members" fido-server` and copy their harness). Two cases:

```rust
// 1. Unauthenticated request -> 401
// 2. Authenticated request for a random Uuid community -> error status (404/500 per ApiError mapping
//    of "Community not found"), NOT a panic. No GitHub call happens because the community lookup fails first.
```

Write them concretely against the real harness; assert the status codes the existing members tests assert for the analogous cases.

- [ ] **Step 2: Run to verify failure, then implement the handler**

In `api/communities.rs`:

```rust
use fido_types::ActivityItem;
use crate::services::activity::ActivityService;

#[derive(Debug, Serialize)]
pub struct CommunityActivityResponse {
    pub items: Vec<ActivityItem>,
    pub fetched_at: String,
}

/// GET /communities/:id/activity - Recent GitHub issues/PRs for the community's repo
pub async fn get_activity(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(community_id): Path<Uuid>,
) -> ApiResult<Json<CommunityActivityResponse>> {
    let service = ActivityService::new(state.repos.clone(), state.github_service.clone());
    let activity = service.get_activity(user_id, community_id).await?;
    Ok(Json(CommunityActivityResponse {
        items: activity.items,
        fetched_at: activity.fetched_at.to_rfc3339(),
    }))
}
```

If `ActivityService` errors don't convert to `ApiResult` automatically, map them the same way `list_members` maps service errors (check how `?` works there — the `ApiError: From<anyhow::Error>` impl likely covers it).

Routes — in BOTH `lib.rs` and `main.rs`, next to the other `/communities/:id/...` routes:

```rust
.route(
    "/communities/:id/activity",
    get(api::communities::get_activity),
)
```

- [ ] **Step 3: Run tests and commit**

Run: `cargo test -p fido-server activity` and `cargo build --workspace`.
Expected: PASS, 0 errors.

```bash
git add fido-server/src
git commit -m "feat(server): GET /communities/:id/activity endpoint"
```

---

### Task 5: TUI activity state, client method, and feed interleave

**Files:**
- Modify: `fido-tui/src/api/client.rs`
- Modify: `fido-tui/src/app/state.rs` (PostsState)
- Create: `fido-tui/src/app/activity.rs` (load + rebuild logic)
- Modify: `fido-tui/src/app/mod.rs` (register module)
- Modify: `fido-tui/src/app/posts.rs` and any other consumer of `list_index_to_post_index` / `list_state.selected()` on the posts list (audit step below)

**Interfaces:**
- Consumes: server endpoint from Task 4; `fido_types::{ActivityItem, ActivityKind, ActivityState}`.
- Produces:
  - `ApiClient::get_community_activity(&self, community_id: Uuid) -> ApiResult<CommunityActivityResponse>` with `pub struct CommunityActivityResponse { pub items: Vec<ActivityItem>, pub fetched_at: String }` (typed, matching the M1 `ConversationInfo` pattern).
  - On `PostsState`: `pub activity_items: Vec<ActivityItem>`, `pub activity_loading: bool`, `pub activity_error: Option<String>`, `pub activity_pending_load: bool`, `pub feed_entries: Vec<FeedEntry>`.
  - `pub enum FeedEntry { Post(usize), Activity(usize) }` in `state.rs`.
  - `pub fn rebuild_feed_entries(posts: &[Post], activity: &[ActivityItem]) -> Vec<FeedEntry>` — pure, in `state.rs` near `PostsState`; merged descending by `created_at`, ties: post first.
  - On `PostsState`: `pub fn selected_feed_entry(&self) -> Option<FeedEntry>` — maps `list_state.selected()` through `items_before_posts()` offset into `feed_entries`.
  - On `App` (in `app/activity.rs`): `pub async fn load_activity(&mut self)` and `pub fn clear_activity(&mut self)`.

- [ ] **Step 1: Write the failing tests** (in `fido-tui/src/app/tests.rs` or a tests module in `state.rs`, matching where M1 put pure-logic tests):

```rust
#[test]
fn feed_entries_interleave_by_created_at_desc() {
    let posts = vec![test_post_created_at("2026-07-02T12:00:00Z"), test_post_created_at("2026-06-30T12:00:00Z")];
    let activity = vec![test_activity_created_at("2026-07-01T12:00:00Z")];
    let entries = rebuild_feed_entries(&posts, &activity);
    assert_eq!(entries, vec![FeedEntry::Post(0), FeedEntry::Activity(0), FeedEntry::Post(1)]);
}

#[test]
fn selected_feed_entry_accounts_for_loading_offset() {
    let mut state = test_posts_state_with(/* 1 post, 1 activity item, loading: true */);
    state.rebuild_feed();  // helper that sets state.feed_entries = rebuild_feed_entries(&state.posts, &state.activity_items)
    state.list_state.select(Some(1)); // 0 is the loading spinner row
    assert_eq!(state.selected_feed_entry(), Some(FeedEntry::Post(0)));
}
```

Write the small `test_post_created_at` / `test_activity_created_at` builders with fixed uuids/strings — complete literal structs, no helpers that hide fields. `FeedEntry` needs `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`.

- [ ] **Step 2: Run to verify failure, implement state + pure logic**

`state.rs` additions:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedEntry {
    Post(usize),
    Activity(usize),
}

pub fn rebuild_feed_entries(posts: &[Post], activity: &[ActivityItem]) -> Vec<FeedEntry> {
    let mut entries: Vec<(chrono::DateTime<chrono::Utc>, FeedEntry)> = posts
        .iter()
        .enumerate()
        .map(|(i, p)| (p.created_at, FeedEntry::Post(i)))
        .chain(
            activity
                .iter()
                .enumerate()
                .map(|(i, a)| (a.created_at, FeedEntry::Activity(i))),
        )
        .collect();
    // Descending by time; ties render the post first (stable sort preserves chain order).
    entries.sort_by(|a, b| b.0.cmp(&a.0));
    entries.into_iter().map(|(_, e)| e).collect()
}
```

`PostsState`: add the five fields (initialize in every `PostsState` constructor — grep for where it's built), plus:

```rust
pub fn rebuild_feed(&mut self) {
    self.feed_entries = rebuild_feed_entries(&self.posts, &self.activity_items);
}

pub fn selected_feed_entry(&self) -> Option<FeedEntry> {
    let list_idx = self.list_state.selected()?;
    let offset = self.items_before_posts();
    self.feed_entries.get(list_idx.checked_sub(offset)?).copied()
}
```

Update `items_before_posts()`: the loading-activity row (added in Task 6) counts too — add `if self.activity_loading { count += 1; }`. Keep `list_index_to_post_index` but reimplement it on top of `selected_feed_entry` semantics:

```rust
pub fn list_index_to_post_index(&self, list_index: usize) -> Option<usize> {
    let offset = self.items_before_posts();
    match self.feed_entries.get(list_index.checked_sub(offset)?)? {
        FeedEntry::Post(i) => Some(*i),
        FeedEntry::Activity(_) => None,
    }
}
```

and `post_index_to_list_index(post_index)` must map through `feed_entries` (position of `FeedEntry::Post(post_index)` plus offset).

- [ ] **Step 3: Consumer audit (state-mutation audit)**

Run: `grep -n "list_index_to_post_index\|post_index_to_list_index\|list_state.selected()" fido-tui/src -r`
Every posts-list consumer must go through the (now activity-aware) helpers. Known offender: `app/posts.rs::vote_on_selected_post` indexes `posts[selected_index]` directly with the raw list index — fix it to use `list_index_to_post_index` and no-op (return Ok) when the selection is not a post. Fix every other direct-index consumer the grep finds the same way. Navigation (up/down) operates on list indices and needs no change, but clamping/wrap logic that uses `posts.len()` as the item count must use `items_before_posts() + feed_entries.len()` instead — grep for `posts.len()` in navigation/selection code and fix each.

- [ ] **Step 4: Client method + load/clear**

`api/client.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct CommunityActivityResponse {
    pub items: Vec<ActivityItem>,
    pub fetched_at: String,
}

/// Recent GitHub issues/PRs for a community's repo
pub async fn get_community_activity(
    &self,
    community_id: Uuid,
) -> ApiResult<CommunityActivityResponse> {
    let url = self.build_url(&format!("/communities/{}/activity", community_id));
    let req = self.add_auth_header(self.client.get(&url));
    let response = req.send().await?;
    self.handle_response(response).await
}
```

`app/activity.rs`:

```rust
use anyhow::Result;

use crate::app::App;

impl App {
    /// Fetch repo activity for the current community. Called by the event
    /// loop after the board has rendered (activity_pending_load pattern) —
    /// never on the render path.
    pub async fn load_activity(&mut self) {
        let Some(community_id) = self.current_community_id() else {
            self.posts_state.activity_loading = false;
            return;
        };
        match self.api_client.get_community_activity(community_id).await {
            Ok(response) => {
                self.posts_state.activity_items = response.items;
                self.posts_state.activity_error = None;
            }
            Err(e) => {
                self.posts_state.activity_items.clear();
                self.posts_state.activity_error = Some(format!("repo activity unavailable: {}", e));
            }
        }
        self.posts_state.activity_loading = false;
        self.posts_state.rebuild_feed();
    }

    pub fn clear_activity(&mut self) {
        self.posts_state.activity_items.clear();
        self.posts_state.activity_error = None;
        self.posts_state.activity_loading = false;
        self.posts_state.activity_pending_load = false;
        self.posts_state.rebuild_feed();
    }
}
```

`current_community_id()` — use whatever accessor the App already has for the active community (grep `community_id` in `app/communities.rs`; if there's no accessor, add one there returning `Option<Uuid>`). Wire the triggers: wherever posts get (re)loaded for a community (grep `load_posts` call sites and the `pending_load` handling in `event_loop.rs`), set `activity_pending_load = true; activity_loading = true;` alongside, and on community switch/leave call `clear_activity()`. In the event loop where `pending_load` is consumed, add the same consumption for `activity_pending_load` → `app.load_activity().await` — activity load runs AFTER the posts load so the board is already populated. Every `load_posts` success path must also call `rebuild_feed()` (posts changed under the entries).

- [ ] **Step 5: Run tests and commit**

Run: `cargo test -p fido-tui` and `cargo clippy -p fido-tui --all-targets -- -D warnings`.
Expected: PASS / clean (rendering not wired yet — that's Task 6; the loading row is invisible until then, which is fine).

```bash
git add fido-tui/src
git commit -m "feat(tui): activity state, client, and feed interleave"
```

---

### Task 6: TUI rendering, `o` key, and footer honesty

**Files:**
- Modify: `fido-tui/src/ui/tabs.rs` (`render_posts_tab_with_data` + posts action bar/footer)
- Modify: `fido-tui/src/event_loop.rs` (async `o` arm + guards)
- Modify: `fido-tui/src/app/activity.rs` (open-in-browser helper)

**Interfaces:**
- Consumes: `FeedEntry`, `selected_feed_entry()`, `activity_items`, `activity_loading`, `activity_error` (Task 5); existing `webbrowser` dependency (see `auth.rs:125` for the pattern).
- Produces: activity rows in the feed; `App::open_selected_activity(&mut self)`.

- [ ] **Step 1: Write the failing tests**

Pure formatting test (put the line-builder in `tabs.rs` or `ui/formatting.rs` as a pure fn so it's testable):

```rust
#[test]
fn activity_line_formats_issue_and_merged_pr() {
    let issue = /* ActivityItem literal: kind Issue, state Open, number 7, title "Fix login", author "alice" */;
    let line = activity_line_text(&issue);
    assert_eq!(line, "⊙ #7 Fix login · issue opened by alice");

    let pr = /* ActivityItem literal: kind PullRequest, state Merged, number 9, title "Dark mode", author "bob" */;
    assert_eq!(activity_line_text(&pr), "⇄ #9 Dark mode · merged · bob");
}
```

Behavior test for `o` (in `app/tests.rs`, following how M1 tested `message_user_from_profile` without a network):

```rust
#[test]
fn open_selected_activity_noops_when_selection_is_a_post() {
    // App with 1 post, 1 activity item, selection on the post row.
    // open_selected_activity() must return None (no URL to open).
}
```

Structure `open_selected_activity` to return the `Option<String>` URL and let the caller do the browser side-effect, so the test asserts the URL choice, not the browser launch.

- [ ] **Step 2: Implement rendering**

In `render_posts_tab_with_data`, replace the posts-only item construction: iterate `app.posts_state.feed_entries` instead of `posts.iter().enumerate()`. `FeedEntry::Post(i)` renders exactly the existing multi-line post item (same code, indexed by `i`); `FeedEntry::Activity(i)` renders one dimmed line:

- Exact text: `activity_line_text(item)` where:
  - Issue: `⊙ #{number} {title} · issue opened by {author_login}` (state shown by glyph color, `closed` appends ` · closed`)
  - PR: `⇄ #{number} {title} · {merged|open|closed} · {author_login}`
- Glyph color: open → `theme.success`, closed → `theme.error`, merged → magenta (`Color::Magenta` if the theme has no magenta slot).
- Rest of line: `theme.text_dim`. Selected activity row: same `▶ ` prefix convention as posts, glyph keeps its color, text goes from dim to normal.
- Truncate title so the line fits `available_width` (reuse the width the post renderer computes).

Loading row: after the existing `⟳ Loading...` block, add:

```rust
if app.posts_state.activity_loading {
    let style = Style::default().fg(theme.text_dim);
    items.push(ListItem::new(create_centered_indicator("⊙ loading repo activity...", style, available_width)));
}
```

(This is the row `items_before_posts()` already counts from Task 5.) `activity_error`: render one dim line `⊙ {error}` at the top of the feed items (not a banner — ambient, like the content it replaces). The "No posts yet" empty state must NOT trigger when `feed_entries` is non-empty (activity alone should render); change the emptiness check from `posts.is_empty()` to `feed_entries.is_empty()`.

Footer: find the posts action bar (grep `action_bar` / the footer text for the posts tab in `tabs.rs`). Gate on `selected_feed_entry()`: activity selected → show `o: Open on GitHub` and navigation keys only (no vote/reply/p hints); post selected → existing hints, no `o`.

- [ ] **Step 3: Implement the `o` key + guards**

`app/activity.rs`:

```rust
/// URL of the selected activity item, if the selection is an activity row.
pub fn selected_activity_url(&self) -> Option<String> {
    match self.posts_state.selected_feed_entry()? {
        FeedEntry::Activity(i) => self.posts_state.activity_items.get(i).map(|a| a.html_url.clone()),
        FeedEntry::Post(_) => None,
    }
}
```

`event_loop.rs`, in `handle_async_key_events`, a guarded arm BEFORE the final `_ =>` arm (model the guard style on the M1 arms; only in Navigation mode, posts tab, no modal open):

```rust
KeyCode::Char('o')
    if app.current_tab == Tab::Posts
        && app.input_mode == InputMode::Navigation
        && app.user_profile_view.is_none()
        && app.selected_activity_url().is_some() =>
{
    if let Some(url) = app.selected_activity_url() {
        if let Err(e) = webbrowser::open(&url) {
            app.posts_state.error = Some(format!("Could not open browser: {}", e));
        }
    }
}
```

Match the actual condition idioms used by neighboring arms (tab/screen/modal checks) — copy their guards, don't invent new ones. Then the no-op guards: find the posts-tab arms for Enter/Space (open post), `u`/`d` (vote), `p` (profile), reply — each must no-op when `selected_feed_entry()` is `Some(FeedEntry::Activity(_))`. Where those arms already resolve the selection through `list_index_to_post_index` (which now returns None for activity rows — Task 5), verify the None path is a clean no-op rather than an error message; fix any that surface errors.

- [ ] **Step 4: Run tests**

Run: `cargo test -p fido-tui` and `cargo clippy --workspace --all-targets -- -D warnings`.
Expected: PASS / clean.

- [ ] **Step 5: Commit**

```bash
git add fido-tui/src
git commit -m "feat(tui): render repo activity in feed with o-to-open"
```

---

### Task 7: Community badge SVG endpoint

**Files:**
- Create: `fido-server/src/api/badge.rs`
- Modify: `fido-server/src/api/mod.rs`
- Modify: `fido-server/src/lib.rs` (route)
- Modify: `fido-server/src/main.rs` (route)

**Interfaces:**
- Consumes: `CommunityRepository` lookup by owner/name (find the existing method — community join resolves owner/name, so a `get_by_owner_name`-shaped method exists or is trivial to add), `MembershipRepository` member count (the `member_count` in `CommunityViewResponse` proves a count method exists).
- Produces: `GET /badge/:owner/:repo.svg` — no auth. `pub fn render_badge_svg(member_count: i64) -> String` (pure).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn badge_svg_contains_count_and_label() {
    let svg = render_badge_svg(14);
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("fido"));
    assert!(svg.contains("14 members"));
}

#[test]
fn badge_svg_singular_member() {
    assert!(render_badge_svg(1).contains("1 member"));
    assert!(!render_badge_svg(1).contains("1 members"));
}
```

Plus endpoint tests in the existing harness style: unknown repo → 404 with `text/plain` body; known community → 200, `content-type: image/svg+xml`, `cache-control: public, max-age=300`, body contains the seeded member count. Route the request WITHOUT an auth header to prove it's public.

- [ ] **Step 2: Implement**

`api/badge.rs`:

```rust
use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};

use crate::state::AppState;

const LABEL: &str = "fido";
const CHAR_WIDTH: usize = 7; // approximate Verdana 11px advance, shields-style
const PAD: usize = 10;

/// Render a shields-flat-style badge: `fido | N members`. Pure — no I/O.
pub fn render_badge_svg(member_count: i64) -> String {
    let value = if member_count == 1 {
        "1 member".to_string()
    } else {
        format!("{} members", member_count)
    };
    let left_w = LABEL.len() * CHAR_WIDTH + PAD;
    let right_w = value.len() * CHAR_WIDTH + PAD;
    let total = left_w + right_w;
    format!(
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{total}" height="20" role="img" aria-label="{label}: {value}">"#,
            r#"<rect width="{left_w}" height="20" fill="#555"/>"#,
            r#"<rect x="{left_w}" width="{right_w}" height="20" fill="#4c1"/>"#,
            r#"<g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" font-size="11">"#,
            r#"<text x="{lx}" y="14">{label}</text>"#,
            r#"<text x="{rx}" y="14">{value}</text>"#,
            r#"</g></svg>"#
        ),
        total = total,
        left_w = left_w,
        right_w = right_w,
        lx = left_w / 2,
        rx = left_w + right_w / 2,
        label = LABEL,
        value = value,
    )
}

/// GET /badge/:owner/:repo.svg — public, no auth.
pub async fn community_badge(
    State(state): State<AppState>,
    Path((owner, repo_svg)): Path<(String, String)>,
) -> Response {
    let Some(repo) = repo_svg.strip_suffix(".svg") else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };

    let community = match state.repos.communities.get_by_owner_name(&owner, repo) {
        Ok(Some(c)) => c,
        Ok(None) => return (StatusCode::NOT_FOUND, "no fido community for this repo").into_response(),
        Err(error) => {
            tracing::warn!(%owner, repo, %error, "Badge community lookup failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, "lookup failed").into_response();
        }
    };

    let count = match state.repos.memberships.count_for_community(&community.id) {
        Ok(count) => count,
        Err(error) => {
            tracing::warn!(community_id = %community.id, %error, "Badge member count failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, "count failed").into_response();
        }
    };

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "public, max-age=300"),
        ],
        render_badge_svg(count),
    )
        .into_response()
}
```

Verify the exact `CommunityRepository`/`MembershipRepository` method names with grep and adapt (`get_by_owner_name`, `count_for_community` are the shapes, not guaranteed names; if a method is missing, add it to the repository following that file's existing style, with SQL matching the members-count query the community view already uses). `raw` string escaping note: `#` inside `r#"..."#` fill colors is fine because the delimiter is `r#" "#` — if the compiler complains, switch those lines to plain strings with `\"` escapes.

Route in BOTH `lib.rs` and `main.rs`, placed with `/health` (before the authenticated section, no auth middleware applies anyway since auth is per-extractor here — placement with `/health` documents intent):

```rust
.route("/badge/:owner/:repo_svg", get(api::badge::community_badge))
```

Register `pub mod badge;` in `api/mod.rs`.

- [ ] **Step 3: Run tests and commit**

Run: `cargo test -p fido-server badge` and `cargo clippy -p fido-server --all-targets -- -D warnings`.
Expected: PASS / clean.

```bash
git add fido-server/src
git commit -m "feat(server): public community badge SVG endpoint"
```

---

### Task 8: e2e scenario, README badge docs, CHANGELOG

**Files:**
- Modify: `scripts/e2e_tui.sh`
- Modify: `README.md`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: everything above; the existing GitHub stub inside `scripts/e2e_tui.sh` (the harness already stubs GitHub for auth — extend the same stub server).

- [ ] **Step 1: Extend the GitHub stub**

In the stub server inside `scripts/e2e_tui.sh`, add a handler for `GET /repos/<owner>/<repo>/issues` returning a JSON array with one open issue (real GitHub shape — copy a reduced element from the Task 2 fixture):

```json
[{"id": 1, "number": 7, "title": "Stub issue for e2e", "state": "open",
  "html_url": "https://github.com/stub/repo/issues/7",
  "created_at": "2026-07-03T00:00:00Z", "user": {"login": "stubuser"}}]
```

- [ ] **Step 2: Add Scenario 5**

Following the structure of Scenario 4 (search → profile → message): open the board for the seeded community, `wait_for` the text `Stub issue for e2e` in the pane (proves fetch + interleave + render), then assert via sqlite that `community_activity` has a row for the community (proves the cache wrote):

```bash
count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM community_activity;")
[ "$count" -ge 1 ] || fail "expected community_activity cache row"
```

Use `wait_for`, never bare `sleep` (M1 rule).

- [ ] **Step 3: Run the harness**

Run: `just e2e-tui`
Expected: all scenarios pass including the new one.

- [ ] **Step 4: README + CHANGELOG**

README: add a `## Badge` section:

```markdown
## Badge

Show your repo's fido community in the README:

​```markdown
[![fido community](https://fido-web-production.up.railway.app/badge/OWNER/REPO.svg)](https://github.com/OWNER/REPO)
​```

The badge is live — it always shows the current member count.
```

CHANGELOG `[Unreleased]` → Added:

```markdown
- Repo activity: GitHub issues and PRs from the last 14 days appear in the community feed. Select one and press `o` to open it on GitHub.
- Community badge: `GET /badge/:owner/:repo.svg` renders a live member-count badge for embedding in a README.
```

- [ ] **Step 5: Commit**

```bash
git add scripts/e2e_tui.sh README.md CHANGELOG.md
git commit -m "test(e2e): activity feed scenario; docs: badge snippet + changelog"
```
