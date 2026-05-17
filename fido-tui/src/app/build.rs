use super::*;
use crate::api::Backend;
use ratatui::style::Style;
use ratatui::widgets::ListState;
use tui_textarea::TextArea;

impl App {
    /// Create a new App with default backend.
    pub fn new() -> Self {
        Self::build(Backend::default(), true, false)
    }

    /// Create a new App with a custom server URL.
    pub fn with_server_url(server_url: String) -> Self {
        Self::build(Backend::api(server_url), true, false)
    }

    /// Create a new App in demo mode with MockBackend.
    pub fn demo() -> Self {
        Self::build(Backend::mock(), false, true)
    }

    fn build(api_client: Backend, show_github_option: bool, is_demo_mode: bool) -> Self {
        let config_manager =
            crate::config::ConfigManager::new().expect("Failed to initialize config manager");
        let instance_id = crate::config::ConfigManager::generate_instance_id();

        // Clean up old sessions on startup
        let _ = config_manager.cleanup_old_sessions();

        Self {
            running: true,
            current_screen: Screen::Auth,
            api_client,
            auth_state: AuthState {
                test_users: Vec::new(),
                selected_index: 0,
                loading: false,
                error: None,
                current_user: None,
                show_github_option,
                github_auth_in_progress: false,
                github_device_code: None,
                github_user_code: None,
                github_verification_uri: None,
                github_poll_interval: None,
                github_auth_start_time: None,
            },
            current_tab: Tab::Posts,
            posts_state: PostsState {
                posts: Vec::new(),
                list_state: ListState::default(),
                loading: false,
                error: None,
                message: None,
                show_new_post_modal: false,
                new_post_content: String::new(),
                pending_load: false,
                current_filter: PostFilter::All,
                show_filter_modal: false,
                filter_modal_state: FilterModalState {
                    selected_tab: FilterTab::All,
                    hashtag_list: Vec::new(),
                    user_list: Vec::new(),
                    selected_index: 0,
                    search_input: String::new(),
                    search_mode: false,
                    search_results: Vec::new(),
                    checked_hashtags: Vec::new(),
                    checked_users: Vec::new(),
                    show_add_hashtag_input: false,
                    add_hashtag_input: String::new(),
                },
                at_end_of_feed: false,
            },
            profile_state: ProfileState {
                profile: None,
                user_posts: Vec::new(),
                list_state: ListState::default(),
                loading: false,
                error: None,
                show_edit_bio_modal: false,
                edit_bio_content: String::new(),
                edit_bio_cursor_position: 0,
            },
            dms_state: DMsState {
                conversations: Vec::new(),
                conversations_loaded: false,
                selection: DMSelection::NewConversation,
                messages: Vec::new(),
                loading: false,
                error: None,
                message_textarea: {
                    let mut textarea = TextArea::default();
                    textarea.set_cursor_line_style(Style::default());
                    textarea.set_style(Style::default());
                    // Enable hard tab indent for better wrapping behavior
                    textarea.set_hard_tab_indent(true);
                    textarea
                },
                show_new_conversation_modal: false,
                new_conversation_username: String::new(),
                pending_conversation_username: None,
                unread_counts: std::collections::HashMap::new(),
                current_conversation_user: None,
                needs_message_load: false,
                show_dm_error_modal: false,
                dm_error_message: String::new(),
                failed_username: None,
                available_mutual_friends: Vec::new(),
                new_conversation_selected_index: 0,
                new_conversation_search_mode: false,
                new_conversation_search_query: String::new(),
            },
            settings_state: SettingsState {
                config: None,
                original_config: None,
                original_max_posts_input: String::new(),
                loading: false,
                error: None,
                selected_field: SettingsField::ColorScheme,
                max_posts_input: String::new(),
                has_unsaved_changes: false,
                show_save_confirmation: false,
                pending_tab: None,
            },
            post_detail_state: None,
            viewing_post_detail: false,
            config_manager,
            instance_id,
            show_help: false,
            input_mode: InputMode::Navigation,
            composer_state: ComposerState::new(),
            friends_state: FriendsState {
                show_friends_modal: false,
                selected_tab: SocialTab::Following,
                following: Vec::new(),
                followers: Vec::new(),
                mutual_friends: Vec::new(),
                selected_index: 0,
                search_query: String::new(),
                search_mode: false,
                error: None,
                loading: false,
                return_to_modal_after_profile: false,
            },
            hashtags_state: HashtagsState {
                hashtags: Vec::new(),
                show_hashtags_modal: false,
                show_add_hashtag_input: false,
                add_hashtag_name: String::new(),
                selected_hashtag: 0,
                error: None,
                loading: false,
                show_unfollow_confirmation: false,
                hashtag_to_unfollow: None,
            },
            user_search_state: UserSearchState {
                show_modal: false,
                search_query: String::new(),
                search_results: Vec::new(),
                selected_index: 0,
                loading: false,
                error: None,
            },
            user_profile_view: None,
            log_config: crate::logging::LogConfig::default(),
            is_demo_mode,
            pending_vote_tasks: Vec::new(),
            update_available: None,
        }
    }
}
