# GitHub Activity as Interactive Posts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace 0.5.0's read-only `ActivityItem`/`FeedEntry` overlay with real `Post` rows for synced GitHub issues/PRs, so votes and replies (comments) work on them for free — no writes back to GitHub, ever.

**Architecture:** `posts` gains four nullable columns (`github_id`, `github_kind`, `github_state`, `github_html_url`). A synthetic `github` system user authors every synced post. `ActivityService::sync_activity` (rewritten) fetches via the existing `GithubService::repo_activity`, upserts posts keyed on `(community_id, github_id)`, and is called from `PostService::get_posts` before it queries — errors are logged and swallowed, never break the read. The TUI's `FeedEntry`/interleave/separate-fetch layer is deleted entirely; the posts list is the only list again, with GitHub posts rendered inline via their `github_kind`/`github_state` fields.

**Tech Stack:** Rust workspace (fido-types / fido-server / fido-tui), rusqlite, Axum, ratatui.

**Spec:** `docs/superpowers/specs/2026-07-04-github-posts-design.md`

## Global Constraints

- Never write to GitHub (no comment/reaction API calls) — fido-native only.
- `content` keeps its 280-char CHECK constraint; a GitHub post's `content` is its title, truncated to 280 chars.
- The synthetic `github` user is created idempotently (get-or-create by username) and its membership in a community is granted idempotently (`MembershipRepository::insert_if_missing`, already idempotent — do not use the non-idempotent `insert`).
- Sync failure must never break serving existing posts: `sync_activity` returns `anyhow::Result<()>`; `PostService::get_posts` logs and discards its `Err`, never propagates.
- `o` (open on GitHub) is shown/active only when the selected post has `github_kind.is_some()`, and **coexists** with vote/reply hints in the footer (not an exclusive branch, unlike 0.5.0).
- `p` (view profile) is disabled — no-op and hidden from the footer — when the selected post's `github_kind.is_some()` (the synthetic `github` user has no real profile).
- A key shown in a footer must work on the current selection (footer-honesty rule).
- New async key arms/guards in `event_loop.rs::handle_async_key_events` go before the final `_ =>` arm, matching neighboring guard idioms exactly.
- `cargo clippy --workspace --all-targets -- -D warnings` must stay clean throughout.
- Still one GitHub API call per sync (14-day window, ≤100 items via `GithubService::repo_activity`, unchanged).

---

### Task 1: Schema — posts gains GitHub columns, plus the sync-gate table

**Files:**
- Modify: `fido-server/src/db/schema.rs`

**Interfaces:**
- Produces: `posts.github_id INTEGER`, `posts.github_kind TEXT`, `posts.github_state TEXT`, `posts.github_html_url TEXT` (all nullable); `UNIQUE(community_id, github_id)` index for non-null `github_id`; new table `community_activity_sync(community_id TEXT PRIMARY KEY, last_synced_at TEXT NOT NULL)`.

- [ ] **Step 1: Add the columns and index**

In `fido-server/src/db/schema.rs`, find the existing `posts` table block (around line 51-66) and add the four columns before the closing paren, then add a unique index after it:

```sql
CREATE TABLE IF NOT EXISTS posts (
    id TEXT PRIMARY KEY,
    author_id TEXT NOT NULL,
    community_id TEXT NOT NULL,
    content TEXT NOT NULL CHECK(length(content) <= 280),
    created_at TEXT NOT NULL,
    upvotes INTEGER NOT NULL DEFAULT 0,
    downvotes INTEGER NOT NULL DEFAULT 0,
    approved INTEGER NOT NULL DEFAULT 1,
    parent_post_id TEXT,
    reply_to_user_id TEXT,
    github_id INTEGER,
    github_kind TEXT,
    github_state TEXT,
    github_html_url TEXT,
    FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (community_id) REFERENCES communities(id),
    FOREIGN KEY (parent_post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (reply_to_user_id) REFERENCES users(id) ON DELETE SET NULL
);

-- One synced post per GitHub issue/PR per community; NULL github_id (normal
-- user posts) is exempt from uniqueness by SQLite's NULL-distinct semantics.
CREATE UNIQUE INDEX IF NOT EXISTS idx_posts_community_github_id ON posts(community_id, github_id);
```

Add the sync-gate table near the (now-unused, left in place per spec) `community_activity` table:

```sql
-- Gates how often a community's GitHub issues/PRs are re-fetched and
-- upserted into posts. No payload — posts themselves are the cache.
CREATE TABLE IF NOT EXISTS community_activity_sync (
    community_id TEXT PRIMARY KEY,
    last_synced_at TEXT NOT NULL,
    FOREIGN KEY (community_id) REFERENCES communities(id) ON DELETE CASCADE
);
```

- [ ] **Step 2: Verify schema initializes cleanly**

Run: `cargo test -p fido-server --features sqlite-tests` (any test that calls `Database::in_memory().initialize()` exercises the new DDL).
Expected: PASS, no SQL errors.

- [ ] **Step 3: Commit**

```bash
git add fido-server/src/db/schema.rs
git commit -m "feat(server): posts gain GitHub fields; add activity sync-gate table"
```

---

### Task 2: fido_types::Post gains GitHub fields

**Files:**
- Modify: `fido-types/src/models.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `Post.github_id: Option<i64>`, `Post.github_kind: Option<ActivityKind>`, `Post.github_state: Option<ActivityState>`, `Post.github_html_url: Option<String>` — all `#[serde(default)]`. `ActivityKind`/`ActivityState` (in `fido-types/src/enums.rs`, shipped 0.5.0) are reused unchanged.

- [ ] **Step 1: Write the failing test**

Append to `fido-types/src/models.rs` (existing test module, or a new one):

```rust
#[cfg(test)]
mod github_post_tests {
    use super::*;
    use crate::enums::{ActivityKind, ActivityState};

    #[test]
    fn post_github_fields_default_to_none_and_round_trip() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "author_id": "550e8400-e29b-41d4-a716-446655440001",
            "author_username": "alice",
            "community_id": "550e8400-e29b-41d4-a716-446655440002",
            "content": "hello",
            "created_at": "2026-07-04T00:00:00Z",
            "upvotes": 0,
            "downvotes": 0,
            "approved": true,
            "hashtags": []
        }"#;
        let post: Post = serde_json::from_str(json).expect("deserialize");
        assert_eq!(post.github_id, None);
        assert_eq!(post.github_kind, None);

        let mut github_post = post;
        github_post.github_id = Some(19);
        github_post.github_kind = Some(ActivityKind::PullRequest);
        github_post.github_state = Some(ActivityState::Merged);
        github_post.github_html_url = Some("https://github.com/o/r/pull/19".to_string());

        let round = serde_json::to_string(&github_post).unwrap();
        let back: Post = serde_json::from_str(&round).unwrap();
        assert_eq!(back.github_id, Some(19));
        assert_eq!(back.github_state, Some(ActivityState::Merged));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p fido-types post_github_fields_default_to_none_and_round_trip`
Expected: FAIL to compile — fields don't exist.

- [ ] **Step 3: Implement**

