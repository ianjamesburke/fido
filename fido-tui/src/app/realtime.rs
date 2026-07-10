use anyhow::Result;
use fido_types::{
    ChannelMessageEvent, DirectMessage, DmConversationState, Event, EventEnvelope, Notification,
    NotificationUnreadCount, Post, SortOrder,
};
use uuid::Uuid;

use super::{App, Conversation, DMSelection, DmRequest, PostFilter, Screen, Tab};
use crate::api::RealtimeStatusUpdate;

impl App {
    pub fn apply_realtime_status(&mut self, update: RealtimeStatusUpdate) {
        self.realtime_state.status = update.status;
        self.realtime_state.last_error = update.message;
    }

    pub fn apply_realtime_envelope(&mut self, envelope: EventEnvelope) {
        self.realtime_state.last_event_at = Some(std::time::Instant::now());
        match envelope.event {
            Event::MessageCreated(payload) => {
                self.apply_realtime_channel_message(payload);
            }
            Event::ThreadCreated(post) => self.apply_realtime_thread(post),
            Event::ThreadPendingApproval(post) => {
                self.realtime_state.seen_posts.insert(post.id);
            }
            Event::DmRequestCreated(payload) => {
                self.apply_realtime_dm_request(payload.message, payload.conversation.state);
            }
            Event::DmMessageCreated(message) => self.apply_realtime_dm_message(message, None),
            Event::NotificationCreated(notification) => {
                self.apply_realtime_notification(notification);
            }
        }
    }

    pub async fn refresh_realtime_fallback_visible_surface(&mut self) -> Result<()> {
        self.refresh_notification_counts().await;

        if self.current_screen != Screen::Main {
            return Ok(());
        }

        match self.current_tab {
            Tab::Posts if self.community.is_some() => {
                self.load_posts().await?;
            }
            Tab::DMs => {
                self.load_conversations().await?;
                if self.dms_state.selection.conversation_index().is_some() {
                    self.load_conversation_messages().await?;
                }
            }
            Tab::Chat if self.community.is_some() => {
                self.load_chat().await?;
            }
            Tab::Profile => {
                if self.profile_state.profile.is_some() {
                    self.load_profile().await?;
                }
            }
            Tab::Settings => {}
            Tab::Chat => {}
            Tab::Posts => {}
        }

        Ok(())
    }

    fn apply_realtime_channel_message(&mut self, payload: ChannelMessageEvent) {
        let message_id = payload.message.id;
        if !self.realtime_state.seen_channel_messages.insert(message_id) {
            return;
        }

        self.realtime_state.channel_message_count =
            self.realtime_state.channel_message_count.saturating_add(1);
        self.realtime_state.last_channel_message = Some(payload.clone());

        let chat_is_open = self.current_screen == Screen::Main
            && self.current_tab == Tab::Chat
            && self.community.as_ref().map(|community| community.id) == Some(payload.community_id)
            && self.selected_chat_channel_id() == Some(payload.message.channel_id);
        if !chat_is_open {
            return;
        }

        if self
            .chat_state
            .messages
            .iter()
            .any(|existing| existing.id == message_id)
        {
            return;
        }
        self.chat_state.messages.push(payload.message);
        self.chat_state
            .list_state
            .select(Some(self.chat_state.messages.len().saturating_sub(1)));
    }

    pub async fn refresh_notification_counts(&mut self) {
        match self.api_client.get_notification_unread_counts().await {
            Ok(counts) => {
                self.realtime_state.unread_notifications = counts;
            }
            Err(e) => {
                self.realtime_state.last_error = Some(e.to_string());
            }
        }
    }

