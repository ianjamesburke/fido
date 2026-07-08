use crate::app::{App, DMSelection, FilterTab, InputMode, PostFilter, Screen, Tab};
use crate::auth::{self, AuthFlow, RestoredSession};
use crate::{log_key_event, ui};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use uuid::Uuid;

const REALTIME_FALLBACK_POLL_INTERVAL: Duration = Duration::from_secs(10);

pub struct EventLoop {
    modal_tracker: ModalStateTracker,
    last_tab: Tab,
    last_dm_selection: DMSelection,
    last_terminal_size: (u16, u16),
    last_device_poll: Instant,
    startup_started: bool,
    startup_data_load_pending: bool,
    session_restore_task: Option<JoinHandle<Result<Option<RestoredSession>, String>>>,
    update_check_task: Option<JoinHandle<Option<String>>>,
    realtime_task: Option<JoinHandle<()>>,
    realtime_rx: Option<mpsc::Receiver<crate::api::RealtimeClientEvent>>,
    realtime_user_id: Option<Uuid>,
    last_realtime_fallback_poll: Instant,
    realtime_refetch_pending: bool,
    is_web_mode: bool,
}

impl EventLoop {
    pub fn new(is_web_mode: bool) -> Self {
        Self {
            modal_tracker: ModalStateTracker::new(),
            last_tab: Tab::Posts, // Default starting tab
            last_dm_selection: DMSelection::NewConversation,
            last_terminal_size: (0, 0),
            last_device_poll: Instant::now(),
            startup_started: false,
            startup_data_load_pending: false,
            session_restore_task: None,
            update_check_task: None,
            realtime_task: None,
            realtime_rx: None,
            realtime_user_id: None,
            last_realtime_fallback_poll: Instant::now(),
            realtime_refetch_pending: false,
            is_web_mode,
        }
    }

    pub async fn run(
        &mut self,
        app: &mut App,
        auth_flow: &mut AuthFlow,
        tui: &mut crate::terminal::Terminal,
    ) -> Result<()> {
        while app.running {
            // Handle GitHub Device Flow polling
            self.handle_github_device_flow(app, auth_flow).await?;

            // Clear expired messages
            app.clear_expired_messages();

            // Drain pushed events before rendering this frame.
            self.handle_realtime(app).await?;

            // Render UI
            self.render_ui(app, tui)?;

            // Process events
            self.process_events(app, auth_flow).await?;

            // Start and drain startup network work only after at least one frame.
            self.handle_startup_tasks(app, auth_flow).await?;

            // Handle pending loads
            self.handle_pending_loads(app).await?;
            app.flush_finished_vote_tasks().await;

            // Check modal state changes and load data as needed
            self.modal_tracker.check_and_load(app).await?;

            // Handle tab changes and data loading after render/event processing
            self.handle_tab_changes(app).await?;

            // Handle DM conversation changes
            self.handle_dm_conversation_changes(app).await?;
        }

        self.stop_realtime();
        Ok(())
    }

    async fn handle_realtime(&mut self, app: &mut App) -> Result<()> {
        self.sync_realtime_lifecycle(app);
        self.drain_realtime_events(app);
        self.handle_realtime_polling_fallback(app).await
    }

    fn sync_realtime_lifecycle(&mut self, app: &mut App) {
        let Some(current_user) = app.auth_state.current_user.as_ref() else {
            self.stop_realtime();
            app.apply_realtime_status(crate::api::RealtimeConnectionStatus::Disabled.into());
            return;
        };
        if app.current_screen != Screen::Main {
            self.stop_realtime();
            app.apply_realtime_status(crate::api::RealtimeConnectionStatus::Disabled.into());
            return;
        }
        if app.api_client.session_token().is_none() {
            self.stop_realtime();
            app.apply_realtime_status(crate::api::RealtimeConnectionStatus::Disabled.into());
            return;
        }

        if self.realtime_user_id != Some(current_user.id) {
            self.stop_realtime();
        }

        if self.realtime_task.is_some() {
            return;
        }
        if app.realtime_state.status == crate::api::RealtimeConnectionStatus::Unauthorized {
            return;
        }

        let (handle, rx) = crate::api::spawn_realtime_task(app.api_client.clone());
        self.realtime_task = Some(handle);
        self.realtime_rx = Some(rx);
        self.realtime_user_id = Some(current_user.id);
        self.realtime_refetch_pending = false;
    }