In `fido-types/src/models.rs`, update the `use crate::enums::{...}` import at the top to include `ActivityKind, ActivityState`, and add fields to `Post` (after `reply_to_username`):

```rust
    /// Username being replied to (for display purposes)
    #[serde(default)]
    pub reply_to_username: Option<String>,
    /// GitHub issue/PR id this post syncs from. None for a normal user post.
    #[serde(default)]
    pub github_id: Option<i64>,
    #[serde(default)]
    pub github_kind: Option<ActivityKind>,
    #[serde(default)]
    pub github_state: Option<ActivityState>,
    /// Link opened by the `o` key. None for a normal user post.
    #[serde(default)]
    pub github_html_url: Option<String>,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p fido-types post_github_fields_default_to_none_and_round_trip` — PASS.
Then `cargo build --workspace` — this will show every `Post { ... }` struct literal across the workspace that now needs the four new fields (Rust requires exhaustive struct literals unless `..Default::default()` is used). Fix each one by adding `github_id: None, github_kind: None, github_state: None, github_html_url: None,` (grep `Post {` across `fido-server/src` and `fido-tui/src` to find every literal construction site, including test fixtures).

- [ ] **Step 5: Commit**

```bash
git add fido-types/src/models.rs
git commit -m "feat(types): Post gains nullable GitHub sync fields"
```

---

### Task 3: PostRepository — GitHub-post columns in every read path + upsert

**Files:**
- Modify: `fido-server/src/db/repositories/post_repository.rs`

**Interfaces:**
- Consumes: `Post`'s new fields (Task 2).
- Produces: `PostRepository::upsert_github_post(&self, post: &Post) -> Result<()>`; `get_posts`, `get_pending_posts`, `get_by_user`, `get_by_id`, `get_replies`, `get_posts_by_username` all populate the four new fields on every returned `Post` (positions 13-16 after the existing 13-column SELECT list, 0-indexed 13-16).

- [ ] **Step 1: Write the failing tests**

Add to the existing `#[cfg(all(test, feature = "sqlite-tests"))] mod tests` in `post_repository.rs` (mirror the existing test setup helpers in this file for user/community fixtures):

```rust
#[test]
fn upsert_github_post_inserts_then_updates_by_community_and_github_id() {
    let db = setup_db(); // existing helper in this file
    let repo = PostRepository::new(db.pool.clone());
    let (author_id, community_id) = setup_user_and_community(&db); // existing helper

    let post_id = Uuid::new_v4();
    let mut post = Post {
        id: post_id,
        author_id,
        author_username: "github".to_string(),
        community_id,
        content: "Fix login bug".to_string(),
        created_at: Utc::now(),
        upvotes: 0,
        downvotes: 0,
        approved: true,
        hashtags: Vec::new(),
        user_vote: None,
        parent_post_id: None,
        reply_count: 0,
        reply_to_user_id: None,
        reply_to_username: None,
        github_id: Some(42),
        github_kind: Some(ActivityKind::Issue),
        github_state: Some(ActivityState::Open),
        github_html_url: Some("https://github.com/o/r/issues/42".to_string()),
    };
    repo.upsert_github_post(&post).expect("insert");

    let fetched = repo.get_by_id(&post_id).expect("query").expect("exists");
    assert_eq!(fetched.github_id, Some(42));
    assert_eq!(fetched.github_state, Some(ActivityState::Open));

    // Re-sync: same community_id + github_id, different id/content/state ->
    // must update the SAME row (same primary id), not insert a duplicate.
    post.content = "Fix login bug (updated)".to_string();
    post.github_state = Some(ActivityState::Closed);
    repo.upsert_github_post(&post).expect("update");

    let updated = repo.get_by_id(&post_id).expect("query").expect("exists");
    assert_eq!(updated.content, "Fix login bug (updated)");
    assert_eq!(updated.github_state, Some(ActivityState::Closed));

    let count: i64 = db
        .pool
        .get()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM posts WHERE community_id = ?1 AND github_id = 42",
            rusqlite::params![community_id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "must not duplicate on re-sync");
}
```

If `setup_user_and_community` doesn't already exist in this file's test module, write it following the file's existing test-fixture style (insert a user row and a community row directly via SQL, matching how other tests in this file seed data — check the file for the closest existing helper before adding a new one).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p fido-server --features sqlite-tests upsert_github_post_inserts_then_updates_by_community_and_github_id`
Expected: FAIL — `upsert_github_post` not found.

- [ ] **Step 3: Implement the upsert**

In `post_repository.rs`, add near `create`:

```rust
use fido_types::{ActivityKind, ActivityState, Post, SortOrder};
```

(update the existing `use fido_types::{Post, SortOrder};` line to include the two enums)

```rust
/// Insert a GitHub-synced post, or update it in place if this community
/// already has a post for that github_id (re-sync picks up state/content
/// changes on the same row — votes and replies are preserved).
pub fn upsert_github_post(&self, post: &Post) -> Result<()> {
    let conn = self.pool.get()?;
    conn.execute(
        "INSERT INTO posts (id, author_id, community_id, content, created_at, upvotes, downvotes, approved, parent_post_id, reply_to_user_id, github_id, github_kind, github_state, github_html_url)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, ?9, ?10, ?11, ?12)
         ON CONFLICT(community_id, github_id) DO UPDATE SET
            content = excluded.content,
            github_state = excluded.github_state,
            github_html_url = excluded.github_html_url",
        (
            post.id.to_string(),
            post.author_id.to_string(),
            post.community_id.to_string(),
            &post.content,
            post.created_at.to_rfc3339(),
            post.upvotes,
            post.downvotes,
            if post.approved { 1 } else { 0 },
            post.github_id,
            post.github_kind.map(|k| activity_kind_to_str(k)),
            post.github_state.map(|s| activity_state_to_str(s)),
            &post.github_html_url,
        ),
    ).context("Failed to upsert GitHub post")?;
    Ok(())
}
```

Add two small string-mapping helpers near `map_post_row` (mirroring the `ActivityKind`/`ActivityState` snake_case serde names already used elsewhere — `issue`/`pull_request`, `open`/`closed`/`merged`):

```rust
fn activity_kind_to_str(kind: ActivityKind) -> &'static str {
    match kind {
        ActivityKind::Issue => "issue",
        ActivityKind::PullRequest => "pull_request",
    }
}

fn activity_state_to_str(state: ActivityState) -> &'static str {
    match state {
        ActivityState::Open => "open",
        ActivityState::Closed => "closed",
        ActivityState::Merged => "merged",
    }
}

fn str_to_activity_kind(s: &str) -> Option<ActivityKind> {
    match s {
        "issue" => Some(ActivityKind::Issue),
        "pull_request" => Some(ActivityKind::PullRequest),
        _ => None,
    }
}

fn str_to_activity_state(s: &str) -> Option<ActivityState> {
    match s {
        "open" => Some(ActivityState::Open),
        "closed" => Some(ActivityState::Closed),
        "merged" => Some(ActivityState::Merged),
        _ => None,
    }
}
```

- [ ] **Step 4: Add the four columns to every SELECT + map_post_row**

