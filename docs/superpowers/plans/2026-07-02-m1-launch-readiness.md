# M1 Launch Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the connection path work end-to-end (find developer → view profile → message them), polish DMs and the community modal, then release.

**Architecture:** Rust workspace: `fido-types` (shared models), `fido-server` (Axum + SQLite, layered api → services → repositories), `fido-tui` (ratatui client; sync key handlers in `app/handlers/`, async operations in `event_loop.rs` `handle_async_key_events`, rendering in `ui/`). The server already has profile-view, follow, search, and DM-request endpoints; almost all work is TUI wiring plus one new server endpoint (community members).

**Tech Stack:** Rust, axum, ratatui, tokio, reqwest, rusqlite, tmux e2e harness (`scripts/e2e_tui.sh`).

## Global Constraints

- Never regex-parse structured syntax; SQL through repositories only.
- No silent fallbacks: fetch failures surface as visible error text in the UI (spec: profile fetch failure shows the modal with an inline error line).
- DM send rejections surface the server's reason verbatim.
- `p` opens a profile from every list that shows a username (friends modal, user search, posts list, DM conversation list). A key shown in a footer must work; remove footer entries that stay unwired.
- Run `cargo fmt` and `cargo clippy` before each commit; server tests need `cargo test -p fido-server --features sqlite-tests`.
- Async key behavior goes in `event_loop.rs::handle_async_key_events` match arms (they run before the sync handlers, so a guarded arm shadows a sync stub — same pattern as the existing `c` claim arm).

## Verified starting facts (do not re-derive)

- Server routes already registered in both `fido-server/src/lib.rs` and `fido-server/src/main.rs` (keep them in sync when adding routes): `GET /users/search`, `GET /users/:id/profile-view`, `POST|DELETE /users/:id/follow`, `GET /dms/requests`, `POST /dms/requests/:user_id/accept`, `POST /dms/requests/:user_id/decline`.
- `fido_types::UserProfileView` (models.rs:111) matches the `/users/:id/profile-view` response shape, including `relationship: RelationshipStatus` (serde-tagged, snake_case, `self` rename) — the TUI can deserialize it directly.
- `UserProfileViewState` (fido-tui `app/state.rs:372`) is never constructed; `p`/`f`/`m` are dead stubs (`app/friends.rs:153`, `app/profile_view.rs:13-18`).
- TUI list types drop user ids: `UserInfo` and `UserSearchResult` hold only username(+counts); server responses include `id` as a string.
- `MembershipRepository` already has `list_members` and `list_admins` (membership_repository.rs:124,137). There is no members API endpoint yet.
- Server DM gating works (`services/dms.rs::ensure_send_allowed`); the TUI has zero DM-request support and parses conversations from untyped JSON.

---

### Task 1: Thread user ids through TUI list types

**Files:**
- Modify: `fido-tui/src/api/client.rs` (structs `SocialUserInfo` ~line 531, `UserSearchResult` ~line 538)
- Modify: `fido-tui/src/app/state.rs` (`UserInfo` line 103, `UserSearchResult` line 133)
- Modify: `fido-tui/src/app/friends.rs` (`load_social_connections` mapping), `fido-tui/src/app/user_search.rs` (`search_users` mapping), `fido-tui/src/app/dms.rs` (`load_mutual_friends_for_dms` mapping)
- Test: `fido-tui/src/app/tests.rs`

**Interfaces:**
- Produces: `state::UserInfo { id: Uuid, username, follower_count, following_count }`, `state::UserSearchResult { id: Uuid, username }`, `api::SocialUserInfo`/`api::UserSearchResult` each with `pub id: String`. Later tasks rely on `UserInfo.id` and `UserSearchResult.id`.

- [ ] **Step 1: Write the failing test** — in `fido-tui/src/app/tests.rs` add:

```rust
#[test]
fn user_info_carries_id() {
    let id = uuid::Uuid::new_v4();
    let info = crate::app::UserInfo {
        id,
        username: "alice".to_string(),
        follower_count: 1,
        following_count: 2,
    };
    assert_eq!(info.id, id);
}
```

- [ ] **Step 2: Run it** — `cargo test -p fido user_info_carries_id` — expect FAIL (no field `id`).

- [ ] **Step 3: Add the fields.** In `api/client.rs` add `pub id: String,` to `SocialUserInfo` and `UserSearchResult` (server already sends `id`). In `app/state.rs` add `pub id: uuid::Uuid,` to `UserInfo` and `UserSearchResult`. Update the three mapping sites to parse:

```rust
// app/friends.rs load_social_connections (all three lists), app/dms.rs load_mutual_friends_for_dms:
.filter_map(|u| {
    Some(UserInfo {
        id: u.id.parse().ok()?,
        username: u.username,
        follower_count: u.follower_count,
        following_count: u.following_count,
    })
})
// app/user_search.rs search_users:
.filter_map(|r| {
    Some(UserSearchResult {
        id: r.id.parse().ok()?,
        username: r.username,
    })
})
```

(Change `.map(...)` to `.filter_map(...)` where needed.) Fix any other construction sites the compiler flags (`cargo build -p fido` and follow errors — `app/tests.rs` likely constructs `UserInfo`).

- [ ] **Step 4: Run tests** — `cargo test -p fido` — expect PASS.

- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat(tui): carry user ids through social list types"`

---

### Task 2: Profile fetch — API client method and rich profile state

**Files:**
- Modify: `fido-tui/src/api/client.rs` (new method near `search_users` ~line 455)
- Modify: `fido-tui/src/app/state.rs` (`UserProfileViewState` line 372), `fido-tui/src/app/build.rs:161` (unchanged, stays `None`)
- Create: `App::open_user_profile` in `fido-tui/src/app/profile_view.rs`
- Test: `fido-tui/src/app/tests.rs`

**Interfaces:**
- Consumes: `fido_types::UserProfileView`, `fido_types::RelationshipStatus`.
- Produces: `ApiClient::get_user_profile_view(user_id: Uuid) -> ApiResult<fido_types::UserProfileView>`; new `UserProfileViewState { user_id: Uuid, username: String, bio: Option<String>, join_date: String, follower_count: usize, following_count: usize, post_count: usize, relationship: fido_types::RelationshipStatus, error: Option<String> }`; `async fn App::open_user_profile(&mut self, user_id: Uuid, username: String) -> Result<()>`.

- [ ] **Step 1: Client method** — in `api/client.rs`:

```rust
/// Get another user's profile with relationship status
pub async fn get_user_profile_view(
    &self,
    user_id: Uuid,
) -> ApiResult<fido_types::UserProfileView> {
    let url = format!("{}/users/{}/profile-view", self.base_url, user_id);
    let req = self.add_auth_header(self.client.get(&url));
    let response = req.send().await?;
    self.handle_response(response).await
}
```

- [ ] **Step 2: Replace `UserProfileViewState`** in `app/state.rs`:

```rust
pub struct UserProfileViewState {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub bio: Option<String>,
    pub join_date: String,
    pub follower_count: usize,
    pub following_count: usize,
    pub post_count: usize,
    pub relationship: fido_types::RelationshipStatus,
    pub error: Option<String>,
}
```

- [ ] **Step 3: `open_user_profile`** in `app/profile_view.rs`:

```rust
impl App {
    /// Fetch a user's profile and open the profile view.
    /// On fetch failure the view still opens with an inline error.
    pub async fn open_user_profile(
        &mut self,
        user_id: uuid::Uuid,
        username: String,
    ) -> Result<()> {
        match self.api_client.get_user_profile_view(user_id).await {
            Ok(p) => {
                self.user_profile_view = Some(crate::app::UserProfileViewState {
                    user_id,
                    username: p.username,
                    bio: p.bio,
                    join_date: p.join_date,
                    follower_count: p.follower_count,
                    following_count: p.following_count,
                    post_count: p.post_count,
                    relationship: p.relationship,
                    error: None,
                });
            }
            Err(e) => {
                self.user_profile_view = Some(crate::app::UserProfileViewState {
                    user_id,
                    username,
                    bio: None,
                    join_date: String::new(),
                    follower_count: 0,
                    following_count: 0,
                    post_count: 0,
                    relationship: fido_types::RelationshipStatus::None,
                    error: Some(format!("Failed to load profile: {}", e)),
                });
            }
        }
        Ok(())
    }
}
```

- [ ] **Step 4: Fix compile fallout.** `cargo build -p fido` — update `ui/tabs.rs:384`-area render call and `ui/modals/social.rs::render_user_profile_view` field uses as the compiler directs (full render rework lands in Task 3). Update `app/post_detail.rs:471` `close_user_profile_view` if it references removed fields.

- [ ] **Step 5: Test** — add to `app/tests.rs` (uses a bogus server URL so the fetch fails; asserts the error-populated view still opens):

```rust
#[tokio::test]
async fn open_user_profile_surfaces_fetch_error() {
    let mut app = test_app(); // reuse the existing test-app constructor in tests.rs
    let id = uuid::Uuid::new_v4();
    app.open_user_profile(id, "ghost".to_string()).await.unwrap();
    let view = app.user_profile_view.as_ref().expect("view opens on error");
    assert_eq!(view.username, "ghost");
    assert!(view.error.is_some());
}
```

(If `tests.rs` has no test-app constructor, follow whatever pattern its existing tests use to build an `App`.) Run `cargo test -p fido open_user_profile_surfaces_fetch_error` — expect PASS.

- [ ] **Step 6: Commit** — `git commit -am "feat(tui): profile-view fetch and rich profile state"`

---

### Task 3: Wire `p` everywhere + render the real profile modal

**Files:**
- Modify: `fido-tui/src/event_loop.rs` (`handle_async_key_events`, before the `_ =>` arm)
- Modify: `fido-tui/src/app/friends.rs:153` (remove dead `p`/`f` stubs), `fido-tui/src/app/user_search.rs` (remove `j/k/d` special-casing bug; all chars type), `fido-tui/src/app/profile_view.rs` (Esc close + return-to-modal)
- Modify: `fido-tui/src/ui/modals/social.rs` (`render_user_profile_view`, friends-modal footer, search-modal footer)
- Test: `scripts/e2e_tui.sh` (assertion added in Task 8; unit-level here)