    fn stop_realtime(&mut self) {
        if let Some(handle) = self.realtime_task.take() {
            handle.abort();
        }
        self.realtime_rx = None;
        self.realtime_user_id = None;
        self.realtime_refetch_pending = false;
    }

    fn drain_realtime_events(&mut self, app: &mut App) {
        loop {
            let event = match self.realtime_rx.as_mut() {
                Some(rx) => rx.try_recv(),
                None => return,
            };

            match event {
                Ok(crate::api::RealtimeClientEvent::Status(update)) => {
                    app.apply_realtime_status(update);
                }
                Ok(crate::api::RealtimeClientEvent::Event(envelope)) => {
                    app.apply_realtime_envelope(*envelope);
                }
                Ok(crate::api::RealtimeClientEvent::RefetchRequired) => {
                    self.realtime_refetch_pending = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => return,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.realtime_rx = None;
                    self.realtime_task = None;
                    return;
                }
            }
        }
    }

    async fn handle_realtime_polling_fallback(&mut self, app: &mut App) -> Result<()> {
        if !app.realtime_state.is_polling_fallback_active() {
            return Ok(());
        }

        let poll_due =
            self.last_realtime_fallback_poll.elapsed() >= REALTIME_FALLBACK_POLL_INTERVAL;
        if !poll_due && !self.realtime_refetch_pending {
            return Ok(());
        }

        self.realtime_refetch_pending = false;
        self.last_realtime_fallback_poll = Instant::now();
        if let Err(e) = app.refresh_realtime_fallback_visible_surface().await {
            app.realtime_state.last_error = Some(e.to_string());
        }
        Ok(())
    }

    async fn handle_startup_tasks(
        &mut self,
        app: &mut App,
        auth_flow: &mut AuthFlow,
    ) -> Result<()> {
        if !self.startup_started {
            self.startup_started = true;

            let api_client = app.api_client.clone();
            self.session_restore_task = Some(tokio::spawn(async move {
                auth::restore_existing_session(api_client)
                    .await
                    .map_err(|e| e.to_string())
            }));

            if !self.is_web_mode {
                self.update_check_task = Some(tokio::spawn(crate::check_for_updates()));
            }
        }

        if self
            .update_check_task
            .as_ref()
            .map(|task| task.is_finished())
            .unwrap_or(false)
        {
            let handle = self.update_check_task.take().unwrap();
            if let Ok(Some(latest_version)) = handle.await {
                app.update_available = Some(latest_version);
            }
        }

        if self
            .session_restore_task
            .as_ref()
            .map(|task| task.is_finished())
            .unwrap_or(false)
        {
            let handle = self.session_restore_task.take().unwrap();
            match handle.await {
                Ok(Ok(Some(restored))) if app.current_screen == Screen::Auth => {
                    log::info!("Restored session for user: {}", restored.user.username);
                    auth_flow
                        .api_client_mut()
                        .set_session_token(Some(restored.session_token.clone()));
                    app.api_client = restored.api_client;
                    app.auth_state.current_user = Some(restored.user);
                    app.auth_state.loading = false;
                    app.auth_state.error = None;
                    app.current_screen = Screen::Main;
                    self.startup_data_load_pending = true;
                    return Ok(());
                }
                Ok(Ok(Some(restored))) => {
                    log::debug!(
                        "Ignoring restored startup session for {} after screen changed",
                        restored.user.username
                    );
                }
                Ok(Ok(None)) => {
                    log::info!("No valid session found, showing authentication screen");
                    app.auth_state.loading = false;
                }
                Ok(Err(e)) => {
                    log::warn!("Startup session restore failed: {}", e);
                    app.auth_state.loading = false;
                }
                Err(e) => {
                    log::warn!("Startup session restore task failed: {}", e);
                    app.auth_state.loading = false;
                }
            }
        }

        if self.startup_data_load_pending && app.current_screen == Screen::Main {
            self.startup_data_load_pending = false;
            let _ = app.load_settings().await;
            app.load_filter_preference();
            let _ = app.init_community_context().await;
            let _ = app.load_posts().await;
        }

        Ok(())
    }

