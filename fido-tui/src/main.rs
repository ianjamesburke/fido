mod api;
mod app;
mod auth;
mod config;
mod debug_log;
mod emoji;
#[macro_use]
mod logging;
mod session;
mod terminal;
mod text_wrapper;
mod ui;

use anyhow::Result;
use app::{App, FilterTab};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;

// Load environment variables from .env file
// This allows FIDO_SERVER_URL and other config to be set without command-line args
fn load_env() {
    // Load from workspace root .env file (fido/.env)
    let _ = dotenv::dotenv();
}

// Performance optimization notes:
// - Lazy rendering: Only visible posts/messages are rendered (not all 1000+)
// - Virtual scrolling: Empty lines represent off-screen content
// - Viewport caching: Terminal size changes trigger viewport recalculation
// - Smooth scrolling: Scroll margin keeps selected item in middle third
// - Minimal redraws: Only changed portions trigger re-render
//
// Performance testing recommendations:
// 1. Test with 1000+ posts: Create test data with large post count
// 2. Monitor frame rate: Should maintain 60fps even with large datasets
// 3. Memory usage: Should remain constant regardless of post count
// 4. Scroll responsiveness: j/k navigation should feel instant

/// Helper to track modal state changes and trigger data loading
struct ModalStateTracker {
    filter_modal: bool,
    friends_modal: bool,
    new_conversation_modal: bool,
}

impl ModalStateTracker {
    fn new() -> Self {
        Self {
            filter_modal: false,
            friends_modal: false,
            new_conversation_modal: false,
        }
    }