Update the SELECT list in `get_posts`, `get_pending_posts`, `get_by_user`, `get_by_id`, `get_replies` (both the base-case and recursive-case sub-selects, plus the outer select), and `get_posts_by_username` — append `, p.github_id, p.github_kind, p.github_state, p.github_html_url` right after `p.approved` (or `rt.approved` in `get_replies`'s outer select — for `get_replies`' CTE, add the four raw `p.github_id, p.github_kind, p.github_state, p.github_html_url` columns to both the base-case and recursive-case inner SELECTs too, so they carry through the CTE, and reference them as `rt.github_id` etc. in the outer SELECT).

Update `map_post_row` to read the four new positional columns (13, 14, 15, 16 — right after `approved` at 12) and populate `Post`'s new fields:

```rust
fn map_post_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Post> {
    let id_str: String = row.get(0)?;
    let author_id_str: String = row.get(1)?;
    let created_at_str: String = row.get(4)?;
    let parent_post_id_str: Option<String> = row.get(7)?;
    let reply_to_user_id_str: Option<String> = row.get(9)?;
    let community_id_str: String = row.get(11)?;
    let github_kind_str: Option<String> = row.get(14)?;
    let github_state_str: Option<String> = row.get(15)?;

    Ok(Post {
        id: row::parse_uuid(&id_str, 0)?,
        author_id: row::parse_uuid(&author_id_str, 1)?,
        author_username: row.get(2)?,
        community_id: row::parse_uuid(&community_id_str, 11)?,
        content: row.get(3)?,
        created_at: row::parse_datetime(&created_at_str, 4)?,
        upvotes: row.get(5)?,
        downvotes: row.get(6)?,
        approved: row.get::<_, i32>(12)? == 1,
        hashtags: Vec::new(),
        user_vote: None,
        parent_post_id: row::parse_optional_uuid(parent_post_id_str.as_deref(), 7)?,
        reply_count: row.get(8)?,
        reply_to_user_id: row::parse_optional_uuid(reply_to_user_id_str.as_deref(), 9)?,
        reply_to_username: row.get(10)?,
        github_id: row.get(13)?,
        github_kind: github_kind_str.and_then(|s| str_to_activity_kind(&s)),
        github_state: github_state_str.and_then(|s| str_to_activity_state(&s)),
        github_html_url: row.get(16)?,
    })
}
```