**Interfaces:**
- Consumes: `App::open_user_profile`, `UserInfo.id`, `UserSearchResult.id`, `Conversation.other_user_id`, `Post.author_id`/`author_username` (fido-types models.rs:46-47).
- Produces: `p` opens the profile from: friends modal, user search modal (also Enter), posts list, DM conversation list (Navigation mode). Esc from profile returns to the friends modal when it was the source (`friends_state.return_to_modal_after_profile` already exists).

- [ ] **Step 1: Async arms in `event_loop.rs`** (insert before the final `_ =>` arm; each extracts data in an inner scope to end the immutable borrow before the `await`):

```rust
// p: profile from friends modal
KeyCode::Char('p') | KeyCode::Char('P')
    if app.friends_state.show_friends_modal && !app.friends_state.search_mode =>
{
    let target = app
        .get_filtered_social_list()
        .get(app.friends_state.selected_index)
        .map(|u| (u.id, u.username.clone()));
    if let Some((id, username)) = target {
        app.friends_state.show_friends_modal = false;
        app.friends_state.return_to_modal_after_profile = true;
        app.open_user_profile(id, username).await?;
    }
}
// Enter: profile from user search modal
KeyCode::Enter if app.user_search_state.show_modal => {
    let target = app
        .user_search_state
        .search_results
        .get(app.user_search_state.selected_index)
        .map(|u| (u.id, u.username.clone()));
    if let Some((id, username)) = target {
        app.close_user_search_modal();
        app.open_user_profile(id, username).await?;
    }
}
// p: profile of selected post's author (posts list, board active)
KeyCode::Char('p') | KeyCode::Char('P')
    if app.current_screen == Screen::Main
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
        .map(|post| (post.author_id, post.author_username.clone()));
    if let Some((id, username)) = target {
        app.open_user_profile(id, username).await?;
    }
}
// p: profile from DM conversation list (navigation mode only)
KeyCode::Char('p') | KeyCode::Char('P')
    if app.current_tab == Tab::DMs
        && app.input_mode == InputMode::Navigation
        && !app.dms_state.show_new_conversation_modal =>
{
    let target = match &app.dms_state.selection {
        DMSelection::Conversation(idx) => app
            .dms_state
            .conversations
            .get(*idx)
            .map(|c| (c.other_user_id, c.other_username.clone())),
        _ => None,
    };
    if let Some((id, username)) = target {
        app.open_user_profile(id, username).await?;
    }
}
```

Guard interaction note: the profile view itself is checked at sync priority 1.5, so add `&& app.user_profile_view.is_none()` to each arm above.

- [ ] **Step 2: Clean the stubs.** Delete the `Char('p')` and `Char('f')` arms in `app/friends.rs::handle_friends_modal_keys` (the async arm now owns `p`; `f` moves to the profile view in Task 4). In `app/user_search.rs::handle_user_search_modal_keys` delete the `'j' | 'J'`, `'k' | 'K'`, `'d' | 'D'` special cases inside `KeyCode::Char(c)` so those letters are typeable in queries (arrow keys still navigate); delete the empty `Enter` arm comment (async owns Enter now). In the DM tab sync handler (`app/dms.rs::handle_dms_keys` Navigation catch-all), exclude `p` from the falls-into-typing catch-all:

```rust
KeyCode::Char('p') | KeyCode::Char('P') => {
    // Profile of selected conversation — handled async in event_loop
}
```

- [ ] **Step 3: Esc/q from profile returns to source.** In `app/profile_view.rs`, make close re-open the friends modal when flagged:

```rust
KeyCode::Esc | KeyCode::Char('q') => {
    self.close_user_profile_view();
    if self.friends_state.return_to_modal_after_profile {
        self.friends_state.return_to_modal_after_profile = false;
        self.friends_state.show_friends_modal = true;
    }
}
```

- [ ] **Step 4: Render the real modal.** In `ui/modals/social.rs::render_user_profile_view`: if `profile.error` is `Some`, render the modal with the username header and the error line styled `theme.error`, footer `"Esc: Close"`, and return. Otherwise replace the hardcoded relationship block (line ~217) with:

```rust
let (status_text, status_color) = match profile.relationship {
    fido_types::RelationshipStatus::Self_ => ("This is you", theme.text_dim),
    fido_types::RelationshipStatus::MutualFriends => ("Mutual friends", theme.success),
    fido_types::RelationshipStatus::Following => ("Following", theme.accent),
    fido_types::RelationshipStatus::FollowsYou => ("Follows you", theme.accent),
    fido_types::RelationshipStatus::None => ("Not following", theme.text_dim),
};
```

Add a join-date line under the header (`profile.join_date` is RFC3339 — show the first 10 chars, `&profile.join_date[..10.min(profile.join_date.len())]`). Footer becomes `"f: Follow/Unfollow | m: Message | Esc: Close"` (`"Esc: Close"` only, for `Self_`). Update the friends-modal footer (line ~136) to `"↑/↓/j/k: Navigate | p: View Profile | /: Search | Tab: Switch | Esc: Close"` (drop the unwired `f`). Update the search-modal footer (line ~376) to `"↑/↓: Navigate | Enter: View Profile | Esc: Close"`.

- [ ] **Step 5: Build and hand-verify** — `cargo build -p fido && cargo clippy -p fido`. Then run the app against a local server (`just server` + `just tui-local`), login as a test user, press `s`, search a user, Enter → profile opens; Esc; open friends modal from Profile tab (`f`), press `p` on a row → profile opens → Esc returns to the modal.