    /// Check and handle modal state changes, loading data when modals open
    async fn check_and_load(&mut self, app: &mut App) -> Result<()> {
        // Filter modal
        if app.posts_state.show_filter_modal && !self.filter_modal {
            app.load_filter_modal_data().await?;
        }
        self.filter_modal = app.posts_state.show_filter_modal;

        // Friends modal
        if app.friends_state.show_friends_modal && !self.friends_modal {
            app.load_social_connections().await?;
        }
        self.friends_modal = app.friends_state.show_friends_modal;

        // New conversation modal
        if app.dms_state.show_new_conversation_modal && !self.new_conversation_modal {
            app.load_mutual_friends_for_dms().await?;
        }
        self.new_conversation_modal = app.dms_state.show_new_conversation_modal;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    load_env();
    
    // Initialize logging system
    // You can change this to LogConfig::disabled() or LogConfig::minimal() to reduce logging
    let log_config = logging::LogConfig::default();
    logging::init_logging(&log_config)?;
    
    // Initialize terminal
    let mut tui = terminal::init()?;

    // Create app with logging config
    let mut app = App::new();
    app.log_config = log_config;

    // Check if running in web mode (for web terminal interface)
    let is_web_mode = std::env::var("FIDO_WEB_MODE").is_ok();
    
    // In web mode, hide GitHub OAuth option (test users only)
    if is_web_mode {
        app.auth_state.show_github_option = false;
    }
    
    // Check for existing session on startup (skip in web mode)
    let mut auth_flow = auth::AuthFlow::new(app.api_client.clone())?;
    if !is_web_mode {
        if let Ok(Some(user)) = auth_flow.check_existing_session().await {
            log::info!("Restored session for user: {}", user.username);
            app.auth_state.current_user = Some(user);
            app.current_screen = app::Screen::Main;
            
            // Update API client with session token
            app.api_client = auth_flow.api_client().clone();
            
            // Load initial data
            let _ = app.load_settings().await;
            app.load_filter_preference();
            let _ = app.load_posts().await;
        } else {
            log::info!("No valid session found, showing authentication screen");
            // Load test users for authentication screen
            let _ = app.load_test_users().await;
        }
    } else {
        log::info!("Running in web mode, loading test users only");
        // In web mode, always show test users (no GitHub OAuth)
        let _ = app.load_test_users().await;
    }

    // Main event loop
    let mut last_tab = app.current_tab;
    let mut last_dm_conversation_index = app.dms_state.selected_conversation_index;
    let mut last_terminal_size = (0, 0);
    let mut modal_tracker = ModalStateTracker::new();
    
    // Track last poll time for device flow
    let mut last_device_poll = std::time::Instant::now();
    
    while app.running {
        // Poll for GitHub Device Flow completion if in progress
        if app.auth_state.github_auth_in_progress {
            // Check for timeout (15 minutes)
            if let Some(start_time) = app.auth_state.github_auth_start_time {
                if start_time.elapsed() > Duration::from_secs(900) {
                    log::warn!("GitHub Device Flow timeout after 15 minutes");
                    app.auth_state.error = Some("Device authorization timeout: Please try again.".to_string());
                    app.auth_state.github_auth_in_progress = false;
                    app.auth_state.github_device_code = None;
                    app.auth_state.github_user_code = None;
                    app.auth_state.github_verification_uri = None;
                    app.auth_state.github_poll_interval = None;
                    app.auth_state.github_auth_start_time = None;
                }
            }
            
            // Only poll at the specified interval (default 5 seconds)
            let poll_interval = app.auth_state.github_poll_interval.unwrap_or(5);
            if last_device_poll.elapsed() >= Duration::from_secs(poll_interval as u64) {
                if let Some(device_code) = &app.auth_state.github_device_code.clone() {
                    log::debug!("Polling GitHub for device authorization...");
                    
                    // Try to poll for device authorization
                    match auth_flow.api_client().github_device_poll(&device_code).await {
                        Ok(login_response) => {
                            log::info!("GitHub Device Flow completed successfully for user: {}", login_response.user.username);
                            
                            // Store session and update state
                            if let Err(e) = auth_flow.save_session(&login_response.session_token) {
                                log::error!("Failed to save session: {}", e);
                            }
                            
                            // Set session token in both API clients
                            auth_flow.api_client_mut().set_session_token(Some(login_response.session_token.clone()));
                            app.api_client.set_session_token(Some(login_response.session_token.clone()));
                            
                            app.auth_state.current_user = Some(login_response.user);
                            app.current_screen = app::Screen::Main;
                            app.auth_state.github_auth_in_progress = false;
                            app.auth_state.github_device_code = None;
                            app.auth_state.github_user_code = None;
                            app.auth_state.github_verification_uri = None;
                            app.auth_state.github_poll_interval = None;
                            app.auth_state.github_auth_start_time = None;
                            app.auth_state.error = None;
                            
                            // Load initial data
                            let _ = app.load_settings().await;
                            app.load_filter_preference();
                            let _ = app.load_posts().await;
                        }
                        Err(e) => {
                            // Check if it's just pending
                            let error_msg = format!("{:?}", e);
                            log::debug!("Device poll error: {}", error_msg);
                            
                            if !error_msg.contains("authorization_pending") {
                                log::error!("Error polling for device authorization: {}", e);
                                app.auth_state.error = Some(format!("Device authorization error: {}", e));
                                app.auth_state.github_auth_in_progress = false;
                                app.auth_state.github_device_code = None;
                                app.auth_state.github_user_code = None;
                                app.auth_state.github_verification_uri = None;
                                app.auth_state.github_poll_interval = None;
                                app.auth_state.github_auth_start_time = None;
                            }
                            // If authorization_pending, just continue polling
                        }
                    }
                    
                    last_device_poll = std::time::Instant::now();
                }
            }
        }

        // Check modal state changes and load data as needed
        modal_tracker.check_and_load(&mut app).await?;

        // Check if we switched tabs and need to load data
        if app.current_tab != last_tab {
            match app.current_tab {
                app::Tab::Profile => {
                    app.load_profile().await?;
                }
                app::Tab::DMs => {
                    app.load_conversations().await?;
                    // load_conversations() will set selected_conversation_index to 0 if conversations exist
                }
                app::Tab::Settings => {
                    app.load_settings().await?;
                }
                _ => {}
            }
        }
        
        // Check if we switched conversations in DMs tab or need to load messages
        if app.current_tab == app::Tab::DMs
            && (app.dms_state.selected_conversation_index != last_dm_conversation_index || app.dms_state.needs_message_load)
            && !app.dms_state.conversations.is_empty()
        {
            app.load_conversation_messages().await?;
            last_dm_conversation_index = app.dms_state.selected_conversation_index;
            app.dms_state.needs_message_load = false;
        }
        
        last_tab = app.current_tab;

        // Render UI with performance optimization
        tui.draw(|frame| {
            // Update viewport height if terminal size changed (for efficient scrolling)
            let current_size = (frame.area().width, frame.area().height);
            if current_size != last_terminal_size {
                last_terminal_size = current_size;
            }
            
            ui::render(&mut app, frame)
        })?;
        
        // Check if we need to perform a pending load (after UI has rendered loading state)
        if app.posts_state.pending_load {
            app.posts_state.pending_load = false;
            app.load_posts().await?;
        }
        
        // Friends modal data loading is now handled above with last_friends_modal_state

        // Load hashtags when modal is opened and hashtags list is empty
        if app.hashtags_state.show_hashtags_modal && app.hashtags_state.hashtags.is_empty() && !app.hashtags_state.loading {
            app.load_hashtags().await?;
        }

        // Handle events with timeout
        if event::poll(Duration::from_millis(100))? {
            let event = event::read()?;
            
            // Filter out mouse events - keyboard-only navigation
            if matches!(event, Event::Mouse(_)) {
                continue;
            }
            
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press {
                    // Log key event with modal context
                    let modal_context = if app.composer_state.is_open() {
                        "composer_open"
                    } else if app.viewing_post_detail {
                        "post_detail"
                    } else {
                        "main_view"
                    };
                    log_key_event!(app.log_config, "key={:?}, context={}", key.code, modal_context);
                    
                    // Handle async operations
                    match key.code {
                        KeyCode::Char('l') if app.current_screen == app::Screen::Auth => {
                            app.load_test_users().await?;
                        }
                        KeyCode::Char('g') | KeyCode::Char('G') if app.current_screen == app::Screen::Auth && !app.auth_state.github_auth_in_progress && app.auth_state.show_github_option => {
                            // Initiate GitHub Device Flow (only if GitHub option is enabled)
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
                                    
                                    // Try to open browser to verification URI
                                    if let Err(e) = auth_flow.open_browser(&verification_uri) {
                                        log::warn!("Failed to open browser: {}", e);
                                        app.auth_state.error = Some(format!(
                                            "Could not open browser automatically. Please visit: {}",
                                            verification_uri
                                        ));
                                    }
                                    
                                    // Polling will happen in the main loop
                                }
                                Err(e) => {
                                    app.auth_state.error = Some(format!("Failed to initiate GitHub Device Flow: {}", e));
                                    app.auth_state.loading = false;
                                }
                            }
                        }
                        KeyCode::Esc if app.current_screen == app::Screen::Auth && app.auth_state.github_auth_in_progress => {
                            // Cancel GitHub Device Flow
                            app.auth_state.github_auth_in_progress = false;
                            app.auth_state.github_device_code = None;
                            app.auth_state.github_user_code = None;
                            app.auth_state.github_verification_uri = None;
                            app.auth_state.github_poll_interval = None;
                            app.auth_state.github_auth_start_time = None;
                            app.auth_state.error = None;
                        }
                        KeyCode::Enter if app.current_screen == app::Screen::Auth && !app.auth_state.github_auth_in_progress => {
                            app.login_selected_user().await?;
                        }
                        // Unified composer: Enter submits for all modes (NewPost, Reply, EditBio, EditPost)
                        KeyCode::Enter if app.composer_state.is_open() => {
                            app.submit_composer().await?;
                        }
                        KeyCode::Enter if app.dms_state.show_new_conversation_modal => {
                            app.start_new_conversation().await?;
                        }
                        KeyCode::Enter if app.posts_state.show_filter_modal => {
                            use app::FilterTab;
                            
                            // In hashtags tab add input mode, Enter follows the hashtag
                            if app.posts_state.filter_modal_state.selected_tab == FilterTab::Hashtags 
                                && app.posts_state.filter_modal_state.show_add_hashtag_input {
                                let hashtag_name = app.posts_state.filter_modal_state.add_hashtag_input.trim().to_string();
                                if !hashtag_name.is_empty() {
                                    app.follow_hashtag(&hashtag_name).await?;
                                    app.posts_state.filter_modal_state.show_add_hashtag_input = false;
                                    app.posts_state.filter_modal_state.add_hashtag_input.clear();
                                }
                                continue; // Don't apply filter, just followed a hashtag
                            }
                            
                            // In hashtags tab on "Add Hashtag" option, don't apply filter (handled by handle_key_event)
                            if app.posts_state.filter_modal_state.selected_tab == FilterTab::Hashtags 
                                && app.posts_state.filter_modal_state.selected_index == app.posts_state.filter_modal_state.hashtag_list.len() {
                                app.handle_key_event(key)?;
                                continue; // Don't apply filter, just opened add input
                            }
                            
                            // Apply filter based on checked items
                            let filter = match app.posts_state.filter_modal_state.selected_tab {
                                FilterTab::All => app::PostFilter::All,
                                FilterTab::Hashtags => {
                                    // Only apply filter if hashtags are checked
                                    if !app.posts_state.filter_modal_state.checked_hashtags.is_empty() {
                                        app::PostFilter::Multi {
                                            hashtags: app.posts_state.filter_modal_state.checked_hashtags.clone(),
                                            users: vec![],
                                        }
                                    } else {
                                        // No hashtags checked = show all posts
                                        app::PostFilter::All
                                    }
                                }
                                FilterTab::Users => {
                                    // Only apply filter if users are checked
                                    if !app.posts_state.filter_modal_state.checked_users.is_empty() {
                                        app::PostFilter::Multi {
                                            hashtags: vec![],
                                            users: app.posts_state.filter_modal_state.checked_users.clone(),
                                        }
                                    } else {
                                        // No users checked = show all posts
                                        app::PostFilter::All
                                    }
                                }
                            };
                            app.apply_filter(filter).await?;
                        }
                        KeyCode::Enter | KeyCode::Char(' ') if app.current_tab == app::Tab::Posts && !app.posts_state.show_new_post_modal && !app.viewing_post_detail && !app.composer_state.is_open() && !app.posts_state.show_filter_modal => {
                            // Open post detail view for selected post (only if filter modal is not open)
                            if let Some(selected_index) = app.posts_state.list_state.selected() {
                                if selected_index < app.posts_state.posts.len() {
                                    let post_id = app.posts_state.posts[selected_index].id;
                                    app.open_post_detail(post_id).await?;
                                }
                            }
                        }
                        KeyCode::Enter if app.current_tab == app::Tab::DMs && !app.dms_state.show_new_conversation_modal && app.input_mode == app::InputMode::Typing => {
                            app.send_dm().await?;
                        }
                        KeyCode::Char('x') | KeyCode::Char('X') if app.posts_state.show_filter_modal && app.posts_state.filter_modal_state.selected_tab == FilterTab::Hashtags && !app.posts_state.filter_modal_state.show_add_hashtag_input => {
                            // Unfollow selected hashtag in filter modal (only if not typing)
                            let selected_index = app.posts_state.filter_modal_state.selected_index;
                            if selected_index < app.posts_state.filter_modal_state.hashtag_list.len() {
                                let hashtag_name = app.posts_state.filter_modal_state.hashtag_list[selected_index].clone();
                                app.unfollow_hashtag(&hashtag_name).await?;
                            }
                        }
                        KeyCode::Char('p') | KeyCode::Char('P') if app.friends_state.show_friends_modal && !app.friends_state.search_mode => {
                            // View selected user's profile from social modal
                            let filtered_list = app.get_filtered_social_list();
                            if let Some(user) = filtered_list.get(app.friends_state.selected_index) {
                                let user_id = user.id.clone();
                                app.friends_state.return_to_modal_after_profile = true;
                                app.close_friends_modal();
                                app.load_user_profile_view(user_id).await?;
                            }
                        }
                        KeyCode::Char('f') | KeyCode::Char('F') if app.friends_state.show_friends_modal && !app.friends_state.search_mode => {
                            // Follow/unfollow selected user from social modal
                            let filtered_list = app.get_filtered_social_list();
                            if let Some(user) = filtered_list.get(app.friends_state.selected_index) {
                                let user_id = user.id.clone();
                                
                                // Check if we're following this user
                                let is_following = app.friends_state.following.iter().any(|u| u.id == user_id);
                                
                                if is_following {
                                    // Unfollow
                                    if let Err(e) = app.api_client.unfollow_user(user_id).await {
                                        app.friends_state.error = Some(format!("Failed to unfollow: {}", e));
                                    } else {
                                        // Reload social connections
                                        app.load_social_connections().await?;
                                    }
                                } else {
                                    // Follow
                                    if let Err(e) = app.api_client.follow_user(user_id).await {
                                        app.friends_state.error = Some(format!("Failed to follow: {}", e));
                                    } else {
                                        // Reload social connections
                                        app.load_social_connections().await?;
                                    }
                                }
                            }
                        }
                        KeyCode::Enter if app.hashtags_state.show_add_hashtag_input => {
                            let hashtag_name = app.hashtags_state.add_hashtag_name.trim().to_string();
                            if !hashtag_name.is_empty() {
                                app.follow_hashtag(&hashtag_name).await?;
                            }
                        }
                        KeyCode::Enter if app.hashtags_state.show_unfollow_confirmation => {
                            if let Some(hashtag) = app.hashtags_state.hashtag_to_unfollow.clone() {
                                app.unfollow_hashtag(&hashtag).await?;
                                app.hashtags_state.show_unfollow_confirmation = false;
                                app.hashtags_state.hashtag_to_unfollow = None;
                            }
                        }

                        KeyCode::Char('u') | KeyCode::Char('U') if app.current_screen == app::Screen::Main && app.current_tab == app::Tab::Posts && !app.composer_state.is_open() && !app.posts_state.show_filter_modal => {
                            if app.viewing_post_detail {
                                app.vote_in_detail_view("up").await?;
                            } else {
                                app.vote_on_selected_post("up").await?;
                            }
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') if app.current_screen == app::Screen::Main && app.current_tab == app::Tab::Posts && !app.composer_state.is_open() && !app.posts_state.show_filter_modal => {
                            if app.viewing_post_detail {
                                app.vote_in_detail_view("down").await?;
                            } else {
                                app.vote_on_selected_post("down").await?;
                            }
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') if app.current_screen == app::Screen::Main && app.current_tab == app::Tab::Settings && !app.settings_state.show_save_confirmation => {
                            app.save_settings().await?;
                        }
                        KeyCode::Char('y') | KeyCode::Char('Y') if app.viewing_post_detail && app.post_detail_state.as_ref().map(|s| s.show_delete_confirmation).unwrap_or(false) => {
                            // Confirm delete post
                            app.delete_post().await?;
                        }
                        KeyCode::Char('y') | KeyCode::Char('Y') if app.settings_state.show_save_confirmation => {
                            // Save settings and then switch tabs
                            app.save_settings().await?;
                            if let Some(pending_tab) = app.settings_state.pending_tab.take() {
                                app.settings_state.show_save_confirmation = false;
                                app.current_tab = pending_tab;
                            }
                        }
                        KeyCode::Char('x') | KeyCode::Char('X') if app.posts_state.show_filter_modal && app.posts_state.filter_modal_state.selected_tab == app::FilterTab::Hashtags && !app.posts_state.filter_modal_state.show_add_hashtag_input => {
                            // Unfollow selected hashtag (only if not on "Add Hashtag" option)
                            let selected_idx = app.posts_state.filter_modal_state.selected_index;
                            if selected_idx < app.posts_state.filter_modal_state.hashtag_list.len() {
                                if let Some(hashtag) = app.posts_state.filter_modal_state.hashtag_list.get(selected_idx).cloned() {
                                    app.unfollow_hashtag(&hashtag).await?;
                                }
                            }
                        }
                        KeyCode::Char('p') | KeyCode::Char('P') if app.current_screen == app::Screen::Main && app.current_tab == app::Tab::Posts && !app.composer_state.is_open() && !app.posts_state.show_filter_modal && app.user_profile_view.is_none() => {
                            // Open user profile view from posts feed or post detail
                            let author_id = if app.viewing_post_detail {
                                app.get_post_detail_author_id()
                            } else {
                                app.get_selected_post_author_id()
                            };
                            
                            if let Some(user_id) = author_id {
                                app.load_user_profile_view(user_id).await?;
                            }
                        }
                        KeyCode::Char('f') | KeyCode::Char('F') if app.user_profile_view.is_some() => {
                            // Toggle follow/unfollow in profile view
                            if let Some(profile) = &app.user_profile_view {
                                let user_id = profile.user_id.clone();
                                let is_following = matches!(
                                    profile.relationship,
                                    app::RelationshipStatus::Following | app::RelationshipStatus::MutualFriends
                                );
                                
                                if is_following {
                                    app.unfollow_user_in_profile_view(user_id).await?;
                                } else if !matches!(profile.relationship, app::RelationshipStatus::Self_) {
                                    app.follow_user_in_profile_view(user_id).await?;
                                }
                            }
                        }
                        KeyCode::Char('m') | KeyCode::Char('M') if app.user_profile_view.is_some() => {
                            // Open DM only if mutual friends
                            if let Some(profile) = &app.user_profile_view {
                                if matches!(profile.relationship, app::RelationshipStatus::MutualFriends) {
                                    let username = profile.username.clone();
                                    let user_id_str = profile.user_id.clone();
                                    app.close_user_profile_view();
                                    
                                    // Switch to DMs tab
                                    app.current_tab = app::Tab::DMs;
                                    
                                    // Try to open existing conversation or create new one
                                    app.open_or_create_dm_conversation(username, user_id_str).await?;
                                }
                            }
                        }
                        KeyCode::Char('L') if app.current_screen == app::Screen::Main => {
                            // Logout (Shift+L)
                            app.logout().await?;
                        }
                        _ => {
                            app.handle_key_event(key)?;
                        }
                    }
                }
            }
        }
    }

    // Restore terminal
    terminal::restore()?;

    Ok(())
}