    fn apply_realtime_thread(&mut self, post: Post) {
        if self
            .posts_state
            .posts
            .iter()
            .any(|existing| existing.id == post.id)
        {
            self.realtime_state.seen_posts.insert(post.id);
            return;
        }
        if !self.realtime_state.seen_posts.insert(post.id) {
            return;
        }
        if !post.approved || post.parent_post_id.is_some() {
            return;
        }
        if self.community.as_ref().map(|c| c.id) != Some(post.community_id) {
            return;
        }
        if !self.post_matches_current_filter(&post) {
            return;
        }

        let selected_post = self.selected_post_id();
        self.posts_state.posts.push(post);
        self.sort_and_limit_posts();
        self.restore_selected_post(selected_post);
    }

    fn post_matches_current_filter(&self, post: &Post) -> bool {
        match &self.posts_state.current_filter {
            PostFilter::All => true,
            PostFilter::Hashtag(tag) => post
                .hashtags
                .iter()
                .any(|hashtag| hashtag.eq_ignore_ascii_case(tag)),
            PostFilter::User(username) => post.author_username.eq_ignore_ascii_case(username),
            PostFilter::Multi { hashtags, users } => {
                let hashtag_match = hashtags.iter().any(|tag| {
                    post.hashtags
                        .iter()
                        .any(|hashtag| hashtag.eq_ignore_ascii_case(tag))
                });
                let user_match = users
                    .iter()
                    .any(|username| post.author_username.eq_ignore_ascii_case(username));
                hashtag_match || user_match
            }
        }
    }

    fn selected_post_id(&self) -> Option<Uuid> {
        self.posts_state
            .list_state
            .selected()
            .and_then(|index| self.posts_state.posts.get(index))
            .map(|post| post.id)
    }

    fn restore_selected_post(&mut self, selected_post: Option<Uuid>) {
        if let Some(post_id) = selected_post {
            if let Some(index) = self
                .posts_state
                .posts
                .iter()
                .position(|post| post.id == post_id)
            {
                self.posts_state.list_state.select(Some(index));
                return;
            }
        }

        if self.posts_state.posts.is_empty() {
            self.posts_state.list_state.select(None);
        } else if self.posts_state.list_state.selected().is_none() {
            self.posts_state.list_state.select(Some(0));
        }
    }

    fn sort_and_limit_posts(&mut self) {
        let sort_order = self
            .settings_state
            .config
            .as_ref()
            .map(|config| config.sort_order)
            .unwrap_or(SortOrder::Newest);

        match sort_order {
            SortOrder::Newest => self
                .posts_state
                .posts
                .sort_by(|a, b| b.created_at.cmp(&a.created_at)),
            SortOrder::Popular => self.posts_state.posts.sort_by(|a, b| {
                b.upvotes
                    .cmp(&a.upvotes)
                    .then_with(|| b.created_at.cmp(&a.created_at))
            }),
            SortOrder::Controversial => self.posts_state.posts.sort_by(|a, b| {
                let a_score = (a.upvotes - a.downvotes).abs();
                let b_score = (b.upvotes - b.downvotes).abs();
                a_score
                    .cmp(&b_score)
                    .then_with(|| b.created_at.cmp(&a.created_at))
            }),
        }

        let max_posts = self
            .settings_state
            .config
            .as_ref()
            .map(|config| config.max_posts_display.max(1) as usize)
            .unwrap_or(25);
        self.posts_state.posts.truncate(max_posts);
    }

    fn apply_realtime_dm_request(&mut self, message: DirectMessage, state: DmConversationState) {
        let Some((other_user_id, other_username, from_me)) = self.dm_counterparty(&message) else {
            return;
        };

        if !from_me
            && !self
                .dms_state
                .pending_requests
                .iter()
                .any(|request| request.from_user_id == other_user_id)
        {
            self.dms_state.pending_requests.push(DmRequest {
                from_user_id: other_user_id,
                from_username: other_username,
            });
        }

        self.apply_realtime_dm_message(message, Some(state));
    }