    async fn handle_github_device_flow(
        &mut self,
        app: &mut App,
        auth_flow: &mut AuthFlow,
    ) -> Result<()> {
        if !app.auth_state.github_auth_in_progress {
            return Ok(());
        }

        // Check for timeout (15 minutes)
        if let Some(start_time) = app.auth_state.github_auth_start_time {
            if start_time.elapsed() > Duration::from_secs(900) {
                log::warn!("GitHub Device Flow timeout after 15 minutes");
                app.auth_state.error =
                    Some("Device authorization timeout: Please try again.".to_string());
                self.reset_github_auth_state(app);
                return Ok(());
            }
        }

        // Only poll at the specified interval
        let poll_interval = app.auth_state.github_poll_interval.unwrap_or(5);
        if self.last_device_poll.elapsed() < Duration::from_secs(poll_interval as u64) {
            return Ok(());
        }

        if let Some(device_code) = app.auth_state.github_device_code.as_deref() {
            log::debug!("Polling GitHub for device authorization...");

            match auth_flow.api_client().github_device_poll(device_code).await {
                Ok(login_response) => {
                    log::info!(
                        "GitHub Device Flow completed successfully for user: {}",
                        login_response.user.username
                    );
                    self.handle_successful_github_login(app, auth_flow, login_response)
                        .await?;
                }
                Err(e) => {
                    self.handle_github_poll_error(app, e);
                }
            }

            self.last_device_poll = std::time::Instant::now();
        }

        Ok(())
    }

    fn reset_github_auth_state(&self, app: &mut App) {
        app.auth_state.github_auth_in_progress = false;
        app.auth_state.github_device_code = None;
        app.auth_state.github_user_code = None;
        app.auth_state.github_verification_uri = None;
        app.auth_state.github_poll_interval = None;
        app.auth_state.github_auth_start_time = None;
    }

    async fn handle_successful_github_login(
        &self,
        app: &mut App,
        auth_flow: &mut AuthFlow,
        login_response: fido_types::LoginResponse,
    ) -> Result<()> {
        // Store session and update state
        if let Err(e) = auth_flow.save_session(&login_response.session_token) {
            log::error!("Failed to save session: {}", e);
        }

        // Set session token in both API clients
        auth_flow
            .api_client_mut()
            .set_session_token(Some(login_response.session_token.clone()));
        app.api_client
            .set_session_token(Some(login_response.session_token.clone()));

        app.auth_state.current_user = Some(login_response.user);
        app.current_screen = Screen::Main;
        self.reset_github_auth_state(app);
        app.auth_state.error = None;

        // Load initial data
        let _ = app.load_settings().await;
        app.load_filter_preference();
        let _ = app.init_community_context().await;
        let _ = app.load_posts().await;

        Ok(())
    }

    fn handle_github_poll_error(&self, app: &mut App, error: crate::api::ApiError) {
        let error_msg = format!("{:?}", error);
        log::debug!("Device poll error: {}", error_msg);

        if !error_msg.contains("authorization_pending") {
            log::error!("Error polling for device authorization: {}", error);
            app.auth_state.error = Some(format!("Device authorization error: {}", error));
            self.reset_github_auth_state(app);
        }
    }

    async fn handle_tab_changes(&mut self, app: &mut App) -> Result<()> {
        if app.current_tab == self.last_tab {
            return Ok(());
        }

        match app.current_tab {
            Tab::Profile => {
                if app.profile_state.profile.is_none() || app.profile_state.error.is_some() {
                    app.load_profile().await?;
                }
            }
            Tab::DMs => {
                if !app.dms_state.conversations_loaded || app.dms_state.error.is_some() {
                    app.load_conversations().await?;
                }
                if app.dms_state.selection.conversation_index().is_some() {
                    app.dms_state.needs_message_load = true;
                }
            }
            Tab::Settings => {
                if app.settings_state.config.is_none() || app.settings_state.error.is_some() {
                    app.load_settings().await?;
                }
            }
            _ => {}
        }

        self.last_tab = app.current_tab;
        Ok(())
    }

