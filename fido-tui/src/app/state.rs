use fido_types::{Post, User, UserProfile};

use ratatui::widgets::ListState;
use std::time::Instant;
use tui_textarea::TextArea;
use uuid::Uuid;

use crate::api::ApiClient;

/// Get platform-appropriate modifier key name for display
/// Returns "Cmd" on macOS, "Ctrl" on other platforms
#[cfg(target_os = "macos")]
pub fn get_modifier_key_name() -> &'static str {
    "Cmd"
}

#[cfg(not(target_os = "macos"))]
pub fn get_modifier_key_name() -> &'static str {
    "Ctrl"
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Navigation, // Browsing content, shortcuts active
    Typing,     // In text input, shortcuts disabled
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsField {
    ColorScheme,
    SortOrder,
    MaxPosts,
}

/// Composer mode - determines what type of content is being composed
#[derive(Debug, Clone)]
pub enum ComposerMode {
    NewPost,
    Reply {
        parent_post_id: Uuid,
        parent_author: String,
        parent_content: String,
    },
    EditPost {
        post_id: Uuid,
    },
    EditBio,
}

/// Unified composer state using tui-textarea
pub struct ComposerState {
    pub mode: Option<ComposerMode>,
    pub textarea: TextArea<'static>,
    pub max_chars: usize,
}

impl ComposerState {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        // Enable hard tab indent for better wrapping behavior
        textarea.set_hard_tab_indent(true);
        Self {
            mode: None,
            textarea,
            max_chars: 280,
        }
    }

    pub fn is_open(&self) -> bool {
        self.mode.is_some()
    }

    pub fn get_content(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub fn char_count(&self) -> usize {
        crate::emoji::count_characters(&self.get_content())
    }
}

/// Social connections modal state
pub struct FriendsState {
    pub show_friends_modal: bool,
    pub selected_tab: SocialTab,
    pub following: Vec<UserInfo>,
    pub followers: Vec<UserInfo>,
    pub mutual_friends: Vec<UserInfo>,
    pub selected_index: usize,
    pub search_query: String,
    pub search_mode: bool,
    pub error: Option<String>,
    pub loading: bool,
    pub return_to_modal_after_profile: bool, // Flag to reopen modal after viewing profile
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocialTab {
    Following,
    Followers,
    MutualFriends,
}

/// User information for social lists
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub follower_count: usize,
    pub following_count: usize,
}

/// Hashtags modal state
pub struct HashtagsState {
    pub hashtags: Vec<String>,
    pub show_hashtags_modal: bool,
    pub show_add_hashtag_input: bool,
    pub add_hashtag_name: String,
    pub selected_hashtag: usize,
    pub error: Option<String>,
    pub loading: bool,
    pub show_unfollow_confirmation: bool,
    pub hashtag_to_unfollow: Option<String>,
}

