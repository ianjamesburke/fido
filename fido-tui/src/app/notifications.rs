use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use uuid::Uuid;

use super::{categorize_error, App, DMSelection, InputMode, Screen, Tab};

impl App {
    pub fn total_unread_notifications(&self) -> i64 {
        self.realtime_state
            .unread_notifications
            .iter()
            .map(|count| count.count)
            .sum()
    }

    pub async fn open_notifications_panel(&mut self) -> Result<()> {
        self.notifications_state.show = true;
        self.input_mode = InputMode::Navigation;
        self.load_notifications().await
    }

    pub fn close_notifications_panel(&mut self) {
        self.notifications_state.show = false;
        self.notifications_state.error = None;
    }

    pub async fn load_notifications(&mut self) -> Result<()> {
        self.notifications_state.loading = true;
        self.notifications_state.error = None;

        match self.api_client.get_notifications(50, 0).await {
            Ok(notifications) => {
                self.notifications_state.notifications = notifications;
                if self.notifications_state.notifications.is_empty() {
                    self.notifications_state.selected_index = 0;
                } else {
                    self.notifications_state.selected_index = self
                        .notifications_state
                        .selected_index
                        .min(self.notifications_state.notifications.len() - 1);
                }
                self.notifications_state.loaded = true;
            }
            Err(e) => {
                self.notifications_state.error = Some(categorize_error(&e.to_string()));
            }
        }

        self.notifications_state.loading = false;
        Ok(())
    }

    pub fn handle_notifications_panel_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('v') | KeyCode::Char('V') => {
                self.close_notifications_panel();
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if !self.notifications_state.notifications.is_empty() {
                    self.notifications_state.selected_index =
                        (self.notifications_state.selected_index + 1)
                            .min(self.notifications_state.notifications.len() - 1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.notifications_state.selected_index =
                    self.notifications_state.selected_index.saturating_sub(1);
            }
            KeyCode::Enter | KeyCode::Char('a') | KeyCode::Char('A') => {}
            _ => {}
        }
        Ok(())
    }

    pub async fn mark_all_notifications_read(&mut self) -> Result<()> {
        self.api_client.mark_notifications_read(None, true).await?;
        for notification in &mut self.notifications_state.notifications {
            notification.read = true;
        }
        self.realtime_state.unread_notifications.clear();
        self.refresh_notification_counts().await;
        Ok(())
    }

    pub async fn open_selected_notification(&mut self) -> Result<()> {
        let Some(notification) = self
            .notifications_state
            .notifications
            .get(self.notifications_state.selected_index)
            .cloned()
        else {
            return Ok(());
        };

        self.api_client
            .mark_notifications_read(Some(notification.id), false)
            .await?;
        if let Some(item) = self
            .notifications_state
            .notifications
            .iter_mut()
            .find(|item| item.id == notification.id)
        {
            item.read = true;
        }
        self.refresh_notification_counts().await;

        match notification.subject_type.as_str() {
            "community" => {
                self.open_notification_community(&notification.subject_id)
                    .await?
            }
            "dm_conversation" => self.open_notification_dm(notification.actor_id).await?,
            _ => {
                self.notifications_state.error = Some(format!(
                    "Unknown notification source: {}",
                    notification.subject_type
                ));
            }
        }

        Ok(())
    }

    async fn open_notification_community(&mut self, subject_id: &str) -> Result<()> {
        let community_id = Uuid::parse_str(subject_id)
            .map_err(|e| anyhow::anyhow!("Invalid notification community id: {}", e))?;
        match self.api_client.get_community(community_id).await {
            Ok(view) => {
                self.clear_chat();
                self.apply_community_view(view);
                self.current_screen = Screen::Main;
                self.current_tab = Tab::Posts;
                self.close_notifications_panel();
                self.load_posts().await?;
            }
            Err(e) => {
                self.notifications_state.error = Some(categorize_error(&e.to_string()));
            }
        }
        Ok(())
    }

    async fn open_notification_dm(&mut self, actor_id: Uuid) -> Result<()> {
        self.current_screen = Screen::Main;
        self.current_tab = Tab::DMs;
        self.close_notifications_panel();
        self.load_conversations().await?;

        if let Some(index) = self
            .dms_state
            .pending_requests
            .iter()
            .position(|request| request.from_user_id == actor_id)
        {
            self.dms_state.selection = DMSelection::Request(index);
            self.dms_state.messages.clear();
            self.dms_state.current_conversation_user = None;
            return Ok(());
        }

        if let Some(index) = self
            .dms_state
            .conversations
            .iter()
            .position(|conversation| conversation.other_user_id == actor_id)
        {
            self.dms_state.selection = DMSelection::Conversation(index);
            self.dms_state.needs_message_load = true;
        }

        Ok(())
    }
}