    async fn handle_dm_conversation_changes(&mut self, app: &mut App) -> Result<()> {
        if app.current_tab != Tab::DMs {
            return Ok(());
        }

        let needs_load = app.dms_state.needs_message_load;

        if needs_load && !app.dms_state.conversations.is_empty() {
            app.load_conversation_messages().await?;
            self.last_dm_selection = app.dms_state.selection.clone();
            app.dms_state.needs_message_load = false;
        } else if app.dms_state.selection != self.last_dm_selection {
            self.last_dm_selection = app.dms_state.selection.clone();
        }

        Ok(())
    }

    fn render_ui(&mut self, app: &mut App, tui: &mut crate::terminal::Terminal) -> Result<()> {
        tui.draw(|frame| {
            // Update viewport height if terminal size changed
            let current_size = (frame.area().width, frame.area().height);
            if current_size != self.last_terminal_size {
                self.last_terminal_size = current_size;
            }

            ui::render(app, frame)
        })?;

        Ok(())
    }

    async fn handle_pending_loads(&self, app: &mut App) -> Result<()> {
        // Check if we need to perform a pending load
        if app.posts_state.pending_load {
            app.posts_state.pending_load = false;
            app.load_posts().await?;
        }

        // Activity load runs after the posts load so the board is already
        // populated when it lands (load_posts sets this flag on success).
        if app.posts_state.activity_pending_load {
            app.posts_state.activity_pending_load = false;
            app.load_activity().await;
        }

        // Load hashtags when modal is opened and hashtags list is empty
        if app.hashtags_state.show_hashtags_modal
            && app.hashtags_state.hashtags.is_empty()
            && !app.hashtags_state.loading
        {
            app.load_hashtags().await?;
        }

        Ok(())
    }