Note: `get_replies`'s CTE has a different column order/count than the other queries (it doesn't join `u`/`u2` in the recursive part, and has a `depth` column) — add the four `github_*` raw columns to both CTE branches in the same relative position (right after `approved`), and add them to the final outer SELECT in the same position used by every other query (right after `rt.approved`, before `rt.depth` — `map_post_row` doesn't read `depth`, so its position doesn't matter to the mapper, just don't let it collide with positions 13-16).

- [ ] **Step 5: Run tests**

Run: `cargo test -p fido-server --features sqlite-tests` (full crate — this SELECT-list change touches every post-reading test).
Expected: PASS, including the new upsert test.

- [ ] **Step 6: Commit**

```bash
git add fido-server/src/db/repositories/post_repository.rs
git commit -m "feat(server): PostRepository reads/writes GitHub sync fields"
```

---

### Task 4: Synthetic `github` system user + idempotent get-or-create

**Files:**
- Modify: `fido-server/src/db/repositories/user_repository.rs`

**Interfaces:**
- Consumes: `UserRepository::get_by_username` (existing, case-insensitive), `UserRepository::create` (existing, currently `#[allow(dead_code)]` — this task removes that allow since it becomes used).
- Produces: `UserRepository::get_or_create_system_user(&self, username: &str) -> Result<User>`.

- [ ] **Step 1: Write the failing test**

Add to `user_repository.rs`'s test module:

```rust
#[test]
fn get_or_create_system_user_is_idempotent() {
    let db = setup_db(); // existing helper in this file
    let repo = UserRepository::new(db.pool.clone());

    let first = repo.get_or_create_system_user("github").expect("create");
    assert_eq!(first.username, "github");
    assert!(!first.is_test_user);

    let second = repo.get_or_create_system_user("github").expect("get existing");
    assert_eq!(first.id, second.id, "must not create a duplicate on second call");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p fido-server --features sqlite-tests get_or_create_system_user_is_idempotent`
Expected: FAIL — method not found.

- [ ] **Step 3: Implement**

In `user_repository.rs`, near `create`:

```rust
/// Get the system user with this username, creating it if it doesn't
/// exist yet. Used for the synthetic `github` author of synced posts —
/// idempotent, safe to call on every sync.
pub fn get_or_create_system_user(&self, username: &str) -> Result<User> {
    if let Some(existing) = self.get_by_username(username)? {
        return Ok(existing);
    }
    let user = User {
        id: Uuid::new_v4(),
        username: username.to_string(),
        bio: None,
        join_date: Utc::now(),
        is_test_user: false,
        is_admin: false,
    };
    self.create(&user)?;
    Ok(user)
}
```

Remove the `#[allow(dead_code)]` above `create` since it now has a real caller.

- [ ] **Step 4: Run tests**

Run: `cargo test -p fido-server --features sqlite-tests get_or_create_system_user_is_idempotent` — PASS.
Run: `cargo clippy -p fido-server --all-targets -- -D warnings` — clean (confirms removing the dead_code allow didn't leave anything else unused).

- [ ] **Step 5: Commit**

```bash
git add fido-server/src/db/repositories/user_repository.rs
git commit -m "feat(server): idempotent get-or-create for the synthetic github user"
```

---

### Task 5: Rewrite ActivityService — sync into posts, not a cache blob

**Files:**
- Modify: `fido-server/src/services/activity.rs`
- Modify: `fido-server/src/db/repositories/activity_repository.rs` (repurpose to the sync-gate table)
- Modify: `fido-server/src/db/repositories/mod.rs`

**Interfaces:**
- Consumes: `GithubService::repo_activity` (unchanged, returns `Vec<ActivityItem>` — reused as an internal fetch-shape only, no longer exposed to any HTTP client); `PostRepository::upsert_github_post` (Task 3); `UserRepository::get_or_create_system_user` (Task 4); `MembershipRepository::insert_if_missing` (existing, unchanged); `CommunityRepository::get_by_id` (existing).
- Produces: `ActivityRepository::{get_last_synced_at(&self, community_id: Uuid) -> Result<Option<DateTime<Utc>>>, mark_synced(&self, community_id: Uuid, at: DateTime<Utc>) -> Result<()>}` against `community_activity_sync`; `ActivityService::sync_activity(&self, community_id: Uuid) -> Result<()>`, `pub const ACTIVITY_SYNC_TTL_MINUTES: i64 = 10`.

- [ ] **Step 1: Repurpose ActivityRepository for the gate table**

Replace the contents of `fido-server/src/db/repositories/activity_repository.rs` entirely:

```rust
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use crate::db::DbPool;

/// Gates how often a community's GitHub activity is re-fetched. No payload —
/// synced GitHub issues/PRs live as rows in `posts`, keyed by github_id.
#[derive(Clone)]
pub struct ActivityRepository {
    pool: DbPool,
}

impl ActivityRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn get_last_synced_at(&self, community_id: Uuid) -> Result<Option<DateTime<Utc>>> {
        let conn = self.pool.get()?;
        let row: Option<String> = conn
            .query_row(
                "SELECT last_synced_at FROM community_activity_sync WHERE community_id = ?1",
                rusqlite::params![community_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        row.map(|s| Ok(DateTime::parse_from_rfc3339(&s)?.with_timezone(&Utc)))
            .transpose()
    }

    pub fn mark_synced(&self, community_id: Uuid, at: DateTime<Utc>) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO community_activity_sync (community_id, last_synced_at)
             VALUES (?1, ?2)
             ON CONFLICT(community_id) DO UPDATE SET last_synced_at = excluded.last_synced_at",
            rusqlite::params![community_id.to_string(), at.to_rfc3339()],
        )?;
        Ok(())
    }
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::db::Database;

    fn setup_db_with_community() -> (Database, Uuid) {
        let db = Database::in_memory().expect("db");
        db.initialize().expect("schema");
        let community_id = Uuid::new_v4();
        let conn = db.connection().expect("conn");
        conn.execute(
            "INSERT INTO communities (id, github_repo_id, owner, name, require_thread_approval, created_at)
             VALUES (?1, 1, 'octocat', 'hello', 0, ?2)",
            rusqlite::params![community_id.to_string(), Utc::now().to_rfc3339()],
        )
        .expect("insert community");
        (db, community_id)
    }

    #[test]
    fn mark_synced_and_get_last_synced_at_round_trip() {
        let (db, community_id) = setup_db_with_community();
        let repo = ActivityRepository::new(db.pool.clone());

        assert!(repo.get_last_synced_at(community_id).unwrap().is_none());

        let now = Utc::now();
        repo.mark_synced(community_id, now).unwrap();
        let fetched = repo.get_last_synced_at(community_id).unwrap().unwrap();
        assert_eq!(fetched, now);

        let later = now + chrono::Duration::minutes(5);
        repo.mark_synced(community_id, later).unwrap();
        assert_eq!(repo.get_last_synced_at(community_id).unwrap().unwrap(), later);
    }
}
```

Verify the `communities` table's exact column list/order against `fido-server/src/db/schema.rs` before writing the test's INSERT (adjust column names if they differ from the assumption above).

- [ ] **Step 2: Update repositories/mod.rs**

`ActivityRepository`'s public surface changed but its name/registration in `mod.rs` (`pub activity: ActivityRepository`, constructed via `ActivityRepository::new(pool.clone())`) needs no changes — same type name, same constructor shape.

- [ ] **Step 3: Write the failing service tests**

Replace `fido-server/src/services/activity.rs`'s test module with pure-logic TTL tests (same shape as 0.5.0's, now against `ACTIVITY_SYNC_TTL_MINUTES`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn sync_is_fresh_within_ttl_and_stale_after() {
        let now = Utc::now();
        assert!(sync_is_fresh(now - Duration::minutes(9), now));
        assert!(!sync_is_fresh(now - Duration::minutes(10), now));
        assert!(!sync_is_fresh(now - Duration::hours(2), now));
    }
}
```

- [ ] **Step 4: Run to verify it fails, then implement**

Run: `cargo test -p fido-server --features sqlite-tests sync_is_fresh` — FAIL (function doesn't exist yet).

Replace `fido-server/src/services/activity.rs`'s non-test content:

```rust
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use fido_types::Post;
use uuid::Uuid;

use crate::db::repositories::Repositories;
use crate::services::github::GithubService;

pub const ACTIVITY_SYNC_TTL_MINUTES: i64 = 10;
const GITHUB_SYSTEM_USERNAME: &str = "github";

pub(crate) fn sync_is_fresh(last_synced_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now - last_synced_at < Duration::minutes(ACTIVITY_SYNC_TTL_MINUTES)
}

pub struct ActivityService {
    repos: Repositories,
    github: GithubService,
}

impl ActivityService {
    pub fn new(repos: Repositories, github: GithubService) -> Self {
        Self { repos, github }
    }

    /// Ensure `posts` reflects the community's repo activity, fetching from
    /// GitHub only if the last sync is older than the TTL. Never errors out
    /// the caller — logs and returns Ok(()) on any failure, since existing
    /// synced posts remain valid even if this sync attempt fails.
    pub async fn sync_activity(&self, community_id: Uuid) -> Result<()> {
        let now = Utc::now();
        if let Some(last_synced_at) = self.repos.activity.get_last_synced_at(community_id)? {
            if sync_is_fresh(last_synced_at, now) {
                return Ok(());
            }
        }

        if let Err(error) = self.try_sync(community_id, now).await {
            tracing::warn!(%community_id, %error, "GitHub activity sync failed; serving existing posts");
        }
        Ok(())
    }

    async fn try_sync(&self, community_id: Uuid, now: DateTime<Utc>) -> Result<()> {
        let community = self
            .repos
            .communities
            .get_by_id(&community_id)?
            .ok_or_else(|| anyhow!("Community {} not found", community_id))?;

        // Any authenticated fido user's token works for a public-repo fetch;
        // system_user itself has no GitHub token, so use its own id only to
        // satisfy the signature — GithubService falls back to unauthenticated
        // when the given user has no stored token.
        let system_user = self
            .repos
            .users
            .get_or_create_system_user(GITHUB_SYSTEM_USERNAME)
            .context("Failed to get or create the github system user")?;

        let items = self
            .github
            .repo_activity(system_user.id, &community.owner, &community.name)
            .await
            .context("Failed to fetch repo activity from GitHub")?;

        if !items.is_empty() {
            self.repos.memberships.insert_if_missing(&fido_types::Membership {
                community_id,
                user_id: system_user.id,
                role: fido_types::MembershipRole::Member,
                created_at: now,
            })?;
        }

        for item in items {
            let mut content = item.title.clone();
            content.truncate(280);
            let post = Post {
                id: uuid::Uuid::new_v4(),
                author_id: system_user.id,
                author_username: system_user.username.clone(),
                community_id,
                content,
                created_at: item.created_at,
                upvotes: 0,
                downvotes: 0,
                approved: true,
                hashtags: Vec::new(),
                user_vote: None,
                parent_post_id: None,
                reply_count: 0,
                reply_to_user_id: None,
                reply_to_username: None,
                github_id: Some(item.github_id),
                github_kind: Some(item.kind),
                github_state: Some(item.state),
                github_html_url: Some(item.html_url),
            };
            self.repos.posts.upsert_github_post(&post)?;
        }

        self.repos.activity.mark_synced(community_id, now)?;
        Ok(())
    }
}
```

Verify `Membership`'s exact field names/`MembershipRole` variant names against `fido_types::models`/`enums` before finalizing — adjust the literal to match if they differ from the assumption above. Also verify `id` (the new post's id) doesn't collide with `upsert_github_post`'s `ON CONFLICT` semantics: on an update, the `id` in the VALUES clause is ignored by the `DO UPDATE SET` (which only touches `content`/`github_state`/`github_html_url`), so the existing row's original `id` is preserved — confirm this by re-reading Task 3's `upsert_github_post` SQL, and note it explicitly in a comment here since it's easy to misread as "the id changes on update" (it does not).

- [ ] **Step 5: Run tests**

Run: `cargo test -p fido-server --features sqlite-tests activity` and `cargo clippy -p fido-server --all-targets -- -D warnings`.
Expected: PASS / clean.

- [ ] **Step 6: Commit**

```bash
git add fido-server/src/services/activity.rs fido-server/src/db/repositories/activity_repository.rs
git commit -m "feat(server): sync GitHub activity into posts instead of a cache blob"
```

---

### Task 6: Wire sync into PostService::get_posts; delete the old activity endpoint

**Files:**
- Modify: `fido-server/src/services/posts.rs`
- Modify: `fido-server/src/api/communities.rs`
- Modify: `fido-server/src/lib.rs`
- Modify: `fido-server/src/main.rs`
- Modify: `fido-server/src/state.rs` (if `ActivityService` needs to be constructed here — check how `PostService` currently gets its dependencies)

**Interfaces:**
- Consumes: `ActivityService::sync_activity` (Task 5).
- Produces: `PostService::get_posts` calls `sync_activity` (swallowed errors) before querying. `GET /communities/:id/activity` endpoint, its route registrations, and `CommunityActivityResponse` are deleted.

- [ ] **Step 1: Give PostService an ActivityService**

Read `fido-server/src/services/posts.rs`'s `PostService` struct and `new` constructor (near the top of the file) to see its current fields (`repos: Repositories`, `event_bus: SharedEventBus`, per prior research). Add a field:

```rust
pub struct PostService {
    repos: Repositories,
    event_bus: SharedEventBus,
    activity: crate::services::activity::ActivityService,
}
```

Update `PostService::new` to accept/construct an `ActivityService` (mirror how it's constructed in `api/communities.rs`'s handlers today: `ActivityService::new(state.repos.clone(), state.github_service.clone())`) — find every call site of `PostService::new` (grep `PostService::new` across `fido-server/src`) and update each to pass the extra dependency.

- [ ] **Step 2: Write the failing test**

In `fido-server/src/services/posts.rs`'s test module, add a test proving `get_posts` doesn't error when there's no GitHub activity to sync (i.e., the swallow-on-error path doesn't accidentally propagate for the common "nothing to sync yet" case) — model this on the existing `get_posts` tests in this file's test module, using a fixture community whose repo doesn't exist upstream (so `try_sync`'s GitHub call errors, proving the error is swallowed and `get_posts` still returns `Ok`):

```rust
#[test]
fn get_posts_succeeds_even_when_activity_sync_fails() {
    // Reuse this file's existing test setup for a PostService with a
    // community and a membership, but point GITHUB_API_BASE at an address
    // with nothing listening (so the sync's GitHub fetch errors).
    // Assert get_posts(...) still returns Ok(vec![]) rather than propagating
    // the sync failure.
}
```

Write this concretely against the file's actual existing test harness (env var pattern for `GITHUB_API_BASE`, `env_lock()` guard) — follow the exact setup other tests in this file already use for constructing a `GithubService`/`PostService` pair.

- [ ] **Step 3: Implement the call site**

In `get_posts` (`fido-server/src/services/posts.rs`), right after `self.require_membership(user_id, community_id)?;`:

```rust
pub fn get_posts(
    &self,
    community_id: Uuid,
    sort_order: SortOrder,
    limit: i32,
    username: Option<&str>,
    user_id: Uuid,
) -> ApiResult<Vec<Post>> {
    self.require_membership(user_id, community_id)?;

    // Best-effort: pull fresh GitHub activity into posts before reading.
    // A tokio runtime must already be active (this is called from an async
    // Axum handler) — use futures::executor::block_on or make get_posts
    // async if the call site allows; check how this function is invoked
    // from api/posts.rs and adjust get_posts's signature to `async fn` if
    // needed rather than block_on-ing inside a sync fn on an async runtime.
    if let Err(e) = tokio::runtime::Handle::current()
        .block_on(self.activity.sync_activity(community_id))
    {
        tracing::warn!(%community_id, error = %e, "activity sync failed ahead of get_posts");
    }

    let mut posts = match username { /* ...unchanged... */ };
    self.populate_posts(&mut posts, Some(user_id))?;
    Ok(posts)
}
```

`tokio::runtime::Handle::current().block_on(...)` inside code that's already running on a tokio worker thread will panic (`Cannot start a runtime from within a runtime`). Check `fido-server/src/api/posts.rs`'s handler for `GET /posts` — if it calls `get_posts` from an `async fn` handler (very likely, since Axum handlers are async), the clean fix is to make `PostService::get_posts` itself `async fn` and `.await` the sync call directly, then update its single call site in `api/posts.rs` to `.await` it. Prefer that over `block_on` — do this if the call site is async (check before choosing).

- [ ] **Step 4: Run tests**

Run: `cargo test -p fido-server --features sqlite-tests` and `cargo clippy --workspace --all-targets -- -D warnings`.
Expected: PASS / clean.

- [ ] **Step 5: Delete the old activity endpoint**

Delete `CommunityActivityResponse` struct and `get_activity` handler from `fido-server/src/api/communities.rs`. Remove the `.route("/communities/:id/activity", get(api::communities::get_activity))` registration from both `fido-server/src/lib.rs` and `fido-server/src/main.rs`.

- [ ] **Step 6: Run full server test suite and commit**

Run: `cargo test -p fido-server --features sqlite-tests` and `cargo build --workspace`.
Expected: PASS, 0 errors.

```bash
git add fido-server/src
git commit -m "feat(server): sync GitHub activity from get_posts; remove old activity endpoint"
```

---

### Task 7: TUI — delete FeedEntry/activity-fetch layer, plain post list again

**Files:**
- Modify: `fido-tui/src/app/state.rs`
- Delete: `fido-tui/src/app/activity.rs`
- Modify: `fido-tui/src/app/mod.rs`
- Modify: `fido-tui/src/app/posts.rs`
- Modify: `fido-tui/src/app/build.rs`
- Modify: `fido-tui/src/app/communities.rs`
- Modify: `fido-tui/src/app/auth.rs`
- Modify: `fido-tui/src/app/settings.rs`
- Modify: `fido-tui/src/event_loop.rs`
- Modify: `fido-tui/src/api/client.rs`

**Interfaces:**
- Produces: `PostsState` with no `activity_items`/`activity_loading`/`activity_error`/`activity_pending_load`/`feed_entries` fields; `items_before_posts`/`post_index_to_list_index`/`list_index_to_post_index` deleted (callers use `list_state.selected()` and `posts[index]` directly); `App.selected_activity_url`/`load_activity`/`clear_activity` deleted.

- [ ] **Step 1: Delete state.rs's activity/feed-entry block**

In `fido-tui/src/app/state.rs`, delete: the `activity_items`, `activity_loading`, `activity_error`, `activity_pending_load`, `feed_entries` fields on `PostsState`; the `FeedEntry` enum; `rebuild_feed_entries`; the `impl PostsState` block containing `items_before_posts`, `rebuild_feed`, `selected_feed_entry`, `post_index_to_list_index`, `list_index_to_post_index`.

- [ ] **Step 2: Delete app/activity.rs and its module declaration**

Delete `fido-tui/src/app/activity.rs`. Remove `mod activity;` from `fido-tui/src/app/mod.rs`.

- [ ] **Step 3: Simplify posts.rs's load success path**

In `fido-tui/src/app/posts.rs::load_posts`, remove the `self.posts_state.rebuild_feed();` call (line ~68 in the None-community early return, and ~162 in the success branch) and remove the activity-trigger lines (`self.posts_state.activity_loading = true; self.posts_state.activity_pending_load = true;`, ~170-171). The success branch becomes just:

```rust
match result {
    Ok(posts) => {
        let has_posts = !posts.is_empty();
        self.posts_state.posts = posts;
        if has_posts {
            self.posts_state.list_state.select(Some(0));
        } else {
            self.posts_state.list_state.select(None);
        }
        self.posts_state.loading = false;
    }
    Err(e) => {
        let error_msg = categorize_error(&e.to_string());
        self.posts_state.error = Some(error_msg);
        self.posts_state.loading = false;
    }
}
```

Also fix `vote_on_selected_post` (same file) which currently does `list_state.selected()` → `list_index_to_post_index` → `posts[post_index]`; simplify to `list_state.selected()` → `posts.get(selected_index)` directly (no translation layer — list index IS post index again).

- [ ] **Step 4: Remove field init and clear_activity call sites**

In `fido-tui/src/app/build.rs`, remove the `activity_pending_load: false,` (and sibling activity field inits) from whatever `PostsState { ... }` literal builds it.

In `fido-tui/src/app/communities.rs`, `fido-tui/src/app/auth.rs`, `fido-tui/src/app/settings.rs`: remove every `self.clear_activity();` call site (these previously ran alongside `self.posts_state.posts.clear();` on logout/community-switch — now just deleting the posts vec and `list_state.select(None)` is sufficient, matching pre-0.5.0 behavior).

- [ ] **Step 5: event_loop.rs — remove the pending-load hook, delete the old `o` arm, adjust the `p` arm**

Remove the `activity_pending_load` block from `handle_pending_loads`:

```rust
async fn handle_pending_loads(&self, app: &mut App) -> Result<()> {
    if app.posts_state.pending_load {
        app.posts_state.pending_load = false;
        app.load_posts().await?;
    }

    if app.hashtags_state.show_hashtags_modal
        && app.hashtags_state.hashtags.is_empty()
        && !app.hashtags_state.loading
    {
        app.load_hashtags().await?;
    }

    Ok(())
}
```

Replace the old `o` key arm (the one gated on `app.selected_activity_url().is_some()`) with one that reads the selected **post** directly:

```rust
// o: open the selected post's GitHub issue/PR in the browser
KeyCode::Char('o')
    if app.current_screen == Screen::Main
        && app.current_tab == Tab::Posts
        && !app.viewing_post_detail
        && !app.composer_state.is_open()
        && !app.posts_state.show_filter_modal
        && app.input_mode == InputMode::Navigation
        && app.user_profile_view.is_none()
        && app
            .posts_state
            .list_state
            .selected()
            .and_then(|i| app.posts_state.posts.get(i))
            .and_then(|p| p.github_html_url.as_ref())
            .is_some() =>
{
    let url = app
        .posts_state
        .list_state
        .selected()
        .and_then(|i| app.posts_state.posts.get(i))
        .and_then(|p| p.github_html_url.clone());
    if let Some(url) = url {
        if let Err(e) = webbrowser::open(&url) {
            app.posts_state.error = Some(format!("Could not open browser: {}", e));
        }
    }
}
```

Update the `p` arm (profile of selected post's author) to also require the post is NOT a GitHub post:

```rust
KeyCode::Char('p') | KeyCode::Char('P')
    if app.user_profile_view.is_none()
        && app.current_screen == Screen::Main
        && app.current_tab == Tab::Posts
        && !app.viewing_post_detail
        && !app.composer_state.is_open()
        && !app.posts_state.show_filter_modal
        && !app.is_home_list_active()
        && app.input_mode == InputMode::Navigation =>
{
    let target = app
        .posts_state
        .list_state
        .selected()
        .and_then(|i| app.posts_state.posts.get(i))
        .filter(|post| post.github_kind.is_none())
        .map(|post| (post.author_id, post.author_username.clone()));
    if let Some((id, username)) = target {
        app.open_user_profile(id, username).await?;
    }
}
```

- [ ] **Step 6: Delete the TUI client's activity fetch method**

In `fido-tui/src/api/client.rs`, delete `get_community_activity` and `CommunityActivityResponse`.

- [ ] **Step 7: Run tests**

Run: `cargo build --workspace` — this will surface every remaining reference to deleted symbols (compile errors are the checklist). Fix each. Then `cargo test -p fido` and `cargo clippy --workspace --all-targets -- -D warnings`.
Expected: build succeeds once all references are gone; tests will still fail until Tasks 8-9 update rendering/tests — that's expected at this point, note it and continue (or, if doing this as one task, proceed directly to Task 8 before running tests, since this task alone leaves tabs.rs/formatting.rs/action_bar.rs/tests.rs referencing deleted symbols).

- [ ] **Step 8: Commit**

Commit only once Task 8 (rendering) is also done and the crate compiles — or commit here with a WIP note if executing tasks strictly sequentially with review gates; if using subagent-driven-development, treat Tasks 7+8 as one atomic unit if the reviewer requires a compiling diff (recommend combining 7 and 8 into a single task's implementer dispatch, since 7 alone doesn't compile — flag this to whoever executes the plan).

---

### Task 8: TUI — render GitHub posts inline, no separate feed-entry layer

**Files:**
- Modify: `fido-tui/src/ui/tabs.rs`
- Modify: `fido-tui/src/ui/formatting.rs`
- Modify: `fido-tui/src/ui/components/action_bar.rs`

**Interfaces:**
- Consumes: `Post.github_kind`/`github_state`/`github_html_url` (Task 2), plain post-list selection (Task 7).
- Produces: `activity_line_text(post: &Post) -> String`, `activity_glyph_color(post: &Post, theme: &ThemeColors) -> Color` (both retargeted from `&ActivityItem` to `&Post`); posts list renders every post via one branch, choosing `@author` vs the glyph/state-label header based on `github_kind`.

- [ ] **Step 1: Retarget formatting.rs**

In `fido-tui/src/ui/formatting.rs`, change the import from `use fido_types::{ActivityItem, ActivityKind, ActivityState};` to `use fido_types::{ActivityKind, ActivityState, Post};`. Rewrite both functions to take `&Post` and read the four Option fields (unwrap via `.unwrap_or` only where a post is guaranteed to have `github_kind.is_some()` at the call site — callers must check `github_kind.is_some()` before calling these):

```rust
/// Text for the GitHub-post header line, e.g.
/// `⊙ #7 Fix login · issue opened by alice` or
/// `⇄ #9 Dark mode · merged · bob`. Caller must ensure post.github_kind
/// is Some before calling.
pub fn activity_line_text(post: &Post) -> String {
    let Some(kind) = post.github_kind else {
        return post.content.clone();
    };
    let number = post.github_id.unwrap_or(0);
    let title = &post.content;
    match kind {
        ActivityKind::Issue => {
            let mut line = format!("⊙ #{} {}", number, title);
            if post.github_state == Some(ActivityState::Closed) {
                line.push_str(" · closed");
            }
            line
        }
        ActivityKind::PullRequest => {
            let status = match post.github_state {
                Some(ActivityState::Merged) => "merged",
                Some(ActivityState::Closed) => "closed",
                _ => "open",
            };
            format!("⇄ #{} {} · {}", number, title, status)
        }
    }
}

/// Color for the GitHub post's leading glyph, based on its state. Caller
/// must ensure post.github_state is Some before calling.
pub fn activity_glyph_color(post: &Post, theme: &ThemeColors) -> Color {
    match post.github_state {
        Some(ActivityState::Open) => theme.success,
        Some(ActivityState::Closed) => theme.error,
        Some(ActivityState::Merged) => Color::Magenta,
        None => theme.text_dim,
    }
}
```

(This drops the `opened by {author}` / `· {author}` suffix from 0.5.0's activity lines, since the header line for a GitHub post in the new design shows the glyph/state instead of `@author` — author attribution for a synced post is the synthetic `github` user, not interesting to display per-line. If the reviewer/user wants the original GitHub author's login preserved in the line text, that data is available on the server via `GithubIssue.user.login` at sync time — it is intentionally NOT carried onto `Post` in this plan since `Post.author_username` is already `"github"`; if per-item attribution turns out to matter, that's a follow-up, not blocking for 0.5.1.)

- [ ] **Step 2: Rewrite tabs.rs's feed rendering as a single post loop**

In `fido-tui/src/ui/tabs.rs`, remove the `FeedEntry` import. Replace the `selected_feed_entry`/`selected_post_index`/`entry_count`/`feed_entries.iter().enumerate().flat_map(...)` block with a single iteration over `app.posts_state.posts`:

```rust
let selected_post_index = app.posts_state.list_state.selected();
let post_count = app.posts_state.posts.len();

let feed_items: Vec<ListItem> = app
    .posts_state
    .posts
    .iter()
    .enumerate()
    .map(|(i, post)| {
        let is_selected = selected_post_index == Some(i);
        let is_last = i == post_count - 1;
        let mut post_lines: Vec<Line> = Vec::new();

        let is_github_post = post.github_kind.is_some();
        let header_style = if is_selected {
            Style::default().fg(theme.success).add_modifier(Modifier::BOLD)
        } else if is_github_post {
            Style::default().fg(activity_glyph_color(post, &theme))
        } else {
            Style::default().fg(theme.primary)
        };
        let prefix = if is_selected { "▶ " } else { "  " };
        let timestamp = format_timestamp(&post.created_at);

        if is_github_post {
            post_lines.push(Line::from(vec![
                Span::styled(prefix, header_style),
                Span::styled(activity_line_text(post), header_style),
                Span::raw(" • "),
                Span::styled(timestamp, Style::default().fg(theme.text_dim)),
            ]));
        } else {
            post_lines.push(Line::from(vec![
                Span::styled(prefix, header_style),
                Span::styled(format!("@{}", post.author_username), header_style),
                Span::raw(" • "),
                Span::styled(timestamp, Style::default().fg(theme.text_dim)),
            ]));
            let content_lines =
                format_post_content_with_width(&post.content, is_selected, &theme, post_width);
            post_lines.extend(content_lines);
        }

        let user_voted_up = post.user_vote.as_deref() == Some("up");
        let user_voted_down = post.user_vote.as_deref() == Some("down");
        let upvote_style = if user_voted_up {
            Style::default().fg(theme.success).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_dim)
        };
        let downvote_style = if user_voted_down {
            Style::default().fg(theme.error).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_dim)
        };
        post_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("↑ {} ", post.upvotes), upvote_style),
            Span::styled(format!("↓ {} ", post.downvotes), downvote_style),
            Span::styled(format!("💬 {}", post.reply_count), Style::default().fg(theme.text_dim)),
        ]));

        if !is_last {
            post_lines.push(Line::from(""));
        }

        ListItem::new(post_lines)
    })
    .collect();