/// User search modal state
pub struct UserSearchState {
    pub show_modal: bool,
    pub search_query: String,
    pub search_results: Vec<UserSearchResult>,
    pub selected_index: usize,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserSearchResult {
    pub id: String,
    pub username: String,
}

/// Main application state
pub struct App {
    pub running: bool,
    pub current_screen: Screen,
    pub api_client: ApiClient,
    pub auth_state: AuthState,
    pub current_tab: Tab,
    pub posts_state: PostsState,
    pub profile_state: ProfileState,
    pub dms_state: DMsState,
    pub settings_state: SettingsState,
    pub post_detail_state: Option<PostDetailState>,
    pub viewing_post_detail: bool,
    pub config_manager: crate::config::ConfigManager,
    pub instance_id: String,
    pub show_help: bool,
    pub input_mode: InputMode,
    pub composer_state: ComposerState,
    pub friends_state: FriendsState,
    pub hashtags_state: HashtagsState,
    pub user_search_state: UserSearchState,
    pub user_profile_view: Option<UserProfileViewState>,
    pub log_config: crate::logging::LogConfig,
}

/// Settings tab state
pub struct SettingsState {
    pub config: Option<fido_types::UserConfig>,
    pub original_config: Option<fido_types::UserConfig>,
    pub original_max_posts_input: String,
    pub loading: bool,
    pub error: Option<String>,
    pub selected_field: SettingsField,
    pub max_posts_input: String,
    pub has_unsaved_changes: bool,
    pub show_save_confirmation: bool,
    pub pending_tab: Option<Tab>,
}

/// DMs tab state
pub struct DMsState {
    pub conversations: Vec<Conversation>,
    pub selected_conversation_index: Option<usize>, // None = no conversation selected
    pub messages: Vec<fido_types::DirectMessage>,
    pub loading: bool,
    pub error: Option<String>,
    pub message_input: String, // Deprecated - kept for compatibility, use message_textarea instead
    pub message_textarea: TextArea<'static>, // TextArea for message input
    pub messages_scroll_offset: usize, // Scroll offset for message history
    pub show_new_conversation_modal: bool,
    pub new_conversation_username: String,
    pub pending_conversation_username: Option<String>, // Username for new conversation not yet created
    pub unread_counts: std::collections::HashMap<uuid::Uuid, usize>, // user_id -> unread count
    pub current_conversation_user: Option<uuid::Uuid>, // Track open conversation
    pub needs_message_load: bool,                      // Flag to trigger message loading
    /// Show DM error modal with friend suggestions
    pub show_dm_error_modal: bool,
    /// Error message to display in the modal
    pub dm_error_message: String,
    /// Username that failed when attempting to start a conversation
    pub failed_username: Option<String>,
    /// Mutual friends available for DMs (full user info with stats)
    pub available_mutual_friends: Vec<UserInfo>,
    /// Selected index in new conversation modal
    pub new_conversation_selected_index: usize,
    /// Search mode for new conversation modal
    pub new_conversation_search_mode: bool,
    /// Search query for new conversation modal
    pub new_conversation_search_query: String,
}

/// Conversation summary
#[derive(Debug, Clone)]
pub struct Conversation {
    pub other_user_id: uuid::Uuid,
    pub other_username: String,
    pub last_message: String,
    pub last_message_time: chrono::DateTime<chrono::Utc>,
    pub unread_count: i32,
}

/// Profile tab state (for viewing own profile)
pub struct ProfileState {
    pub profile: Option<UserProfile>,
    pub user_posts: Vec<Post>,
    pub list_state: ListState,
    pub loading: bool,
    pub error: Option<String>,
    pub show_edit_bio_modal: bool,
    pub edit_bio_content: String,
    pub edit_bio_cursor_position: usize,
}

/// User profile view state (for viewing other users' profiles)
pub struct UserProfileViewState {
    pub user_id: String,
    pub username: String,
    pub bio: Option<String>,
    pub join_date: String,
    pub follower_count: usize,
    pub following_count: usize,
    pub post_count: usize,
    pub relationship: RelationshipStatus,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationshipStatus {
    Self_,
    MutualFriends,
    Following,
    FollowsYou,
    None,
}

/// Filter type for posts
#[derive(Debug, Clone, PartialEq)]
pub enum PostFilter {
    All,
    Hashtag(String),
    User(String),
    Multi {
        hashtags: Vec<String>,
        users: Vec<String>,
    },
}

impl PostFilter {
    /// Convert to UserPreferences format for saving
    pub fn to_preferences(&self) -> crate::config::UserPreferences {
        match self {
            PostFilter::All => crate::config::UserPreferences {
                filter_type: "all".to_string(),
                filter_hashtag: None,
                filter_user: None,
                filter_hashtags: Vec::new(),
                filter_users: Vec::new(),
            },
            PostFilter::Hashtag(tag) => crate::config::UserPreferences {
                filter_type: "hashtag".to_string(),
                filter_hashtag: Some(tag.clone()),
                filter_user: None,
                filter_hashtags: Vec::new(),
                filter_users: Vec::new(),
            },
            PostFilter::User(user) => crate::config::UserPreferences {
                filter_type: "user".to_string(),
                filter_hashtag: None,
                filter_user: Some(user.clone()),
                filter_hashtags: Vec::new(),
                filter_users: Vec::new(),
            },
            PostFilter::Multi { hashtags, users } => crate::config::UserPreferences {
                filter_type: "multi".to_string(),
                filter_hashtag: None,
                filter_user: None,
                filter_hashtags: hashtags.clone(),
                filter_users: users.clone(),
            },
        }
    }