    async fn process_events(&self, app: &mut App, auth_flow: &mut AuthFlow) -> Result<()> {
        if !event::poll(Duration::from_millis(33))? {
            return Ok(());
        }

        let event = event::read()?;

        // Filter out mouse events - keyboard-only navigation
        if matches!(event, Event::Mouse(_)) {
            return Ok(());
        }

        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                // Log key event with modal context
                let modal_context = self.get_modal_context(app);
                log_key_event!(
                    app.log_config,
                    "key={:?}, context={}",
                    key.code,
                    modal_context
                );

                // Handle async operations that were previously in main.rs
                self.handle_async_key_events(app, key, auth_flow).await?;
            }
        }

        Ok(())
    }

    fn get_modal_context(&self, app: &App) -> &'static str {
        if app.composer_state.is_open() {
            "composer_open"
        } else if app.viewing_post_detail {
            "post_detail"
        } else {
            "main_view"
        }
    }

    async fn handle_async_key_events(
        &self,
        app: &mut App,
        key: crossterm::event::KeyEvent,
        auth_flow: &mut AuthFlow,
    ) -> Result<()> {
        // Ctrl+C always quits immediately (highest priority)
        if key.code == KeyCode::Char('c')
            && key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
        {
            app.running = false;
            return Ok(());
        }

        // Handle the async key events that were previously in main.rs
        match key.code {
            KeyCode::Char('l') if app.current_screen == Screen::Auth && !app.auth_state.loading => {
                app.load_test_users().await?;
            }
            KeyCode::Char('g') | KeyCode::Char('G')
                if app.current_screen == Screen::Auth
                    && !app.auth_state.loading
                    && !app.auth_state.github_auth_in_progress
                    && app.auth_state.show_github_option =>
            {
                self.initiate_github_device_flow(app, auth_flow).await?;
            }
            KeyCode::Esc
                if app.current_screen == Screen::Auth && app.auth_state.github_auth_in_progress =>
            {
                self.reset_github_auth_state(app);
            }
            KeyCode::Char('o') | KeyCode::Char('O')
                if app.current_screen == Screen::Auth && app.auth_state.github_auth_in_progress =>
            {
                if let Some(uri) = app.auth_state.github_verification_uri.as_deref() {
                    if let Err(e) = auth_flow.open_browser(uri) {
                        app.auth_state.error = Some(format!(
                            "Could not open browser automatically. Please visit: {} ({})",
                            uri, e
                        ));
                    } else {
                        app.auth_state.error = None;
                    }
                }
            }
            KeyCode::Enter
                if app.current_screen == Screen::Auth
                    && !app.auth_state.loading
                    && !app.auth_state.github_auth_in_progress =>
            {
                app.login_selected_user().await?;
            }
            KeyCode::Enter if app.composer_state.is_open() => {
                app.submit_composer().await?;
            }
            KeyCode::Enter if app.dms_state.show_new_conversation_modal => {
                app.start_new_conversation().await?;
            }
            KeyCode::Enter if app.posts_state.show_filter_modal => {
                self.handle_filter_modal_enter(app).await?;
            }
            KeyCode::Char('b') | KeyCode::Char('B')
                if app.current_screen == Screen::Main
                    && app.input_mode == InputMode::Navigation
                    && !app.community_browser_state.show
                    && !app.composer_state.is_open()
                    && !app.viewing_post_detail
                    && app.user_profile_view.is_none() =>
            {
                app.open_community_browser().await?;
            }
            KeyCode::Enter
                if app.current_screen == Screen::Main
                    && app.community_browser_state.show
                    && !app.community_browser_state.loading
                    && !app.community_browser_state.joining =>
            {
                app.open_or_join_browser_selection().await?;
            }
            KeyCode::Char('r') | KeyCode::Char('R')
                if app.current_screen == Screen::Main
                    && app.community_browser_state.show
                    && !app.community_browser_state.loading
                    && !app.community_browser_state.joining =>
            {
                app.community_browser_state.loaded = false;
                app.load_community_browser().await;
            }
            KeyCode::Enter | KeyCode::Char(' ')
                if app.current_tab == Tab::Posts
                    && !app.posts_state.show_new_post_modal
                    && !app.viewing_post_detail
                    && !app.composer_state.is_open()
                    && !app.posts_state.show_filter_modal
                    && !app.user_search_state.show_modal
                    && app.user_profile_view.is_none() =>
            {
                if app.is_home_list_active() {
                    app.open_home_selection().await?;
                } else {
                    self.handle_post_selection(app).await?;
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C')
                if app.show_community_modal
                    && app.community.as_ref().map(|c| !c.claimed).unwrap_or(false) =>
            {
                app.claim_current_community().await?;
            }
            KeyCode::Char('r') | KeyCode::Char('R')
                if app.current_screen == Screen::Main
                    && app.current_tab == Tab::Posts
                    && !app.viewing_post_detail
                    && !app.composer_state.is_open()
                    && (app.community_error.is_some()
                        || (app.is_home_list_active() && app.home_state.error.is_some())) =>
            {
                app.retry_community().await?;
            }
            KeyCode::Enter
                if app.current_tab == Tab::DMs
                    && !app.dms_state.show_new_conversation_modal
                    && app.input_mode == InputMode::Typing =>
            {
                app.send_dm().await?;
            }
            KeyCode::Char('u') | KeyCode::Char('U')
                if app.current_screen == Screen::Main
                    && app.current_tab == Tab::Posts
                    && !app.composer_state.is_open()
                    && !app.posts_state.show_filter_modal
                    && app.user_profile_view.is_none() =>
            {
                self.handle_vote(app, "up").await?;
            }
            KeyCode::Char('d') | KeyCode::Char('D')
                if app.current_screen == Screen::Main
                    && app.current_tab == Tab::Posts
                    && !app.composer_state.is_open()
                    && !app.posts_state.show_filter_modal
                    && app.user_profile_view.is_none() =>
            {
                self.handle_vote(app, "down").await?;
            }
            KeyCode::Char('s') | KeyCode::Char('S')
                if app.current_screen == Screen::Main
                    && app.current_tab == Tab::Settings
                    && !app.settings_state.show_save_confirmation =>
            {
                app.save_settings().await?;
            }
            KeyCode::Char('y') | KeyCode::Char('Y')
                if app.viewing_post_detail
                    && app
                        .post_detail_state
                        .as_ref()
                        .map(|s| s.show_delete_confirmation)
                        .unwrap_or(false) =>
            {
                app.delete_post().await?;
            }
            KeyCode::Char('y') | KeyCode::Char('Y')
                if app.settings_state.show_save_confirmation =>
            {
                self.handle_save_confirmation(app).await?;
            }
            KeyCode::Char('L') if app.current_screen == Screen::Main => {
                app.logout().await?;
            }
            // p: profile from friends modal
            KeyCode::Char('p') | KeyCode::Char('P')
                if app.user_profile_view.is_none()
                    && app.friends_state.show_friends_modal
                    && !app.friends_state.search_mode =>
            {
                let target = app
                    .get_filtered_social_list()
                    .get(app.friends_state.selected_index)
                    .map(|u| (u.id, u.username.clone()));
                if let Some((id, username)) = target {
                    app.friends_state.show_friends_modal = false;
                    app.friends_state.return_to_modal_after_profile = true;
                    app.open_user_profile(id, username).await?;
                }
            }
            // Enter: profile from user search modal
            KeyCode::Enter
                if app.user_profile_view.is_none() && app.user_search_state.show_modal =>
            {
                let target = app
                    .user_search_state
                    .search_results
                    .get(app.user_search_state.selected_index)
                    .map(|u| (u.id, u.username.clone()));
                if let Some((id, username)) = target {
                    app.close_user_search_modal();
                    app.open_user_profile(id, username).await?;
                }
            }
            // p: profile of selected post's author (posts list, board active)
            KeyCode::Char('p') | KeyCode::Char('P')
                if app.user_profile_view.is_none()
                    && app.current_screen == Screen::Main
                    && app.current_tab == Tab::Posts
                    && !app.viewing_post_detail
                    && !app.composer_state.is_open()
                    && !app.posts_state.show_filter_modal
                    && !app.is_home_list_active()
                    && app.input_mode == InputMode::Navigation =>
            {
                let target = app
                    .posts_state
                    .list_state
                    .selected()
                    .and_then(|i| app.posts_state.list_index_to_post_index(i))
                    .and_then(|post_index| app.posts_state.posts.get(post_index))
                    .map(|post| (post.author_id, post.author_username.clone()));
                if let Some((id, username)) = target {
                    app.open_user_profile(id, username).await?;
                }
            }
            // o: open the selected repo-activity item on GitHub
            KeyCode::Char('o')
                if app.current_screen == Screen::Main
                    && app.current_tab == Tab::Posts
                    && !app.viewing_post_detail
                    && !app.composer_state.is_open()
                    && !app.posts_state.show_filter_modal
                    && app.input_mode == InputMode::Navigation
                    && app.user_profile_view.is_none()
                    && app.selected_activity_url().is_some() =>
            {
                if let Some(url) = app.selected_activity_url() {
                    if let Err(e) = webbrowser::open(&url) {
                        app.posts_state.error = Some(format!("Could not open browser: {}", e));
                    }
                }
            }
            // p: profile from DM conversation list (navigation mode only)
            KeyCode::Char('p') | KeyCode::Char('P')
                if app.user_profile_view.is_none()
                    && app.current_tab == Tab::DMs
                    && app.input_mode == InputMode::Navigation
                    && !app.dms_state.show_new_conversation_modal =>
            {
                let target = match &app.dms_state.selection {
                    DMSelection::Conversation(idx) => app
                        .dms_state
                        .conversations
                        .get(*idx)
                        .map(|c| (c.other_user_id, c.other_username.clone())),
                    _ => None,
                };
                if let Some((id, username)) = target {
                    app.open_user_profile(id, username).await?;
                }
            }
            KeyCode::Char('f') | KeyCode::Char('F') if app.user_profile_view.is_some() => {
                app.toggle_follow_from_profile().await?;
            }
            KeyCode::Char('m') | KeyCode::Char('M') if app.user_profile_view.is_some() => {
                app.message_user_from_profile().await?;
            }
            // a: accept a pending DM request
            KeyCode::Char('a') | KeyCode::Char('A')
                if app.current_tab == Tab::DMs
                    && app.input_mode == InputMode::Navigation
                    && matches!(app.dms_state.selection, DMSelection::Request(_)) =>
            {
                if let DMSelection::Request(idx) = app.dms_state.selection {
                    if let Some(req) = app.dms_state.pending_requests.get(idx) {
                        let from = req.from_user_id;
                        match app.api_client.accept_dm_request(from).await {
                            Ok(()) => {
                                app.load_conversations().await?;
                                app.dms_state.restore_request_selection(idx);
                            }
                            Err(e) => app.dms_state.error = Some(format!("Accept failed: {}", e)),
                        }
                    }
                }
            }
            // x: decline a pending DM request
            KeyCode::Char('x') | KeyCode::Char('X')
                if app.current_tab == Tab::DMs
                    && app.input_mode == InputMode::Navigation
                    && matches!(app.dms_state.selection, DMSelection::Request(_)) =>
            {
                if let DMSelection::Request(idx) = app.dms_state.selection {
                    if let Some(req) = app.dms_state.pending_requests.get(idx) {
                        let from = req.from_user_id;
                        match app.api_client.decline_dm_request(from).await {
                            Ok(()) => {
                                app.load_conversations().await?;
                                app.dms_state.restore_request_selection(idx);
                            }
                            Err(e) => app.dms_state.error = Some(format!("Decline failed: {}", e)),
                        }
                    }
                }
            }
            _ => {
                // Delegate to synchronous key handling
                app.handle_key_event(key)?;
            }
        }

        Ok(())
    }

    async fn initiate_github_device_flow(
        &self,
        app: &mut App,
        auth_flow: &mut AuthFlow,
    ) -> Result<()> {
        app.auth_state.loading = true;
        app.auth_state.error = None;

        match auth_flow.initiate_github_device_flow().await {
            Ok((device_code, user_code, verification_uri, interval)) => {
                app.auth_state.github_device_code = Some(device_code);
                app.auth_state.github_user_code = Some(user_code.clone());
                app.auth_state.github_verification_uri = Some(verification_uri.clone());
                app.auth_state.github_poll_interval = Some(interval);
                app.auth_state.github_auth_in_progress = true;
                app.auth_state.github_auth_start_time = Some(std::time::Instant::now());
                app.auth_state.loading = false;
            }
            Err(e) => {
                app.auth_state.error =
                    Some(format!("Failed to initiate GitHub Device Flow: {}", e));
                app.auth_state.loading = false;
            }
        }

        Ok(())
    }

    async fn handle_filter_modal_enter(&self, app: &mut App) -> Result<()> {
        // In hashtags tab add input mode, Enter follows the hashtag
        if app.posts_state.filter_modal_state.selected_tab == FilterTab::Hashtags
            && app.posts_state.filter_modal_state.show_add_hashtag_input
        {
            let hashtag_name = app
                .posts_state
                .filter_modal_state
                .add_hashtag_input
                .trim()
                .to_string();
            if !hashtag_name.is_empty() {
                app.follow_hashtag(&hashtag_name).await?;
                app.posts_state.filter_modal_state.show_add_hashtag_input = false;
                app.posts_state.filter_modal_state.add_hashtag_input.clear();
            }
            return Ok(()); // Don't apply filter, just followed a hashtag
        }

        // In hashtags tab on "Add Hashtag" option, don't apply filter
        if app.posts_state.filter_modal_state.selected_tab == FilterTab::Hashtags
            && app.posts_state.filter_modal_state.selected_index
                == app.posts_state.filter_modal_state.hashtag_list.len()
        {
            // This will be handled by synchronous key handling
            return Ok(());
        }

        // Apply filter based on checked items
        let filter = match app.posts_state.filter_modal_state.selected_tab {
            FilterTab::All => PostFilter::All,
            FilterTab::Hashtags => {
                if !app
                    .posts_state
                    .filter_modal_state
                    .checked_hashtags
                    .is_empty()
                {
                    PostFilter::Multi {
                        hashtags: app.posts_state.filter_modal_state.checked_hashtags.clone(),
                        users: vec![],
                    }
                } else {
                    PostFilter::All
                }
            }
            FilterTab::Users => {
                if !app.posts_state.filter_modal_state.checked_users.is_empty() {
                    PostFilter::Multi {
                        hashtags: vec![],
                        users: app.posts_state.filter_modal_state.checked_users.clone(),
                    }
                } else {
                    PostFilter::All
                }
            }
        };
        app.apply_filter(filter).await?;

        Ok(())
    }

    async fn handle_post_selection(&self, app: &mut App) -> Result<()> {
        if let Some(list_index) = app.posts_state.list_state.selected() {
            if let Some(post_index) = app.posts_state.list_index_to_post_index(list_index) {
                let post_id = app.posts_state.posts[post_index].id;
                app.open_post_detail(post_id).await?;
            }
        }
        Ok(())
    }

    async fn handle_vote(&self, app: &mut App, direction: &str) -> Result<()> {
        if app.viewing_post_detail {
            app.vote_in_detail_view(direction).await?;
        } else {
            app.vote_on_selected_post(direction).await?;
        }
        Ok(())
    }

    async fn handle_save_confirmation(&self, app: &mut App) -> Result<()> {
        app.save_settings().await?;
        if let Some(pending_tab) = app.settings_state.pending_tab.take() {
            app.settings_state.show_save_confirmation = false;
            app.current_tab = pending_tab;
        }
        Ok(())
    }
}

/// Helper to track modal state changes and trigger data loading
struct ModalStateTracker {
    filter_modal: bool,
    friends_modal: bool,
    new_conversation_modal: bool,
    user_search_modal: bool,
    last_search_query: String,
    community_modal: bool,
}

impl ModalStateTracker {
    fn new() -> Self {
        Self {
            filter_modal: false,
            friends_modal: false,
            new_conversation_modal: false,
            user_search_modal: false,
            last_search_query: String::new(),
            community_modal: false,
        }
    }

    /// Check and handle modal state changes, loading data when modals open
    async fn check_and_load(&mut self, app: &mut App) -> Result<()> {
        self.handle_filter_modal(app).await?;
        self.handle_friends_modal(app).await?;
        self.handle_user_search_modal(app).await?;
        self.handle_new_conversation_modal(app).await?;
        self.handle_community_modal(app).await?;
        Ok(())
    }

    async fn handle_filter_modal(&mut self, app: &mut App) -> Result<()> {
        if app.posts_state.show_filter_modal && !self.filter_modal {
            app.load_filter_modal_data().await?;
        }
        self.filter_modal = app.posts_state.show_filter_modal;
        Ok(())
    }

    async fn handle_friends_modal(&mut self, app: &mut App) -> Result<()> {
        if app.friends_state.show_friends_modal && !self.friends_modal {
            app.load_social_connections().await?;
        }
        self.friends_modal = app.friends_state.show_friends_modal;
        Ok(())
    }

    async fn handle_user_search_modal(&mut self, app: &mut App) -> Result<()> {
        if app.user_search_state.show_modal {
            if !self.user_search_modal {
                // Modal just opened - no search yet
                self.user_search_modal = true;
                self.last_search_query = String::new();
            } else if app.user_search_state.search_query != self.last_search_query {
                // Query changed - trigger search
                self.last_search_query = app.user_search_state.search_query.clone();
                app.search_users().await?;
            }
        } else {
            self.user_search_modal = false;
            self.last_search_query.clear();
        }
        Ok(())
    }

    async fn handle_new_conversation_modal(&mut self, app: &mut App) -> Result<()> {
        if app.dms_state.show_new_conversation_modal && !self.new_conversation_modal {
            app.load_mutual_friends_for_dms().await?;
        }
        self.new_conversation_modal = app.dms_state.show_new_conversation_modal;
        Ok(())
    }

    async fn handle_community_modal(&mut self, app: &mut App) -> Result<()> {
        if app.show_community_modal && !self.community_modal {
            if let Some(community) = &app.community {
                let id = community.id;
                app.community_members = app
                    .api_client
                    .get_community_members(id)
                    .await
                    .unwrap_or_default();
            }
        }
        self.community_modal = app.show_community_modal;
        Ok(())
    }
}
