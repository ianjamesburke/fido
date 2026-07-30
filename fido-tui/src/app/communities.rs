//! Community context: the launch directory decides the community.
//!
//! Repo mode (launched inside a GitHub repo): join that repo's community and
//! scope the board to it. Home mode (anywhere else): list joined communities
//! and let the user open one.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::api::CommunityViewResponse;

use super::{categorize_error, App};

impl App {
    /// The id of the community whose board is currently shown, if any.
    pub fn current_community_id(&self) -> Option<uuid::Uuid> {
        self.community.as_ref().map(|c| c.id)
    }

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
                self.clear_chat();
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

    /// Whether the current board was reached by browsing away from the repo
    /// that selected the initial community at launch.
    pub fn is_away_from_launch_community(&self) -> bool {
        let Some(repo) = &self.launch_repo else {
            return false;
        };
        let Some(community) = &self.community else {
            return false;
        };

        community.owner != repo.owner || community.name != repo.name
    }

    /// Return a repo-launched session to its initial board after browsing.
    pub async fn return_to_launch_community(&mut self) -> Result<()> {
        if !self.is_away_from_launch_community() {
            return Ok(());
        }

        let Some(repo) = self.launch_repo.clone() else {
            return Ok(());
        };
        self.enter_repo_community(&repo.owner, &repo.name).await;

        if self.is_away_from_launch_community() {
            return Ok(());
        }

        self.load_posts().await?;
        self.posts_state.message = Some((
            format!("Returned to {}/{}", repo.owner, repo.name),
            std::time::Instant::now(),
        ));
        Ok(())
    }

    /// Home mode: open the selected community's board.
    pub async fn open_home_selection(&mut self) -> Result<()> {
        let Some(selected) = self.home_state.selected() else {
            return Ok(());
        };
        let community_id = selected.community.id;

        match self.api_client.get_community(community_id).await {
            Ok(view) => {
                self.clear_chat();
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
        self.clear_chat();
    }

    /// Repo mode: retry after a failed join (server down, repo unresolvable).
    pub async fn retry_community(&mut self) -> Result<()> {
        self.init_community_context().await?;
        if self.community.is_some() {
            self.load_posts().await?;
        }
        Ok(())
    }

    /// Claim admin of the current community; the server verifies GitHub admin
    /// permission on the repo.
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

    pub(crate) fn apply_community_view(&mut self, view: CommunityViewResponse) {
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

    /// Open the repo browser (starred, owned, and contributed), loading it on first use.
    pub async fn open_community_browser(&mut self) -> Result<()> {
        self.community_browser_state.show = true;
        self.community_browser_state.error = None;
        if !self.community_browser_state.loaded {
            self.load_community_browser().await;
        }
        Ok(())
    }

    pub fn close_community_browser(&mut self) {
        self.community_browser_state.show = false;
        self.community_browser_state.error = None;
        self.community_browser_state.message = None;
        self.community_browser_state.filter.clear();
        self.community_browser_state.apply_filter();
    }

    pub async fn load_community_browser(&mut self) {
        self.community_browser_state.loading = true;
        self.community_browser_state.error = None;
        self.community_browser_state.message = None;

        match self.api_client.browse_communities().await {
            Ok(repos) => {
                self.community_browser_state.repos = repos;
                self.community_browser_state.loaded = true;
                // Reloading keeps whatever the user has typed so far.
                self.community_browser_state.apply_filter();
            }
            Err(e) => {
                log::error!("Failed to browse communities: {}", e);
                self.community_browser_state.error = Some(categorize_error(&e.to_string()));
            }
        }

        self.community_browser_state.loading = false;
    }

    pub async fn open_or_join_browser_selection(&mut self) -> Result<()> {
        let Some(selected) = self.community_browser_state.selected() else {
            return Ok(());
        };

        if selected.private {
            self.community_browser_state.error =
                Some("Private repositories cannot create Fido communities yet".to_string());
            return Ok(());
        }

        let owner = selected.owner.clone();
        let name = selected.name.clone();
        let existing_community = selected
            .community
            .as_ref()
            .filter(|_| selected.membership.is_some())
            .map(|community| community.id);

        self.community_browser_state.joining = true;
        self.community_browser_state.error = None;
        self.community_browser_state.message = None;

        let result = if let Some(community_id) = existing_community {
            self.api_client.get_community(community_id).await
        } else {
            self.api_client.join_community(&owner, &name).await
        };

        match result {
            Ok(view) => {
                self.clear_chat();
                self.apply_community_view(view);
                self.current_tab = crate::app::Tab::Posts;
                self.community_browser_state.show = false;
                self.community_browser_state.loaded = false;
                self.load_home_communities().await;
                self.load_posts().await?;
            }
            Err(e) => {
                log::error!("Failed to open or join {}/{}: {}", owner, name, e);
                self.community_browser_state.error = Some(categorize_error(&e.to_string()));
            }
        }

        self.community_browser_state.joining = false;
        Ok(())
    }

    pub fn next_browser_repo(&mut self) {
        move_browser_selection(&mut self.community_browser_state, 1);
    }

    pub fn previous_browser_repo(&mut self) {
        move_browser_selection(&mut self.community_browser_state, -1);
    }

    /// The browser is a text-input surface: plain characters type into the
    /// fuzzy filter, so navigation is arrow-keys only and reload moved to
    /// Ctrl+R (handled in the event loop, which owns the async reload).
    pub fn handle_community_browser_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Esc clears an active filter first, and only closes once empty.
            KeyCode::Esc => {
                if self.community_browser_state.filter.is_empty() {
                    self.close_community_browser();
                } else {
                    self.community_browser_state.filter.clear();
                    self.community_browser_state.apply_filter();
                }
            }
            KeyCode::Down => self.next_browser_repo(),
            KeyCode::Up => self.previous_browser_repo(),
            KeyCode::Backspace => {
                self.community_browser_state.filter.pop();
                self.community_browser_state.apply_filter();
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.community_browser_state.filter.push(c);
                self.community_browser_state.apply_filter();
            }
            _ => {}
        }
        Ok(())
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

fn move_browser_selection(browser: &mut crate::app::state::CommunityBrowserState, delta: i64) {
    // Navigation walks the filtered rows, not the full repo list.
    let len = browser.visible.len();
    if len == 0 {
        browser.list_state.select(None);
        return;
    }
    let current = browser.list_state.selected().unwrap_or(0) as i64;
    let next = (current + delta).rem_euclid(len as i64) as usize;
    browser.list_state.select(Some(next));
}
