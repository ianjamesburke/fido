# Repo Activity Feed + Community Badge Design

Date: 2026-07-03
Status: approved direction, spec for implementation planning

## Goal

Two features that make communities alive and discoverable:

1. **Repo activity feed**: GitHub issues and PRs from the last 14 days (up to 100) appear in the community feed as a distinct, read-only item type. Solves cold start: every community has content on first open.
2. **Community badge**: a live SVG endpoint (`/badge/:owner/:repo.svg`) showing the community's member count, for maintainers to embed in their README. Always current, committed once.

Verification target: the `ianjamesburke/fido` community on prod.

## Part 1: Repo Activity Feed

### Data model (fido-types)

New model in `fido-types/src/models.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityItem {
    pub github_id: i64,
    pub kind: ActivityKind,       // enums.rs: Issue | PullRequest
    pub number: i64,
    pub title: String,
    pub author_login: String,
    pub state: ActivityState,     // enums.rs: Open | Closed | Merged
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
    pub html_url: String,
}
```

`ActivityKind` and `ActivityState` are new enums in `fido-types/src/enums.rs`, serialized snake_case like the existing enums. Activity items are NOT posts: no votes, no replies, no post id. This keeps the posts pipeline clean and preserves the `.fido/community.toml` event-filtering guardrail (settings resolver decides later which kinds show).

### Server: fetch + cache

- `GithubService::repo_activity(user_id, owner, name) -> Result<Vec<ActivityItem>>`:
  - One request: `GET {api_base}/repos/{owner}/{name}/issues?state=all&since=<now - 14 days>&per_page=100&sort=created&direction=desc`.
  - Authenticated with the **requesting user's stored token** (every fido user has one from OAuth login; 5000 req/hr per token). Falls back to unauthenticated only if the token row is missing, using the existing `get_public` helper.
  - GitHub returns PRs in the issues endpoint; an item with a `pull_request` key maps to `ActivityKind::PullRequest`. Merged detection: `pull_request.merged_at` non-null → `Merged`; otherwise map GitHub `state` open/closed.
- Cache table `community_activity` in `db/schema.rs`:
  - `community_id TEXT PRIMARY KEY, payload TEXT NOT NULL, fetched_at TEXT NOT NULL` — payload is the serialized `Vec<ActivityItem>`. Blob-not-columns, per the community-config guardrail (settings as data).
  - TTL 10 minutes: repository method returns cached payload when `fetched_at` is fresh; service refreshes on miss/stale and upserts.
  - GitHub fetch failure with a stale cache present: serve stale, log warn. Failure with no cache: propagate the error to the endpoint (surfaced in TUI as a feed-level notice line, not a crash).
- Endpoint `GET /communities/:id/activity` (authenticated), in `api/communities.rs`, routed in BOTH `lib.rs` and `main.rs`. Response: `{ "items": [ActivityItem], "fetched_at": rfc3339 }`.

### TUI

- `App` state: `activity_items: Vec<ActivityItem>`, `activity_loading: bool`, `activity_error: Option<String>`.
- Load is async and non-blocking, same pattern as the community-modal member fetch: board renders immediately from fido data; activity fetch fires after community resolution; `activity_loading` drives a single dim row `⊙ loading repo activity...` at the top of the feed until it resolves.
- Feed rendering: activity items interleave with posts by `created_at` in the posts list. Distinct style so they read as ambient, not authored:
  - Issues: `⊙ #123 Fix login crash · opened by alice · 2d` — `⊙` green when open, red when closed.
  - PRs: `⇄ #124 Add dark mode · merged · bob · 1d` — `⇄` magenta when merged, green open, red closed.
  - Whole line dimmed relative to posts; no vote counts, no reply affordance.
- Selection: the posts-list selection model gains an Activity variant. On an activity item:
  - `o` opens `html_url` via the existing `webbrowser` helper (reuse `auth.rs` open pattern; extract if needed).
  - Enter, votes (`u`/`d`), reply keys are no-ops on activity items; footer shows only keys that work on the current selection (footer-honesty constraint from M1).
- `o` is also advertised in the footer only when an activity item is selected.

### Non-goals (this iteration)

- Releases, stars, forks as activity kinds.
- Filtering which kinds appear (that is `.fido/community.toml` territory later).
- Persisting activity as posts, or any interaction with activity items beyond opening the browser.

## Part 2: Community Badge

- Public (unauthenticated) endpoint `GET /badge/:owner/:repo.svg` in fido-server:
  - Looks up the community by owner/name. 404 with a plain-text body when no community exists.
  - Renders a static-template SVG, shields.io flat style: left segment `fido`, right segment `N members`. Member count from the existing membership repository.
  - `Content-Type: image/svg+xml`, `Cache-Control: public, max-age=300`.
  - No new dependencies: the SVG is a `format!` template with the count and computed text widths. Escape nothing user-controlled beyond the count (a number) — owner/repo never appear in the SVG body.
- Rate limiting: endpoint sits behind the existing public rate-limit layer.
- Snippet for maintainers, documented in README under a "Badge" section:
  ```markdown
  [![fido community](https://fido-web-production.up.railway.app/badge/OWNER/REPO.svg)](https://github.com/OWNER/REPO)
  ```
- Notable members / admins display is out of scope for the badge itself; the badge links wherever the maintainer points it. A future fido web landing page per community can carry admins/most-upvoted.

## Testing

- Server unit tests: activity mapping (issue vs PR vs merged), cache TTL behavior (fresh hit skips fetch, stale refresh, stale-serve on fetch error), badge SVG contains the member count and correct content type, badge 404 on unknown repo.
- TUI unit tests: interleave ordering, selection navigation across mixed posts/activity, `o` no-op on posts / fires on activity, footer honesty per selection.
- e2e (`scripts/e2e_tui.sh`): stub GitHub issues endpoint in the existing GitHub stub; assert the feed shows an activity row and the loading row appears then disappears.
- Live verification on prod against the `ianjamesburke/fido` community, and badge rendered for `ianjamesburke/fido`.

## Performance answers (from brainstorm)

- 100 issues over 14 days = one GitHub request (~300-800ms), per community, per 10-minute cache window. Not a bottleneck.
- Startup cost: zero blocking. Board paints as today; activity streams in after, behind the loading row.