    /// Create from UserPreferences
    pub fn from_preferences(prefs: &crate::config::UserPreferences) -> Self {
        match prefs.filter_type.as_str() {
            "hashtag" => {
                if let Some(tag) = &prefs.filter_hashtag {
                    PostFilter::Hashtag(tag.clone())
                } else {
                    PostFilter::All
                }
            }
            "user" => {
                if let Some(user) = &prefs.filter_user {
                    PostFilter::User(user.clone())
                } else {
                    PostFilter::All
                }
            }
            "multi" => PostFilter::Multi {
                hashtags: prefs.filter_hashtags.clone(),
                users: prefs.filter_users.clone(),
            },
            _ => PostFilter::All,
        }
    }
}

/// Posts tab state
pub struct PostsState {
    pub posts: Vec<Post>,
    pub list_state: ListState,
    pub loading: bool,
    pub error: Option<String>,
    pub message: Option<(String, Instant)>, // (message, timestamp) - auto-clears after 3 seconds
    pub show_new_post_modal: bool,
    pub new_post_content: String,
    /// Flag to trigger actual load after UI renders loading state
    pub pending_load: bool,
    /// Current filter applied to posts
    pub current_filter: PostFilter,
    /// Show filter modal
    pub show_filter_modal: bool,
    /// Filter modal state
    pub filter_modal_state: FilterModalState,
    /// Current sort order display
    pub sort_order: String,
    /// Track if at end of feed (for "End of Feed" indicator)
    pub at_end_of_feed: bool,
}

impl PostsState {
    /// Calculate how many items appear before posts in the rendered list
    /// This is used to convert between post indices and list indices
    pub fn items_before_posts(&self) -> usize {
        let mut count = 0;

        // Loading spinner
        if self.loading && !self.posts.is_empty() {
            count += 1;
        }

        count
    }

    /// Convert a post index to a list index
    pub fn post_index_to_list_index(&self, post_index: usize) -> usize {
        post_index + self.items_before_posts()
    }