    fn apply_realtime_dm_message(
        &mut self,
        message: DirectMessage,
        state_hint: Option<DmConversationState>,
    ) {
        if self
            .dms_state
            .messages
            .iter()
            .any(|existing| existing.id == message.id)
        {
            self.realtime_state.seen_dm_messages.insert(message.id);
            return;
        }
        if !self.realtime_state.seen_dm_messages.insert(message.id) {
            return;
        }

        let Some((other_user_id, other_username, from_me)) = self.dm_counterparty(&message) else {
            return;
        };

        let is_open = self.current_screen == Screen::Main
            && self.current_tab == Tab::DMs
            && self.dms_state.current_conversation_user == Some(other_user_id);
        let is_incoming = !from_me;

        if is_open {
            let mut visible_message = message.clone();
            visible_message.is_read = true;
            self.dms_state.messages.push(visible_message);
            self.dms_state.unread_counts.insert(other_user_id, 0);
            if is_incoming {
                self.dms_state.needs_message_load = true;
            }
        } else if is_incoming {
            let count = self
                .dms_state
                .unread_counts
                .entry(other_user_id)
                .or_insert(0);
            *count = count.saturating_add(1);
        }

        let pending_incoming =
            is_incoming && matches!(state_hint, Some(DmConversationState::Pending));
        if !pending_incoming {
            self.upsert_dm_conversation(
                other_user_id,
                other_username,
                message.content,
                state_hint.unwrap_or(DmConversationState::Accepted),
                from_me,
            );
        }

        self.sync_conversation_unread_count(other_user_id);
    }

    fn dm_counterparty(&self, message: &DirectMessage) -> Option<(Uuid, String, bool)> {
        let current_user = self.auth_state.current_user.as_ref()?;
        if message.from_user_id == current_user.id {
            Some((message.to_user_id, message.to_username.clone(), true))
        } else if message.to_user_id == current_user.id {
            Some((message.from_user_id, message.from_username.clone(), false))
        } else {
            None
        }
    }

    fn upsert_dm_conversation(
        &mut self,
        other_user_id: Uuid,
        other_username: String,
        last_message: String,
        state: DmConversationState,
        initiated_by_me: bool,
    ) {
        if let Some(conversation) = self
            .dms_state
            .conversations
            .iter_mut()
            .find(|conversation| conversation.other_user_id == other_user_id)
        {
            conversation.last_message = last_message;
            conversation.state = state;
            return;
        }

        let unread_count = self
            .dms_state
            .unread_counts
            .get(&other_user_id)
            .copied()
            .unwrap_or_default() as i32;
        self.dms_state.conversations.insert(
            0,
            Conversation {
                other_user_id,
                other_username,
                last_message,
                unread_count,
                state,
                initiated_by_me,
            },
        );
        self.dms_state.conversations_loaded = true;
        if self.dms_state.selection == DMSelection::NewConversation {
            self.dms_state.selection = DMSelection::Conversation(0);
        }
    }

    fn sync_conversation_unread_count(&mut self, other_user_id: Uuid) {
        let unread_count = self
            .dms_state
            .unread_counts
            .get(&other_user_id)
            .copied()
            .unwrap_or_default() as i32;
        if let Some(conversation) = self
            .dms_state
            .conversations
            .iter_mut()
            .find(|conversation| conversation.other_user_id == other_user_id)
        {
            conversation.unread_count = unread_count;
        }
    }

    fn apply_realtime_notification(&mut self, notification: Notification) {
        if notification.read
            || !self
                .realtime_state
                .seen_notifications
                .insert(notification.id)
        {
            return;
        }

        if let Some(count) = self
            .realtime_state
            .unread_notifications
            .iter_mut()
            .find(|count| {
                count.subject_type == notification.subject_type
                    && count.subject_id == notification.subject_id
            })
        {
            count.count += 1;
        } else {
            self.realtime_state
                .unread_notifications
                .push(NotificationUnreadCount {
                    subject_type: notification.subject_type,
                    subject_id: notification.subject_id,
                    count: 1,
                });
        }
    }
}
