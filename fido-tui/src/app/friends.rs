use super::*;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Load social connections (following, followers, mutual friends)
    pub async fn load_social_connections(&mut self) -> Result<()> {
        self.friends_state.loading = true;
        self.friends_state.error = None;

        // Load all three lists concurrently to reduce modal open latency.
        let following_future = self.api_client.get_following_list();
        let followers_future = self.api_client.get_followers_list();
        let mutual_future = self.api_client.get_mutual_friends_list();
        let (following_result, followers_result, mutual_result) =
            tokio::join!(following_future, followers_future, mutual_future);

        // Extract values, handling errors inline
        let following = following_result.map_err(|e| {
            let error_msg = format!("Failed to load following: {}", e);
            self.friends_state.error = Some(error_msg.clone());
            self.friends_state.loading = false;
            anyhow::anyhow!(error_msg)
        })?;
        let followers = followers_result.map_err(|e| {
            let error_msg = format!("Failed to load followers: {}", e);
            self.friends_state.error = Some(error_msg.clone());
            self.friends_state.loading = false;
            anyhow::anyhow!(error_msg)
        })?;
        let mutual = mutual_result.map_err(|e| {
            let error_msg = format!("Failed to load mutual friends: {}", e);
            self.friends_state.error = Some(error_msg.clone());
            self.friends_state.loading = false;
            anyhow::anyhow!(error_msg)
        })?;

        self.friends_state.following = following
            .into_iter()
            .map(|u| UserInfo {
                username: u.username,
                follower_count: u.follower_count,
                following_count: u.following_count,
            })
            .collect();

        self.friends_state.followers = followers
            .into_iter()
            .map(|u| UserInfo {
                username: u.username,
                follower_count: u.follower_count,
                following_count: u.following_count,
            })
            .collect();

        self.friends_state.mutual_friends = mutual
            .into_iter()
            .map(|u| UserInfo {
                username: u.username,
                follower_count: u.follower_count,
                following_count: u.following_count,
            })
            .collect();

        self.friends_state.loading = false;
        Ok(())
    }

    /// Close social connections modal
    pub fn close_friends_modal(&mut self) {
        self.friends_state.show_friends_modal = false;
        self.friends_state.search_mode = false;
        self.friends_state.search_query.clear();
        self.friends_state.selected_index = 0;
        self.friends_state.error = None;
    }

    /// Get filtered user list based on current tab and search
    pub fn get_filtered_social_list(&self) -> Vec<&UserInfo> {
        let list = match self.friends_state.selected_tab {
            SocialTab::Following => &self.friends_state.following,
            SocialTab::Followers => &self.friends_state.followers,
            SocialTab::MutualFriends => &self.friends_state.mutual_friends,
        };

        if self.friends_state.search_query.is_empty() {
            list.iter().collect()
        } else {
            let query = self.friends_state.search_query.to_lowercase();
            list.iter()
                .filter(|u| u.username.to_lowercase().contains(&query))
                .collect()
        }
    }

    /// Handle social connections modal key events
    pub fn handle_friends_modal_keys(&mut self, key: KeyEvent) -> Result<()> {
        // If in search mode, handle search input
        if self.friends_state.search_mode {
            match key.code {
                KeyCode::Char(c) => {
                    self.friends_state.search_query.push(c);
                    self.friends_state.selected_index = 0; // Reset selection when searching
                }
                KeyCode::Backspace => {
                    self.friends_state.search_query.pop();
                    self.friends_state.selected_index = 0;
                }
                KeyCode::Esc => {
                    self.friends_state.search_mode = false;
                    self.friends_state.search_query.clear();
                }
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if self.friends_state.selected_index > 0 {
                    self.friends_state.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                let max_index = self.get_filtered_social_list().len().saturating_sub(1);
                if self.friends_state.selected_index < max_index {
                    self.friends_state.selected_index += 1;
                }
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                // Cycle through tabs
                self.friends_state.selected_tab = match self.friends_state.selected_tab {
                    SocialTab::Following => SocialTab::Followers,
                    SocialTab::Followers => SocialTab::MutualFriends,
                    SocialTab::MutualFriends => SocialTab::Following,
                };
                self.friends_state.selected_index = 0;
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                // Cycle through tabs backwards
                self.friends_state.selected_tab = match self.friends_state.selected_tab {
                    SocialTab::Following => SocialTab::MutualFriends,
                    SocialTab::Followers => SocialTab::Following,
                    SocialTab::MutualFriends => SocialTab::Followers,
                };
                self.friends_state.selected_index = 0;
            }
            KeyCode::Char('/') => {
                // Enter search mode
                self.friends_state.search_mode = true;
                self.friends_state.search_query.clear();
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                // View selected user's profile (handled in main.rs)
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                // Follow/unfollow selected user (handled in main.rs)
            }
            _ => {}
        }
        Ok(())
    }
}
