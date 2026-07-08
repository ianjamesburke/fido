use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Style;
use tui_textarea::{Input, TextArea};
use uuid::Uuid;

use super::{categorize_error, App, InputMode};

impl App {
    pub fn clear_chat(&mut self) {
        self.chat_state.channels.clear();
        self.chat_state.selected_channel_index = 0;
        self.chat_state.messages.clear();
        self.chat_state.list_state.select(None);
        self.chat_state.loading = false;
        self.chat_state.loaded = false;
        self.chat_state.loading_older = false;
        self.chat_state.pending_older_load = false;
        self.chat_state.at_history_start = false;
        self.chat_state.error = None;
        self.clear_chat_message();
    }

    pub fn selected_chat_channel_id(&self) -> Option<Uuid> {
        self.chat_state.selected_channel().map(|channel| channel.id)
    }

    pub async fn load_chat(&mut self) -> Result<()> {
        let Some(community_id) = self.current_community_id() else {
            self.chat_state.error = Some("Open a community before using chat.".to_string());
            return Ok(());
        };

        self.chat_state.loading = true;
        self.chat_state.error = None;

        match self.api_client.list_community_channels(community_id).await {
            Ok(channels) => {
                self.chat_state.channels = channels;
                self.chat_state.selected_channel_index = 0;
                self.chat_state.loaded = true;
                self.chat_state.at_history_start = false;
                self.load_selected_channel_messages().await?;
            }
            Err(e) => {
                self.chat_state.error = Some(categorize_error(&e.to_string()));
            }
        }

        self.chat_state.loading = false;
        Ok(())
    }

    pub async fn load_selected_channel_messages(&mut self) -> Result<()> {
        let Some(channel_id) = self.selected_chat_channel_id() else {
            self.chat_state.messages.clear();
            self.chat_state.list_state.select(None);
            return Ok(());
        };

        match self
            .api_client
            .get_channel_messages(channel_id, None, None, 50)
            .await
        {
            Ok(messages) => {
                self.chat_state.messages = messages;
                self.realtime_state
                    .seen_channel_messages
                    .extend(self.chat_state.messages.iter().map(|message| message.id));
                self.select_last_chat_message();
            }
            Err(e) => {
                self.chat_state.error = Some(categorize_error(&e.to_string()));
            }
        }

        Ok(())
    }

    pub async fn load_older_channel_messages(&mut self) -> Result<()> {
        if self.chat_state.loading_older || self.chat_state.at_history_start {
            return Ok(());
        }
        let Some(channel_id) = self.selected_chat_channel_id() else {
            return Ok(());
        };
        let Some(before) = self.chat_state.messages.first().map(|message| message.id) else {
            return Ok(());
        };

        self.chat_state.loading_older = true;
        match self
            .api_client
            .get_channel_messages(channel_id, Some(before), None, 50)
            .await
        {
            Ok(mut older) => {
                if older.is_empty() {
                    self.chat_state.at_history_start = true;
                } else {
                    older.append(&mut self.chat_state.messages);
                    self.chat_state.messages = older;
                    self.chat_state.list_state.select(Some(0));
                }
            }
            Err(e) => {
                self.chat_state.error = Some(categorize_error(&e.to_string()));
            }
        }
        self.chat_state.loading_older = false;
        Ok(())
    }

    pub async fn send_channel_message(&mut self) -> Result<()> {
        let Some(channel_id) = self.selected_chat_channel_id() else {
            self.chat_state.error = Some("No chat channel is available.".to_string());
            return Ok(());
        };

        let content = self.get_chat_message_content();
        let trimmed = content.trim();
        if trimmed.is_empty() {
            self.chat_state.error = Some("Cannot send an empty message.".to_string());
            return Ok(());
        }

        match self
            .api_client
            .send_channel_message(channel_id, trimmed.to_string())
            .await
        {
            Ok(message) => {
                self.apply_chat_message(message);
                self.clear_chat_message();
                self.input_mode = InputMode::Navigation;
                self.chat_state.error = None;
            }
            Err(e) => {
                self.chat_state.error = Some(categorize_error(&e.to_string()));
            }
        }

        Ok(())
    }

    pub fn apply_chat_message(&mut self, message: fido_types::Message) {
        if self
            .chat_state
            .messages
            .iter()
            .any(|existing| existing.id == message.id)
        {
            self.realtime_state.seen_channel_messages.insert(message.id);
            return;
        }
        self.realtime_state.seen_channel_messages.insert(message.id);
        self.chat_state.messages.push(message);
        self.select_last_chat_message();
    }

    pub fn handle_chat_keys(&mut self, key: KeyEvent) -> Result<()> {
        match self.input_mode {
            InputMode::Navigation => match key.code {
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.next_chat_message();
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.previous_chat_message();
                }
                KeyCode::Enter => {
                    self.input_mode = InputMode::Typing;
                }
                _ => {
                    self.input_mode = InputMode::Typing;
                    self.handle_chat_input(key);
                }
            },
            InputMode::Typing => match key.code {
                KeyCode::Esc => {
                    self.clear_chat_message();
                    self.input_mode = InputMode::Navigation;
                }
                KeyCode::Enter => {}
                _ => {
                    self.handle_chat_input(key);
                    if self.is_chat_message_empty() && key.code == KeyCode::Backspace {
                        self.input_mode = InputMode::Navigation;
                    }
                }
            },
        }
        Ok(())
    }

    pub fn get_chat_message_content(&self) -> String {
        self.chat_state.message_textarea.lines().join("\n")
    }

    pub fn clear_chat_message(&mut self) {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        textarea.set_style(Style::default());
        textarea.set_hard_tab_indent(true);
        self.chat_state.message_textarea = textarea;
    }

    fn is_chat_message_empty(&self) -> bool {
        self.get_chat_message_content().trim().is_empty()
    }

    fn handle_chat_input(&mut self, key: KeyEvent) {
        let input = Input::from(crossterm::event::Event::Key(key));
        self.chat_state.message_textarea.input(input);
        crate::text_wrapper::wrap_textarea_if_needed(
            &mut self.chat_state.message_textarea,
            crate::text_wrapper::WrapConfig::DM_PANEL,
        );
    }

    fn next_chat_message(&mut self) {
        let len = self.chat_state.messages.len();
        if len == 0 {
            self.chat_state.list_state.select(None);
            return;
        }
        let current = self.chat_state.list_state.selected().unwrap_or(len - 1);
        self.chat_state
            .list_state
            .select(Some((current + 1).min(len - 1)));
    }

    fn previous_chat_message(&mut self) {
        let len = self.chat_state.messages.len();
        if len == 0 {
            self.chat_state.list_state.select(None);
            return;
        }
        let current = self.chat_state.list_state.selected().unwrap_or(len - 1);
        if current == 0 {
            self.chat_state.pending_older_load = true;
        }
        self.chat_state
            .list_state
            .select(Some(current.saturating_sub(1)));
    }

    fn select_last_chat_message(&mut self) {
        if self.chat_state.messages.is_empty() {
            self.chat_state.list_state.select(None);
        } else {
            self.chat_state
                .list_state
                .select(Some(self.chat_state.messages.len() - 1));
        }
    }
}
