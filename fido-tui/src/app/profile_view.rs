use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use uuid::Uuid;

use super::App;

impl App {
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

    /// Handle keyboard events for user profile view
    pub fn handle_user_profile_view_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.close_user_profile_view();
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                // Toggle follow/unfollow (will be handled async in main loop)
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                // Open DM if mutual friends (will be handled async in main loop)
            }
            _ => {}
        }
        Ok(())
    }
}