items.extend(feed_items);

if !app.posts_state.posts.is_empty() {
    // ...existing "End of feed" block, changed from
    // `!app.posts_state.feed_entries.is_empty()` to `!app.posts_state.posts.is_empty()`
}
```

Note: this plan renders a GitHub post's title-as-content on one line (via `activity_line_text`) rather than the multi-line wrapped content a normal post gets — matching 0.5.0's single-line activity row look. If a GitHub post's title is long, `activity_line_text`'s output is not wrapped; add width-based truncation identical to 0.5.0's `truncated_activity_line` if lines overflow badly in manual testing (check this during Task 8's manual verification pass before considering the task done — don't skip it if titles visibly overflow).

Also update the empty-state guard (previously `app.posts_state.feed_entries.is_empty() && !app.posts_state.loading && !app.posts_state.activity_loading`) to `app.posts_state.posts.is_empty() && !app.posts_state.loading`.

Delete the now-unused `truncated_activity_line` function if nothing else calls it after this change (grep to confirm before deleting).

- [ ] **Step 3: Update action_bar.rs footer logic**

Replace the exclusive `else if matches!(app.posts_state.selected_feed_entry(), Some(FeedEntry::Activity(_)))` branch with compositional string-building based on the selected post:

```rust
} else if app.is_home_list_active() {
    "↑/↓/j/k: Navigate | Enter: Open community"
} else {
    let selected_post = app
        .posts_state
        .list_state
        .selected()
        .and_then(|i| app.posts_state.posts.get(i));
    let is_github_post = selected_post.map(|p| p.github_kind.is_some()).unwrap_or(false);

    if is_github_post {
        "u/d: Vote | Enter: Reply | o: Open on GitHub | i: Community | f: Filter | s: Search"
    } else {
        "u/d: Vote | n: Post | i: Community | f: Filter | s: Search | Space: View"
    }
}
```

Note: this function returns `&'static str`, so it cannot build a dynamic string with `format!` — the two arms above are both static literals, which is why `o: Open on GitHub` is folded into a second static string for the GitHub-post case rather than composed at runtime. Verify the non-GitHub-post hint text is byte-for-byte identical to what already exists there today (don't introduce an unintended footer wording change for normal posts) — confirm against the current file before finalizing this arm. Also confirm whichever key currently opens the reply/detail view for a normal post (`Space: View` per the existing hint) is the same key that lets a user comment on a GitHub post — if "comment" actually requires entering post-detail view first and composing a reply there (not a direct top-level key), change `"Enter: Reply"` above to whatever the real key/label is (check `post_detail.rs`'s reply-composer entry key before finalizing this string).

- [ ] **Step 4: Run tests**

Run: `cargo build --workspace` — fix remaining reference errors.
Run: `cargo test -p fido` — will still fail on Task 7/8's stale test file until Task 9. Proceed to Task 9 before expecting green.

- [ ] **Step 5: Commit** (combine with Task 7's changes into one commit if the crate didn't compile between them)

```bash
git add fido-tui/src/ui
git commit -m "feat(tui): render GitHub posts inline in the plain post list"
```

---

### Task 9: Rewrite TUI tests for the plain-post-list model

**Files:**
- Modify: `fido-tui/src/app/tests.rs`
- Modify: `fido-tui/src/ui/formatting.rs` (test module)

**Interfaces:**
- Consumes: everything from Tasks 7-8.
- Produces: passing test suite with no references to `FeedEntry`/`ActivityItem`/`feed_entries`/`selected_feed_entry`/`rebuild_feed_entries`.

- [ ] **Step 1: Delete obsolete tests**

In `fido-tui/src/app/tests.rs`, delete: `test_activity_created_at`, the activity-aware version of `test_posts_state_with` (replace with a plain version that only sets `posts`, matching pre-0.5.0 shape), `feed_entries_interleave_by_created_at_desc`, `selected_feed_entry_accounts_for_loading_offset`, `selected_feed_entry_accounts_for_activity_error_offset`, `open_selected_activity_noops_when_selection_is_a_post`, `logout_shaped_posts_clear_leaves_feed_entries_empty`, `open_selected_activity_returns_url_when_selection_is_activity`.

- [ ] **Step 2: Write replacement tests**

Add tests proving the new behavior — a GitHub post is selectable/votable like any post, and `o`/`p` gating works via `github_kind` directly:

```rust
#[test]
fn github_post_is_votable_like_any_post() {
    let mut app = App::new();
    app.posts_state.posts = vec![test_github_post()]; // helper: full Post literal with github_id: Some(19), github_kind: Some(ActivityKind::PullRequest), github_state: Some(ActivityState::Merged), github_html_url: Some(...)
    app.posts_state.list_state.select(Some(0));
    // Assert the post is a normal element of posts_state.posts, selectable
    // the same way any post is (list_state.selected() == Some(0) maps
    // directly to posts[0] — no translation layer).
    assert_eq!(app.posts_state.list_state.selected(), Some(0));
    assert!(app.posts_state.posts[0].github_kind.is_some());
}
```

Write a `test_github_post()` helper as a complete `Post` struct literal (all fields explicit, matching this codebase's "no placeholders" convention) alongside the file's existing `test_post_created_at` helper.

- [ ] **Step 3: Update formatting.rs's test module**

Rewrite `activity_line_formats_issue_and_merged_pr`-style tests to build `Post` literals instead of `ActivityItem` literals, calling the retargeted `activity_line_text(&Post)`/`activity_glyph_color(&Post, ...)`.

- [ ] **Step 4: Run full TUI test suite**

Run: `cargo test -p fido` and `cargo clippy --workspace --all-targets -- -D warnings`.
Expected: PASS / clean.

- [ ] **Step 5: Commit**

```bash
git add fido-tui/src/app/tests.rs fido-tui/src/ui/formatting.rs
git commit -m "test(tui): rewrite activity tests against the plain post-list model"
```

---

### Task 10: fido-types cleanup — drop the now-unused ActivityItem struct (keep the enums)

**Files:**
- Modify: `fido-types/src/models.rs`
- Modify: `fido-server/src/services/github.rs`

**Interfaces:**
- Consumes: nothing (this is a cleanup pass after every consumer of `ActivityItem` has been migrated to `Post`'s fields in Tasks 5-9).
- Produces: `ActivityItem` struct removed from `fido_types` public surface (or kept private to `fido-server` — see below); `ActivityKind`/`ActivityState` enums remain in `fido_types` (still used by `Post`).

- [ ] **Step 1: Confirm nothing outside fido-server still references ActivityItem**

Run: `grep -rn "ActivityItem" fido-tui/src fido-types/src fido-server/src`
Expected: only `fido-server/src/services/github.rs` (the `repo_activity`/`map_issue_to_activity` fetch-shape) and `fido-types/src/models.rs` (its definition) remain.

- [ ] **Step 2: Move ActivityItem out of the public fido_types surface**

Since `ActivityItem` is now purely an internal fetch-shape used only inside `fido-server/src/services/github.rs`, move its definition there (as a private `struct` local to that module) instead of leaving it in `fido_types`. Delete `ActivityItem` and its test module from `fido-types/src/models.rs`. Add the struct directly in `github.rs`:

```rust
/// One GitHub issue or PR as fetched from the API — an internal shape used
/// only to build synced Posts in ActivityService. Not exposed to the TUI.
#[derive(Debug, Clone)]
pub(crate) struct ActivityItem {
    pub github_id: i64,
    pub kind: ActivityKind,
    pub number: i64,
    pub title: String,
    pub author_login: String,
    pub state: ActivityState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub html_url: String,
}
```

Update `repo_activity`'s return type and `map_issue_to_activity`'s signature to use this local type instead of `fido_types::ActivityItem`. `ActivityKind`/`ActivityState` continue to come from `fido_types::enums` (imported as before) since `Post` still uses them.

- [ ] **Step 2: Run tests**

Run: `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`.
Expected: PASS / clean.

- [ ] **Step 3: Commit**

```bash
git add fido-types/src/models.rs fido-server/src/services/github.rs
git commit -m "refactor: ActivityItem becomes an internal fetch-shape, not a public type"
```

---

### Task 11: e2e coverage + CHANGELOG

**Files:**
- Modify: `scripts/e2e_tui.sh`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: everything above.

- [ ] **Step 1: Update Scenario 5 (from 0.5.0) for the new model**

The 0.5.0 e2e scenario asserted a `community_activity` cache row existed after board load. Update it to assert instead: (a) the rendered feed shows the stub issue's title styled as a GitHub post (grep for the stub's title text in the tmux pane, same as before), and (b) a `posts` row exists with `github_id` set for that community (`sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM posts WHERE community_id = '<id>' AND github_id IS NOT NULL;"`), and (c) vote on that post (`u` key) and confirm `upvotes` incremented in the DB — this is the actual point of 0.5.1, so the e2e scenario must prove interactivity, not just presence.

- [ ] **Step 2: Run the harness**

Run: `just e2e-tui`
Expected: all scenarios pass.

- [ ] **Step 3: CHANGELOG**

Add under `[Unreleased]` → `Added`/`Changed`:

```markdown
### Added

- Repo-activity posts (issues/PRs) can now be upvoted and replied to (commented on) directly in fido — same as any post.

### Changed

- GitHub activity is synced into real posts instead of a separate read-only overlay; the old `GET /communities/:id/activity` endpoint is removed.
```

- [ ] **Step 4: Commit**

```bash
git add scripts/e2e_tui.sh CHANGELOG.md
git commit -m "test(e2e): activity-post interactivity scenario; docs: changelog"
```
