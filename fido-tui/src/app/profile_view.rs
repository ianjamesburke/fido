use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use uuid::Uuid;

use super::App;

impl App {
    /// Calculate the next relationship state after a toggle, and whether to follow (true) or unfollow (false).
    /// Returns None for Self_ (can't toggle own profile).
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

    /// Fetch a user's profile and open the profile view.
    /// On fetch failure the view still opens with an inline error.
    pub async fn open_user_profile(&mut self, user_id: Uuid, username: String) -> Result<()> {
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

    /// Toggle follow status from profile view
    pub async fn toggle_follow_from_profile(&mut self) -> Result<()> {
        let Some(view) = &self.user_profile_view else {
            return Ok(());
        };
        if view.error.is_some() {
            return Ok(());
        }
        let Some((should_follow, new_rel)) =
            Self::next_relationship_after_toggle(&view.relationship)
        else {
            return Ok(());
        };
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
                    if should_follow {
                        view.follower_count += 1;
                    } else {
                        view.follower_count = view.follower_count.saturating_sub(1);
                    }
                }
                Err(e) => view.error = Some(format!("Follow action failed: {}", e)),
            }
        }
        Ok(())
    }

    /// Open DM with user from profile view
    pub async fn message_user_from_profile(&mut self) -> Result<()> {
        let Some(view) = &self.user_profile_view else {
            return Ok(());
        };
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

    /// Handle keyboard events for user profile view
    pub fn handle_user_profile_view_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.close_user_profile_view();
            }
            _ => {}
        }
        Ok(())
    }
}
