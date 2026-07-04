# GitHub Activity as Posts Design

Date: 2026-07-04
Status: approved, spec for implementation planning

## Goal

fido 0.5.0 shipped GitHub issues/PRs as a read-only `ActivityItem`/`FeedEntry` layer, interleaved into the feed but not interactive — `o` opens the item in a browser, nothing else. 0.5.1 makes them fully interactive within fido: upvote and comment (reply), without writing anything back to GitHub.

The shape: a GitHub issue/PR becomes a real `Post` row. Votes and replies work for free — they're already built for posts. "Comment" is the existing reply pipeline; no new UI. This replaces the `ActivityItem`/`FeedEntry`/interleave machinery shipped in 0.5.0, it does not extend it.

## Data model

`posts` gains four nullable columns:
- `github_id INTEGER` — GitHub's numeric id for the issue/PR. NULL for a normal user post. `UNIQUE(community_id, github_id)` where non-null, so re-sync upserts instead of duplicating.
- `github_kind TEXT` — `'issue' | 'pull_request'`, NULL for a normal post.
- `github_state TEXT` — `'open' | 'closed' | 'merged'`, NULL for a normal post.
- `github_html_url TEXT` — link opened by `o`, NULL for a normal post.

`fido_types::Post` gains the matching `Option` fields, all `#[serde(default)]` so existing clients degrade gracefully.

`content` keeps its existing 280-char CHECK constraint. A GitHub post's `content` is the issue/PR title, truncated to 280 chars if needed — the same field every renderer already reads. The dedicated `github_*` columns drive the glyph (⊙ issue / ⇄ PR), the state label (open/closed/merged), and the `o` action; they do not change how `content` renders.

## Authorship

Posts require a real `author_id` (`NOT NULL FOREIGN KEY -> users`). GitHub issue/PR authors usually aren't fido users. Each fido install gets one synthetic system user, username `github`, created lazily on first sync (idempotent `get_or_create` by username, matching the pattern seed/test users already use). All synced GitHub posts are authored by this user. If a real fido user happens to match the GitHub author's login, this design does not attempt to attribute the post to them — that's a future enhancement, not required for interactivity.

The `github` user must hold membership in every community it posts into, since `PostService::create_post` enforces `require_membership`. Sync grants that membership automatically (idempotent) before the first upsert into a community.

## Sync mechanism

Replaces the `community_activity` payload-cache table from 0.5.0 with `community_activity_sync(community_id TEXT PRIMARY KEY, last_synced_at TEXT NOT NULL)` — no payload, just a gate. `ActivityService::sync_activity(community_id)`:
1. If `last_synced_at` is within the existing 10-minute TTL, no-op (posts table already holds the current state).
2. Otherwise: call the existing `GithubService::repo_activity` (unchanged — same one-call, 14-day, ≤100-item fetch), ensure the `github` user + membership exist, and for each item upsert a post keyed on `(community_id, github_id)`: insert if new, update `content`/`github_state` if the cached row's state changed (an issue can close/reopen; a PR can merge) — votes and replies on that post are preserved across the update since it's the same post id. Update `last_synced_at`.
3. Fetch failure: log and leave existing synced posts as-is (they simply age past freshness until the next successful sync) — no error surfaced to the reader, since the posts already in the feed remain valid.

Triggered the same way 0.5.0 triggered activity load: after `load_posts` succeeds, fire-and-forget a sync call, then re-fetch posts so newly synced items appear. Concretely: `GET /communities/:id/posts` gains a param or the service internally calls `sync_activity` before querying — simplest: `PostService::get_posts` calls `ActivityService::sync_activity` at the top (swallowing errors per point 3) before its existing query, so posts and GitHub items are always consistent in one read. This removes the separate `/communities/:id/activity` endpoint, the TUI's separate fetch-after-posts-load step, and the `activity_loading`/`activity_pending_load`/`activity_error` state entirely.

## TUI changes

Delete: `ActivityItem`/`FeedEntry`/`rebuild_feed_entries`/`selected_feed_entry`/the activity-aware index-translation layer/`app/activity.rs`'s fetch logic/the separate loading-row rendering. The posts list becomes the only list again — `list_state.selected()` maps directly to `posts[index]`, as it did before 0.5.0. This also retires the stale-`feed_entries` bug class the final review caught in 0.5.0, since there's no longer a second index space to go stale.

Add: a `Post` with `github_kind.is_some()` renders with the ⊙/⇄ glyph and state label in place of `@author`, using `github_state` for glyph color (open/closed/merged) exactly as 0.5.0's activity rows did. Vote (`u`/`d`) and reply/comment (the existing reply flow — same key that opens the reply composer on any post) work unchanged, since it's a real post underneath. `p` (view profile) is disabled on a GitHub post: the synthetic `github` user has no real profile worth viewing, so `p` no-ops and is hidden from the footer for that selection, per the existing footer-honesty rule ("a key shown in a footer must work"). `o` still opens `github_html_url` in the browser, shown/active only when the selected post has `github_kind.is_some()`, and coexists in the footer alongside the vote/reply hints.

## Non-goals

- Never write back to GitHub (no comment/reaction API calls) — explicit user decision this session.
- No per-user GitHub-author attribution matching.
- No change to what's fetched (still 14 days, ≤100 items, one API call) — only what happens to the fetched items server-side.

## Migration note

Existing prod communities already have 0.5.0's `community_activity` cache table and any joined `ianjamesburke/fido`/`ashaweeee/sudoku` communities have never had GitHub posts before. `CREATE TABLE IF NOT EXISTS` additions are additive and safe; the old `community_activity` table is simply left unused (dropping it is optional cleanup, not required — matches this codebase's existing practice of not writing destructive migrations casually).