    /// Convert a list index to a post index (returns None if list index points to a non-post item)
    pub fn list_index_to_post_index(&self, list_index: usize) -> Option<usize> {
        let offset = self.items_before_posts();
        if list_index >= offset {
            Some(list_index - offset)
        } else {
            None
        }
    }
}

/// Filter modal state
pub struct FilterModalState {
    pub selected_tab: FilterTab,
    pub hashtag_list: Vec<String>,
    pub user_list: Vec<String>,
    pub selected_index: usize,
    pub search_input: String,
    pub search_mode: bool,
    pub search_results: Vec<String>,
    /// Checked hashtags for multi-select
    pub checked_hashtags: Vec<String>,
    /// Checked users for multi-select
    pub checked_users: Vec<String>,
    /// Show add hashtag input in hashtags tab
    pub show_add_hashtag_input: bool,
    /// Input for adding new hashtag
    pub add_hashtag_input: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterTab {
    All,
    Hashtags,
    Users,
}

/// Post detail view state
pub struct PostDetailState {
    pub post: Option<Post>,
    pub replies: Vec<Post>,
    pub reply_list_state: ListState,
    pub loading: bool,
    pub error: Option<String>,
    pub message: Option<(String, Instant)>, // (message, timestamp) - auto-clears after 3 seconds
    pub show_reply_composer: bool,
    pub reply_content: String,
    pub show_delete_confirmation: bool,
    pub previous_feed_position: Option<usize>,
    /// Track which posts are expanded (post_id -> is_expanded)
    pub expanded_posts: std::collections::HashMap<Uuid, bool>,
    /// Show full post modal
    pub show_full_post_modal: bool,
    /// Post ID for full post modal
    pub full_post_modal_id: Option<Uuid>,
    /// Modal-specific list state for nested reply navigation
    pub modal_list_state: ListState,
    /// Track expansion state within modal (separate from main view)
    pub modal_expanded_posts: std::collections::HashMap<Uuid, bool>,
}

impl PostDetailState {
    /// Get direct replies (replies that are not nested under other replies).
    ///
    /// Direct replies are those whose parent_post_id is not in the replies list,
    /// meaning they reply directly to the main post rather than to another reply.
    pub fn get_direct_replies(&self) -> Vec<&Post> {
        use std::collections::HashSet;

        // Build a set of all reply IDs for O(1) lookups
        let reply_ids: HashSet<Uuid> = self.replies.iter().map(|r| r.id).collect();

        // Filter replies whose parent is not in the reply list
        self.replies
            .iter()
            .filter(|reply| {
                reply
                    .parent_post_id
                    .map(|parent_id| !reply_ids.contains(&parent_id))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Get the post that should be deleted based on current selection.
    ///
    /// Returns the selected reply if one is selected and exists, otherwise returns the main post.
    /// Handles both main detail view and full post modal.
    pub fn get_deletable_post(&self) -> Option<&Post> {
        // If in full post modal, get the selected post from modal state
        if self.show_full_post_modal {
            if let Some(selected_idx) = self.modal_list_state.selected() {
                if selected_idx == 0 {
                    // First item is always the root post in modal
                    return self.full_post_modal_id.and_then(|id| {
                        if self.post.as_ref().map(|p| p.id) == Some(id) {
                            self.post.as_ref()
                        } else {
                            self.replies.iter().find(|r| r.id == id)
                        }
                    });
                } else {
                    // Get the flattened visible posts to find the selected one
                    if let Some(root_id) = self.full_post_modal_id {
                        let mut flattened_posts = Vec::new();
                        self.collect_visible_posts_for_modal(root_id, &mut flattened_posts);

                        if selected_idx > 0 && selected_idx <= flattened_posts.len() {
                            let post_id = flattened_posts[selected_idx - 1];
                            return self.replies.iter().find(|r| r.id == post_id);
                        }
                    }
                }
            }
            return None;
        }

        // Main detail view logic (existing)
        if self.replies.is_empty() {
            return self.post.as_ref();
        }

        if let Some(selected_idx) = self.reply_list_state.selected() {
            let direct_replies = self.get_direct_replies();
            if let Some(reply) = direct_replies.get(selected_idx) {
                return Some(reply);
            }
        }

        self.post.as_ref()
    }

    /// Helper to collect visible posts in modal (for deletion)
    fn collect_visible_posts_for_modal(&self, root_id: Uuid, result: &mut Vec<Uuid>) {
        use std::collections::HashMap;

        let mut children_map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for reply in &self.replies {
            if let Some(parent_id) = reply.parent_post_id {
                children_map
                    .entry(parent_id)
                    .or_default()
                    .push(reply.id);
            }
        }

        fn collect(
            post_id: &Uuid,
            children_map: &HashMap<Uuid, Vec<Uuid>>,
            expanded: &std::collections::HashMap<Uuid, bool>,
            result: &mut Vec<Uuid>,
        ) {
            if let Some(children) = children_map.get(post_id) {
                for child_id in children {
                    result.push(*child_id);
                    if expanded.get(child_id).copied().unwrap_or(false) {
                        collect(child_id, children_map, expanded, result);
                    }
                }
            }
        }

        collect(&root_id, &children_map, &self.modal_expanded_posts, result);
    }
}

/// Authentication state
pub struct AuthState {
    pub test_users: Vec<User>,
    pub selected_index: usize,
    pub loading: bool,
    pub error: Option<String>,
    pub current_user: Option<User>,
    pub show_github_option: bool,
    pub github_auth_in_progress: bool,
    pub github_device_code: Option<String>,
    pub github_user_code: Option<String>,
    pub github_verification_uri: Option<String>,
    pub github_poll_interval: Option<i64>,
    pub github_auth_start_time: Option<std::time::Instant>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Auth,
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Posts,
    DMs,
    Profile,
    Settings,
}

impl Tab {
    pub fn next(&self) -> Self {
        match self {
            Tab::Posts => Tab::DMs,
            Tab::DMs => Tab::Profile,
            Tab::Profile => Tab::Settings,
            Tab::Settings => Tab::Posts,
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            Tab::Posts => Tab::Settings,
            Tab::DMs => Tab::Posts,
            Tab::Profile => Tab::DMs,
            Tab::Settings => Tab::Profile,
        }
    }
}