- [ ] **Step 6: Commit** — `git commit -am "feat(tui): wire p to open user profiles from all lists"`

---

### Task 4: Profile actions — `f` follow toggle, `m` message

**Files:**
- Modify: `fido-tui/src/api/client.rs` (follow/unfollow methods)
- Modify: `fido-tui/src/app/profile_view.rs` (toggle logic), `fido-tui/src/event_loop.rs` (two async arms)
- Test: `fido-tui/src/app/tests.rs`

**Interfaces:**
- Consumes: `POST|DELETE /users/:id/follow` (returns bare 200, no JSON body), `UserProfileViewState.relationship`, DM pending-draft mechanism (`dms_state.pending_conversation_username`, `DMSelection::PendingDraft`).
- Produces: `ApiClient::follow_user(user_id) -> ApiResult<()>`, `ApiClient::unfollow_user(user_id) -> ApiResult<()>`, `fn App::next_relationship_after_toggle(rel) -> Option<(bool, RelationshipStatus)>` (pure, `(should_follow, new_rel)`), async `App::toggle_follow_from_profile`, async `App::message_user_from_profile`.

- [ ] **Step 1: Failing test for the pure toggle transition** in `app/tests.rs`:

```rust
#[test]
fn follow_toggle_transitions() {
    use fido_types::RelationshipStatus as R;
    use crate::app::App;
    assert_eq!(App::next_relationship_after_toggle(&R::None), Some((true, R::Following)));
    assert_eq!(App::next_relationship_after_toggle(&R::FollowsYou), Some((true, R::MutualFriends)));
    assert_eq!(App::next_relationship_after_toggle(&R::Following), Some((false, R::None)));
    assert_eq!(App::next_relationship_after_toggle(&R::MutualFriends), Some((false, R::FollowsYou)));
    assert_eq!(App::next_relationship_after_toggle(&R::Self_), None);
}
```

- [ ] **Step 2: Run it** — expect FAIL (function missing).

