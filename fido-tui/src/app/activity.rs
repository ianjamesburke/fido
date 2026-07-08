use crate::app::{App, FeedEntry};

impl App {
    /// URL of the selected activity item, if the selection is an activity row.
    pub fn selected_activity_url(&self) -> Option<String> {
        match self.posts_state.selected_feed_entry()? {
            FeedEntry::Activity(i) => self
                .posts_state
                .activity_items
                .get(i)
                .map(|a| a.html_url.clone()),
            FeedEntry::Post(_) => None,
        }
    }

    /// Fetch repo activity for the current community. Called by the event
    /// loop after the board has rendered (activity_pending_load pattern) —
    /// never on the render path.
    pub async fn load_activity(&mut self) {
        let Some(community_id) = self.current_community_id() else {
            self.posts_state.activity_loading = false;
            return;
        };
        match self.api_client.get_community_activity(community_id).await {
            Ok(response) => {
                self.posts_state.activity_items = response.items;
                self.posts_state.activity_error = None;
            }
            Err(e) => {
                self.posts_state.activity_items.clear();
                self.posts_state.activity_error = Some(format!("repo activity unavailable: {}", e));
            }
        }
        self.posts_state.activity_loading = false;
        self.posts_state.rebuild_feed();
    }

    pub fn clear_activity(&mut self) {
        self.posts_state.activity_items.clear();
        self.posts_state.activity_error = None;
        self.posts_state.activity_loading = false;
        self.posts_state.activity_pending_load = false;
        self.clear_approval_queue();
        self.posts_state.rebuild_feed();
    }
}
