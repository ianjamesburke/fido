use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use super::{App, RelationshipStatus, UserProfileViewState};

impl App {
    /// Toggle follow/unfollow for user in profile view
    pub fn toggle_follow_in_profile_view(&mut self) {
        if let Some(profile) = &mut self.user_profile_view {
            match &profile.relationship {
                RelationshipStatus::Following | RelationshipStatus::MutualFriends => {
                    // Will unfollow (handled async in main loop)
                }
                RelationshipStatus::None | RelationshipStatus::FollowsYou => {
                    // Will follow (handled async in main loop)
                }
                RelationshipStatus::Self_ => {
                    // Cannot follow yourself
                }
            }
        }
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

    /// Load user profile view
    pub async fn load_user_profile_view(&mut self, user_id: String) -> Result<()> {
        match self.api_client.get_user_profile_view(user_id.clone()).await {
            Ok(profile_data) => {
                self.user_profile_view = Some(UserProfileViewState {
                    user_id: profile_data.id,
                    username: profile_data.username,
                    bio: profile_data.bio,
                    join_date: profile_data.join_date,
                    follower_count: profile_data.follower_count,
                    following_count: profile_data.following_count,
                    post_count: profile_data.post_count,
                    relationship: match profile_data.relationship {
                        fido_types::RelationshipStatus::Self_ => RelationshipStatus::Self_,
                        fido_types::RelationshipStatus::MutualFriends => {
                            RelationshipStatus::MutualFriends
                        }
                        fido_types::RelationshipStatus::Following => RelationshipStatus::Following,
                        fido_types::RelationshipStatus::FollowsYou => {
                            RelationshipStatus::FollowsYou
                        }
                        fido_types::RelationshipStatus::None => RelationshipStatus::None,
                    },
                    loading: false,
                    error: None,
                });
            }
            Err(e) => {
                // Show error but don't open profile view
                self.posts_state.error = Some(format!("Failed to load profile: {}", e));
            }
        }
        Ok(())
    }

    /// Follow user in profile view
    pub async fn follow_user_in_profile_view(&mut self, user_id: String) -> Result<()> {
        match self.api_client.follow_user(user_id.clone()).await {
            Ok(_) => {
                // Reload profile to get updated relationship status
                self.load_user_profile_view(user_id).await?;
            }
            Err(e) => {
                if let Some(profile) = &mut self.user_profile_view {
                    profile.error = Some(format!("Failed to follow: {}", e));
                }
            }
        }
        Ok(())
    }

    /// Unfollow user in profile view
    pub async fn unfollow_user_in_profile_view(&mut self, user_id: String) -> Result<()> {
        match self.api_client.unfollow_user(user_id.clone()).await {
            Ok(_) => {
                // Reload profile to get updated relationship status
                self.load_user_profile_view(user_id).await?;
            }
            Err(e) => {
                if let Some(profile) = &mut self.user_profile_view {
                    profile.error = Some(format!("Failed to unfollow: {}", e));
                }
            }
        }
        Ok(())
    }
}