- [ ] **Step 3: Implement.** Client methods (bare-status responses, so don't use `handle_response`):

```rust
/// Follow a user
pub async fn follow_user(&self, user_id: Uuid) -> ApiResult<()> {
    let url = format!("{}/users/{}/follow", self.base_url, user_id);
    let req = self.add_auth_header(self.client.post(&url));
    req.send().await?.error_for_status()?;
    Ok(())
}

/// Unfollow a user
pub async fn unfollow_user(&self, user_id: Uuid) -> ApiResult<()> {
    let url = format!("{}/users/{}/follow", self.base_url, user_id);
    let req = self.add_auth_header(self.client.delete(&url));
    req.send().await?.error_for_status()?;
    Ok(())
}
```

(If `ApiError` lacks a `From<reqwest::Error>` covering `error_for_status`, map it to the client's existing error variant.) In `app/profile_view.rs`:

```rust
pub fn next_relationship_after_toggle(
    rel: &fido_types::RelationshipStatus,
) -> Option<(bool, fido_types::RelationshipStatus)> {
    use fido_types::RelationshipStatus as R;
    match rel {
        R::Self_ => None,
        R::None => Some((true, R::Following)),
        R::FollowsYou => Some((true, R::MutualFriends)),
        R::Following => Some((false, R::None)),
        R::MutualFriends => Some((false, R::FollowsYou)),
    }
}

pub async fn toggle_follow_from_profile(&mut self) -> Result<()> {
    let Some(view) = &self.user_profile_view else { return Ok(()) };
    if view.error.is_some() { return Ok(()); }
    let Some((should_follow, new_rel)) = Self::next_relationship_after_toggle(&view.relationship)
    else { return Ok(()) };
    let user_id = view.user_id;
    let result = if should_follow {
        self.api_client.follow_user(user_id).await
    } else {
        self.api_client.unfollow_user(user_id).await
    };
    if let Some(view) = self.user_profile_view.as_mut() {
        match result {
            Ok(()) => {
                view.relationship = new_rel;
                if should_follow { view.follower_count += 1; }
                else { view.follower_count = view.follower_count.saturating_sub(1); }
            }
            Err(e) => view.error = Some(format!("Follow action failed: {}", e)),
        }
    }
    Ok(())
}

pub async fn message_user_from_profile(&mut self) -> Result<()> {
    let Some(view) = &self.user_profile_view else { return Ok(()) };
    let username = view.username.clone();
    self.close_user_profile_view();
    self.friends_state.return_to_modal_after_profile = false;
    self.friends_state.show_friends_modal = false;
    self.current_tab = crate::app::Tab::DMs;
    if !self.dms_state.conversations_loaded {
        self.load_conversations().await?;
    }
    if let Some(idx) = self
        .dms_state
        .conversations
        .iter()
        .position(|c| c.other_username == username)
    {
        self.dms_state.selection = crate::app::DMSelection::Conversation(idx);
        self.dms_state.needs_message_load = true;
    } else {
        self.dms_state.pending_conversation_username = Some(username);
        self.dms_state.selection = crate::app::DMSelection::PendingDraft;
        self.dms_state.messages.clear();
    }
    self.input_mode = crate::app::InputMode::Typing;
    Ok(())
}
```

Event-loop arms (before `_ =>`):

```rust
KeyCode::Char('f') | KeyCode::Char('F') if app.user_profile_view.is_some() => {
    app.toggle_follow_from_profile().await?;
}
KeyCode::Char('m') | KeyCode::Char('M') if app.user_profile_view.is_some() => {
    app.message_user_from_profile().await?;
}
```

Delete the now-shadowed empty `f`/`m` arms in `handle_user_profile_view_keys`.

- [ ] **Step 4: Run tests** — `cargo test -p fido` — expect PASS.

- [ ] **Step 5: Hand-verify the connection path** (definition of done): local server, two test users. `s` → search → Enter (profile) → `m` → type → Enter sends. As a non-mutual stranger the message goes out as a request (server auto-pending) — confirm no crash and the conversation appears.

- [ ] **Step 6: Commit** — `git commit -am "feat(tui): follow toggle and message action from profile view"`

---

### Task 5: Typed DM conversations with pending state

**Files:**
- Modify: `fido-server/src/services/dms.rs` (`ConversationSummary` + `get_conversations`)
- Modify: `fido-tui/src/api/client.rs` (`get_conversations` return type), `fido-tui/src/app/state.rs` (`Conversation`), `fido-tui/src/app/dms.rs` (`load_conversations`)
- Test: server test in `services/dms.rs` tests module

**Interfaces:**
- Consumes: `dm_conversations` repo (state + initiator already persisted; `DMService::should_auto_accept`, `ensure_send_allowed` unchanged).
- Produces: `ConversationSummary` gains `pub state: fido_types::DmConversationState, pub initiated_by_me: bool`; TUI `Conversation` gains the same two fields; `ApiClient::get_conversations() -> ApiResult<Vec<ConversationInfo>>` (new typed struct in client.rs mirroring the summary, `other_user_id: String`).

- [ ] **Step 1: Failing server test** — in the existing `#[cfg(all(test, feature = "sqlite-tests"))] mod tests` of `services/dms.rs` (reuse its `setup()`/`test_user` helpers):

```rust
#[test]
fn conversation_summary_exposes_pending_state() -> Result<()> {
    let (_db, repos, event_bus, sender, recipient) = setup()?;
    let service = DMService::new(repos, event_bus);
    service.send_message(&sender.id, &recipient.username, "hello")?;
    let sender_convos = service.get_conversations(&sender.id)?;
    assert_eq!(sender_convos.len(), 1);
    assert_eq!(sender_convos[0].state, fido_types::DmConversationState::Pending);
    assert!(sender_convos[0].initiated_by_me);
    let recipient_convos = service.get_conversations(&recipient.id)?;
    assert!(!recipient_convos[0].initiated_by_me);
    Ok(())
}
```

(Adjust the `DMService::new` call to the tests' existing construction pattern.)

- [ ] **Step 2: Run** — `cargo test -p fido-server --features sqlite-tests conversation_summary_exposes_pending_state` — expect FAIL.

- [ ] **Step 3: Implement server side.** Add the two fields to `ConversationSummary` (derive already includes Serialize) and populate them in `get_conversations` from the conversation row (state enum, `initiator_id == *user_id`). Follow the existing query/mapping code in that function; the conversations repo already loads state and initiator for `ensure_send_allowed`.

- [ ] **Step 4: Run** — expect PASS. Also `cargo test -p fido-server --features sqlite-tests` (full) — expect PASS.

- [ ] **Step 5: Type the TUI side.** In `api/client.rs`:

```rust
#[derive(Debug, serde::Deserialize)]
pub struct ConversationInfo {
    pub other_user_id: String,
    pub other_username: String,
    pub last_message: String,
    pub last_message_time: String,
    pub unread_count: usize,
    pub state: fido_types::DmConversationState,
    pub initiated_by_me: bool,
}
```

Change `get_conversations` to return `ApiResult<Vec<ConversationInfo>>` (same body — `handle_response` deserializes). In `app/state.rs` add `pub state: fido_types::DmConversationState, pub initiated_by_me: bool` to `Conversation`. Rewrite the JSON-poking block in `app/dms.rs::load_conversations` to:

```rust
self.dms_state.conversations = convos
    .into_iter()
    .filter_map(|c| {
        Some(Conversation {
            other_user_id: c.other_user_id.parse().ok()?,
            other_username: c.other_username,
            last_message: c.last_message,
            unread_count: c.unread_count as i32,
            state: c.state,
            initiated_by_me: c.initiated_by_me,
        })
    })
    .collect();
```

- [ ] **Step 6: Build + tests** — `cargo build && cargo test -p fido` — fix construction fallout, expect PASS.

- [ ] **Step 7: Commit** — `git commit -am "feat(dms): typed conversation summaries with pending state"`

---

### Task 6: DM requests in the TUI (accept/decline) and pending UX

**Files:**
- Modify: `fido-tui/src/api/client.rs` (3 methods + `DmRequestInfo` type)
- Modify: `fido-tui/src/app/state.rs` (`DMsState.pending_requests`, `DMSelection::Request(usize)`, navigation), `fido-tui/src/app/dms.rs` (load + keys), `fido-tui/src/event_loop.rs` (accept/decline arms)
- Modify: `fido-tui/src/ui/tabs.rs` (DM sidebar: requests section, pending badge; message panel: pending hint)
- Test: `fido-tui/src/app/tests.rs`

**Interfaces:**
- Consumes: `GET /dms/requests` → `[{from_user_id, from_username, created_at}]`, `POST /dms/requests/:user_id/accept|decline`, `Conversation.state/initiated_by_me` from Task 5.
- Produces: `ApiClient::{get_pending_dm_requests() -> ApiResult<Vec<DmRequestInfo>>, accept_dm_request(Uuid) -> ApiResult<()>, decline_dm_request(Uuid) -> ApiResult<()>}`; `DMsState.pending_requests: Vec<DmRequest>` where `pub struct DmRequest { pub from_user_id: Uuid, pub from_username: String }`; `DMSelection::Request(usize)` variant ordered between `NewConversation`/`PendingDraft` and `Conversation` in navigation.

- [ ] **Step 1: Failing navigation test** in `app/tests.rs`:

```rust
#[test]
fn dm_navigation_visits_requests() {
    let mut state = crate::app::DMsState::default(); // use the existing DMsState construction pattern from build.rs if no Default
    state.pending_requests = vec![crate::app::DmRequest {
        from_user_id: uuid::Uuid::new_v4(),
        from_username: "alice".to_string(),
    }];
    state.conversations = vec![]; // requests but no conversations
    state.selection = crate::app::DMSelection::NewConversation;
    state.navigate_down();
    assert!(matches!(state.selection, crate::app::DMSelection::Request(0)));
    state.navigate_up();
    assert!(matches!(state.selection, crate::app::DMSelection::NewConversation));
}
```

- [ ] **Step 2: Run** — expect FAIL (no `pending_requests`, no `Request` variant).

- [ ] **Step 3: Implement state + navigation.** Add `DmRequest` struct and `pending_requests: Vec<DmRequest>` to `DMsState` (initialize empty in `app/build.rs`). Add `Request(usize)` to `DMSelection`. Navigation order top-to-bottom: `NewConversation` → `PendingDraft` (if any) → `Request(0..n)` → `Conversation(0..m)`. Extend `navigate_down`/`navigate_up` accordingly (mirror the existing arm style; `conversation_index()` returns `None` for `Request`). Update `load_conversation_messages` and `send_dm` match arms to treat `Request(_)` like `NewConversation` (nothing to load / "accept the request first" error).

- [ ] **Step 4: Run the test** — expect PASS.

- [ ] **Step 5: Client methods:**

```rust
#[derive(Debug, serde::Deserialize)]
pub struct DmRequestInfo {
    pub from_user_id: String,
    pub from_username: String,
    pub created_at: String,
}

/// List pending DM requests addressed to me
pub async fn get_pending_dm_requests(&self) -> ApiResult<Vec<DmRequestInfo>> {
    let url = format!("{}/dms/requests", self.base_url);
    let req = self.add_auth_header(self.client.get(&url));
    let response = req.send().await?;
    self.handle_response(response).await
}

/// Accept a pending DM request from a user
pub async fn accept_dm_request(&self, from_user_id: Uuid) -> ApiResult<()> {
    let url = format!("{}/dms/requests/{}/accept", self.base_url, from_user_id);
    let req = self.add_auth_header(self.client.post(&url));
    let response = req.send().await?;
    let _: serde_json::Value = self.handle_response(response).await?;
    Ok(())
}

/// Decline a pending DM request from a user
pub async fn decline_dm_request(&self, from_user_id: Uuid) -> ApiResult<()> {
    let url = format!("{}/dms/requests/{}/decline", self.base_url, from_user_id);
    let req = self.add_auth_header(self.client.post(&url));
    let response = req.send().await?;
    let _: serde_json::Value = self.handle_response(response).await?;
    Ok(())
}
```

- [ ] **Step 6: Load requests with conversations.** At the top of `load_conversations` (before fetching conversations):

```rust
self.dms_state.pending_requests = self
    .api_client
    .get_pending_dm_requests()
    .await
    .map(|reqs| {
        reqs.into_iter()
            .filter_map(|r| {
                Some(DmRequest {
                    from_user_id: r.from_user_id.parse().ok()?,
                    from_username: r.from_username,
                })
            })
            .collect()
    })
    .unwrap_or_default();
```

Incoming pending conversations are represented by the requests list; filter them out of the conversation list to avoid double entries: after mapping conversations, `self.dms_state.conversations.retain(|c| !(c.state == fido_types::DmConversationState::Pending && !c.initiated_by_me));`

- [ ] **Step 7: Keys.** Sync handler (`handle_dms_keys` Navigation mode): add before the catch-all so `a`/`x` on a request don't fall into typing:

```rust
KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Char('x') | KeyCode::Char('X')
    if matches!(self.dms_state.selection, DMSelection::Request(_)) =>
{
    // accept/decline — handled async in event_loop
}
```

Event-loop arms:

```rust
KeyCode::Char('a') | KeyCode::Char('A')
    if app.current_tab == Tab::DMs
        && app.input_mode == InputMode::Navigation
        && matches!(app.dms_state.selection, DMSelection::Request(_)) =>
{
    if let DMSelection::Request(idx) = app.dms_state.selection {
        if let Some(req) = app.dms_state.pending_requests.get(idx) {
            let from = req.from_user_id;
            match app.api_client.accept_dm_request(from).await {
                Ok(()) => app.load_conversations().await?,
                Err(e) => app.dms_state.error = Some(format!("Accept failed: {}", e)),
            }
        }
    }
}
KeyCode::Char('x') | KeyCode::Char('X')
    if app.current_tab == Tab::DMs
        && app.input_mode == InputMode::Navigation
        && matches!(app.dms_state.selection, DMSelection::Request(_)) =>
{
    if let DMSelection::Request(idx) = app.dms_state.selection {
        if let Some(req) = app.dms_state.pending_requests.get(idx) {
            let from = req.from_user_id;
            match app.api_client.decline_dm_request(from).await {
                Ok(()) => app.load_conversations().await?,
                Err(e) => app.dms_state.error = Some(format!("Decline failed: {}", e)),
            }
        }
    }
}
```

- [ ] **Step 8: Render.** In `ui/tabs.rs`, find the DM sidebar list construction (the code that renders `New Conversation`, the pending draft, and `dms_state.conversations` — search `render_dms` / `DMSelection`). Insert a requests section between the draft entry and conversations: one row per request, text `format!("✉ @{} wants to chat", r.from_username)`, highlighted when `selection == DMSelection::Request(i)`. Conversations with `state == Pending && initiated_by_me` get a ` (pending)` suffix styled `theme.text_dim`. In the message panel, when the open conversation is outgoing-pending, render a dim hint line above the input: `"Request sent — waiting for @{username} to accept. You can't send more messages until they do."`. When `DMSelection::Request(_)` is selected, the message panel shows `"a: Accept | x: Decline"` instead of the input. Update the DMs footer/help line to include `p: Profile | a: Accept | x: Decline` where requests exist.

- [ ] **Step 9: Verify live.** Two test users, user A messages user B (stranger, no shared community): B's DM tab shows the request row; `a` accepts; conversation appears for both; second message from A now delivers. Re-run `cargo test -p fido`.

- [ ] **Step 10: Commit** — `git commit -am "feat(dms): incoming request accept/decline and pending-state UX"`

---

### Task 7: Community members endpoint + modal rework

**Files:**
- Modify: `fido-server/src/api/communities.rs` (new handler + response type), `fido-server/src/services/communities.rs` (new method + test), `fido-server/src/lib.rs` AND `fido-server/src/main.rs` (route)
- Modify: `fido-tui/src/api/client.rs` (client method), `fido-tui/src/app/state.rs` (`community_members` field), `fido-tui/src/event_loop.rs` (`ModalStateTracker`), `fido-tui/src/ui/modals/community.rs` (rework)

**Interfaces:**
- Consumes: `MembershipRepository::list_members` (membership_repository.rs:124), `repos.users`.
- Produces: `GET /communities/:id/members` → `Vec<CommunityMemberResponse { username: String, role: MembershipRole }>` sorted admins-first then by username; `CommunitiesService::list_members_with_usernames(&Uuid) -> Result<Vec<(String, MembershipRole)>>`; `ApiClient::get_community_members(community_id: Uuid) -> ApiResult<Vec<CommunityMemberInfo>>` where `CommunityMemberInfo { username: String, role: fido_types::MembershipRole }`; `App.community_members: Vec<CommunityMemberInfo>`.

- [ ] **Step 1: Failing service test** in the existing tests module of `services/communities.rs` (reuse its db/repos setup; create a community + two memberships, one admin one member):

```rust
#[test]
fn list_members_returns_usernames_admins_first() -> Result<()> {
    // build db/repos/users/community per this module's existing setup pattern
    // membership: zed -> Member, alice -> Admin
    let members = service.list_members_with_usernames(&community.id)?;
    assert_eq!(members[0], ("alice".to_string(), MembershipRole::Admin));
    assert_eq!(members[1], ("zed".to_string(), MembershipRole::Member));
    Ok(())
}
```

- [ ] **Step 2: Run** — `cargo test -p fido-server --features sqlite-tests list_members_returns_usernames_admins_first` — expect FAIL.

- [ ] **Step 3: Implement service:**

```rust
/// Members of a community with usernames, admins first then alphabetical.
pub fn list_members_with_usernames(
    &self,
    community_id: &Uuid,
) -> Result<Vec<(String, MembershipRole)>> {
    let memberships = self.repos.memberships.list_members(community_id)?;
    let mut members: Vec<(String, MembershipRole)> = memberships
        .into_iter()
        .filter_map(|m| {
            let user = self.repos.users.get(&m.user_id).ok().flatten()?;
            Some((user.username, m.role))
        })
        .collect();
    members.sort_by(|a, b| {
        let rank = |r: &MembershipRole| match r {
            MembershipRole::Admin => 0,
            MembershipRole::Contributor => 1,
            MembershipRole::Member => 2,
        };
        rank(&a.1).cmp(&rank(&b.1)).then(a.0.cmp(&b.0))
    });
    Ok(members)
}
```

(Adjust the users-repo getter name to what `UserRepository` actually exposes — check `db/repositories/user_repository.rs`.) API handler in `api/communities.rs` following the file's existing handler style, response `Vec<CommunityMemberResponse { username, role }>`. Register `.route("/communities/:id/members", get(api::communities::list_members))` in **both** `lib.rs` and `main.rs` next to the other community routes.

- [ ] **Step 4: Run server tests** — expect PASS.

- [ ] **Step 5: TUI fetch.** Client method `get_community_members` (GET, `handle_response`, type `CommunityMemberInfo` deriving Deserialize). Add `pub community_members: Vec<crate::api::CommunityMemberInfo>` to `App` (init empty in `build.rs`). In `event_loop.rs::ModalStateTracker` add a `community_modal: bool` field and:

```rust
async fn handle_community_modal(&mut self, app: &mut App) -> Result<()> {
    if app.show_community_modal && !self.community_modal {
        if let Some(community) = &app.community {
            let id = community.id;
            app.community_members = app
                .api_client
                .get_community_members(id)
                .await
                .unwrap_or_default();
        }
    }
    self.community_modal = app.show_community_modal;
    Ok(())
}
```

Call it from `check_and_load`.

- [ ] **Step 6: Rework the modal.** Rewrite `ui/modals/community.rs::render_community_modal`: size `55, 60`; drop `Alignment::Center` on the body (title line can stay centered via the modal title); left-aligned label/value lines with the existing `label_style`/`value_style`:

```
octocat/Hello-World          <- accent, bold
Owner:      octocat          <- community.owner
Your role:  admin
Members:    12
Admin:      claimed
Admins:     alice, bob       <- from app.community_members filtered to MembershipRole::Admin; "unclaimed — none" when empty
```

Keep the `c: Claim admin` line when unclaimed and the `Esc: Close` footer line. If more than 5 admins, show the first 5 and `+N more`.

- [ ] **Step 7: Verify live** — open a community board, press `i`: left-aligned modal with owner and admins list. `cargo test && cargo clippy`.

- [ ] **Step 8: Commit** — `git commit -am "feat(community): members endpoint and reworked community modal"`

---

### Task 8: Help modal, README keys, e2e coverage

**Files:**
- Modify: `fido-tui/src/ui/modals/help.rs` (add `p`, DM `a`/`x`, search Enter), `README.md` (Key Controls: add `p - View profile`, `s - Search users`)
- Modify: `scripts/e2e_tui.sh`

**Interfaces:**
- Consumes: everything above.

- [ ] **Step 1: Help + README.** Add to the help modal's key list (match its existing entry format): `p: View profile (posts, DMs, friends, search)`, `s: Search users (Posts tab)`, `a/x: Accept/decline DM request (DMs tab)`, `m: Message (profile view)`, `f: Follow/unfollow (profile view)`. Add `p` and `s` lines to README Key Controls.

- [ ] **Step 2: e2e profile flow.** Extend `scripts/e2e_tui.sh` after the existing board assertions (reuse `wait_for`, `pane`, tmux `send-keys` patterns already in the script): press `s`, type the seeded second test user's name, wait for the result row, press Enter, `wait_for "User Profile" "profile modal opens"`, assert the username appears in the pane, press `m`, `wait_for` the DMs tab marker, type `hello from e2e` + Enter, then assert via sqlite3 that a `direct_messages` row with that content exists (mirror the script's existing sqlite assertions). If only one test user is seeded, seed a second via the same mechanism the script already uses.

- [ ] **Step 3: Run** — `just e2e-tui` — expect PASS (build debug binaries first if the script expects them: `cargo build`).

- [ ] **Step 4: Commit** — `git commit -am "test(e2e): search→profile→message connection path; docs for new keys"`

---

### Task 9: Stabilize, release, seed the flagship community

**Files:**
- Modify: `Cargo.toml` workspace version (all three crates), `CHANGELOG.md`

- [ ] **Step 1: Full verification** — `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo test -p fido-server --features sqlite-tests && just e2e-tui`. Fix anything red before proceeding.
- [ ] **Step 2: Manual pass** of the main flows against the local stack (`./start.sh` or `just server` + `just tui-local`): login (test user + GitHub device flow), board browse/post/reply/vote, community modal, search→profile→message, DM request accept/decline, settings save, quit. Log every crash/wedge found; fix P0/P1s, file the rest.
- [ ] **Step 3: Version + changelog.** Bump versions, write CHANGELOG entries for: profile viewing, DM requests UI, community modal, search polish.
- [ ] **Step 4: Publish** — `just deploy-cargo` dry-run recipe first, then the real publish (per justfile; the arg-form dry-run bug was already fixed in e7b347f). Push `main`; Railway auto-deploys — verify the web demo at the production URL matches the new build.
- [ ] **Step 5: Seed the fido community** (in the deployed app, as the real GitHub account): a welcome thread, a feedback thread ("what hurt?"), and a "what should we build next" thread.
- [ ] **Step 6: Announce** — draft the Show HN / r/rust / r/commandline posts for user review; post ONE first (user's call which), wait for feedback before the next. **User action required for publish credentials and announcements.**
- [ ] **Step 7: Commit + tag** — `git commit -am "release: M1 launch readiness" && git tag v<version>`

---

## Deferred (M2 — do not build now)

Contribution graph, markdown renderer, profile READMEs, group chats, server-side search ranking.
