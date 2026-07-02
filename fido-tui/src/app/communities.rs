//! Community context: the launch directory decides the community.
//!
//! Repo mode (launched inside a GitHub repo): join that repo's community and
//! scope the board to it. Home mode (anywhere else): list joined communities
//! and let the user open one.

use anyhow::Result;

use crate::api::CommunityViewResponse;

use super::{categorize_error, App};

impl App {
    /// Entry point after login or session restore.
    pub async fn init_community_context(&mut self) -> Result<()> {
        self.community = None;
        self.community_error = None;

        if let Some(repo) = self.launch_repo.clone() {
            self.enter_repo_community(&repo.owner, &repo.name).await;
        } else {
            self.load_home_communities().await;
        }
        Ok(())
    }

    /// Repo mode: join (lazily creating) the launch repo's community.
    async fn enter_repo_community(&mut self, owner: &str, name: &str) {
        match self.api_client.join_community(owner, name).await {
            Ok(view) => {
                self.apply_community_view(view);
            }
            Err(e) => {
                log::error!("Failed to join community {}/{}: {}", owner, name, e);
                self.community_error = Some(format!(
                    "Could not open the {}/{} community: {}",
                    owner,
                    name,
                    categorize_error(&e.to_string())
                ));
            }
        }
    }

    /// Home mode: load the joined-communities list.
    pub async fn load_home_communities(&mut self) {
        self.home_state.loading = true;
        self.home_state.error = None;

        match self.api_client.list_communities().await {
            Ok(communities) => {
                self.home_state.communities = communities;
                self.home_state.loaded = true;
                if self.home_state.communities.is_empty() {
                    self.home_state.list_state.select(None);
                } else {
                    self.home_state.list_state.select(Some(0));
                }
            }
            Err(e) => {
                log::error!("Failed to list joined communities: {}", e);
                self.home_state.error = Some(categorize_error(&e.to_string()));
            }
        }

        self.home_state.loading = false;
    }

    /// Home mode: whether the joined-communities list is the active Posts view.
    pub fn is_home_list_active(&self) -> bool {
        self.launch_repo.is_none() && self.community.is_none()
    }

    /// Home mode: open the selected community's board.
    pub async fn open_home_selection(&mut self) -> Result<()> {
        let Some(selected) = self.home_state.selected() else {
            return Ok(());
        };
        let community_id = selected.community.id;

        match self.api_client.get_community(community_id).await {
            Ok(view) => {
                self.apply_community_view(view);
                self.load_posts().await?;
            }
            Err(e) => {
                log::error!("Failed to open community {}: {}", community_id, e);
                self.home_state.error = Some(categorize_error(&e.to_string()));
            }
        }
        Ok(())
    }

    /// Home mode: leave the open board and return to the communities list.
    pub fn close_community(&mut self) {
        self.community = None;
        self.show_community_modal = false;
        self.posts_state.posts.clear();
        self.posts_state.list_state.select(None);
        self.posts_state.error = None;
    }

    /// Repo mode: retry after a failed join (server down, repo unresolvable).
    pub async fn retry_community(&mut self) -> Result<()> {
        self.init_community_context().await?;
        if self.community.is_some() {
            self.load_posts().await?;
        }
        Ok(())
    }

    /// Claim admin of the current community; the server verifies GitHub
    /// admin/maintain permission on the repo.
    pub async fn claim_current_community(&mut self) -> Result<()> {
        let Some(community_id) = self.community.as_ref().map(|c| c.id) else {
            return Ok(());
        };

        match self.api_client.claim_community(community_id).await {
            Ok(view) => {
                self.apply_community_view(view);
                self.posts_state.message = Some((
                    "Community claimed - you are now admin".to_string(),
                    std::time::Instant::now(),
                ));
            }
            Err(e) => {
                log::error!("Failed to claim community {}: {}", community_id, e);
                self.posts_state.message = Some((
                    format!("Claim failed: {}", categorize_error(&e.to_string())),
                    std::time::Instant::now(),
                ));
            }
        }
        Ok(())
    }

    fn apply_community_view(&mut self, view: CommunityViewResponse) {
        self.community = Some(crate::app::state::CommunityContext {
            id: view.community.id,
            owner: view.community.owner,
            name: view.community.name,
            role: view.membership.map(|m| m.role),
            member_count: view.member_count,
            claimed: view.community.claimed_by.is_some(),
        });
        self.community_error = None;
    }

    /// Home list navigation
    pub fn next_home_community(&mut self) {
        move_home_selection(&mut self.home_state, 1);
    }

    pub fn previous_home_community(&mut self) {
        move_home_selection(&mut self.home_state, -1);
    }
}

fn move_home_selection(home: &mut crate::app::state::HomeState, delta: i64) {
    let len = home.communities.len();
    if len == 0 {
        home.list_state.select(None);
        return;
    }
    let current = home.list_state.selected().unwrap_or(0) as i64;
    let next = (current + delta).rem_euclid(len as i64) as usize;
    home.list_state.select(Some(next));
}
