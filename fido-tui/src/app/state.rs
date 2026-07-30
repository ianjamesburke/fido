use fido_types::{
    Channel, ChannelMessageEvent, Message, Notification, NotificationUnreadCount, Post, User,
    UserProfile,
};

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::time::Instant;
use tui_textarea::TextArea;
use uuid::Uuid;

use crate::api::{ApiClient, RealtimeConnectionStatus};

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
    pub id: uuid::Uuid,
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
    pub id: uuid::Uuid,
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
    pub chat_state: ChatState,
    pub profile_state: ProfileState,
    pub dms_state: DMsState,
    pub settings_state: SettingsState,
    pub notifications_state: NotificationsState,
    pub post_detail_state: Option<PostDetailState>,
    pub viewing_post_detail: bool,
    pub show_help: bool,
    pub input_mode: InputMode,
    pub composer_state: ComposerState,
    pub friends_state: FriendsState,
    pub hashtags_state: HashtagsState,
    pub user_search_state: UserSearchState,
    pub user_profile_view: Option<UserProfileViewState>,
    pub log_config: crate::logging::LogConfig,
    pub pending_vote_tasks:
        Vec<tokio::task::JoinHandle<(uuid::Uuid, crate::api::ApiResult<serde_json::Value>)>>,
    /// Latest version available on crates.io (if newer than current)
    pub update_available: Option<String>,
    /// GitHub repo detected from the launch directory; decides the community
    pub launch_repo: Option<crate::repo_context::RepoRef>,
    /// The community whose board is currently shown
    pub community: Option<CommunityContext>,
    /// Why the launch repo's community could not be entered (repo mode only)
    pub community_error: Option<String>,
    /// Joined-communities list shown when launched outside a repo
    pub home_state: HomeState,
    /// Starred-repo browser for joining additional communities
    pub community_browser_state: CommunityBrowserState,
    /// Community settings modal (role, member count, claim)
    pub show_community_modal: bool,
    /// Members of the current community, loaded when the community modal opens
    pub community_members: Vec<crate::api::CommunityMemberInfo>,
    /// Realtime transport and applied-event bookkeeping
    pub realtime_state: RealtimeState,
}

/// Realtime transport state plus idempotency markers for pushed events.
pub struct RealtimeState {
    pub status: RealtimeConnectionStatus,
    pub last_error: Option<String>,
    pub last_event_at: Option<Instant>,
    pub unread_notifications: Vec<NotificationUnreadCount>,
    pub last_channel_message: Option<ChannelMessageEvent>,
    pub channel_message_count: u64,
    pub seen_posts: HashSet<Uuid>,
    pub seen_dm_messages: HashSet<Uuid>,
    pub seen_notifications: HashSet<Uuid>,
    pub seen_channel_messages: HashSet<Uuid>,
}

/// Notifications overlay state.
pub struct NotificationsState {
    pub show: bool,
    pub notifications: Vec<Notification>,
    pub selected_index: usize,
    pub loading: bool,
    pub error: Option<String>,
    pub loaded: bool,
}

impl RealtimeState {
    pub fn new() -> Self {
        Self {
            status: RealtimeConnectionStatus::Disabled,
            last_error: None,
            last_event_at: None,
            unread_notifications: Vec::new(),
            last_channel_message: None,
            channel_message_count: 0,
            seen_posts: HashSet::new(),
            seen_dm_messages: HashSet::new(),
            seen_notifications: HashSet::new(),
            seen_channel_messages: HashSet::new(),
        }
    }

    pub fn is_polling_fallback_active(&self) -> bool {
        self.status.uses_polling_fallback()
    }
}

/// The community the board is scoped to, with the caller's standing in it
#[derive(Debug, Clone)]
pub struct CommunityContext {
    pub id: Uuid,
    pub owner: String,
    pub name: String,
    pub role: Option<fido_types::MembershipRole>,
    pub member_count: i64,
    pub claimed: bool,
}

impl CommunityContext {
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

/// Home mode state: the joined-communities list (launched outside a repo)
pub struct HomeState {
    pub communities: Vec<crate::api::CommunityViewResponse>,
    pub list_state: ListState,
    pub loading: bool,
    pub loaded: bool,
    pub error: Option<String>,
}

impl HomeState {
    pub fn new() -> Self {
        Self {
            communities: Vec::new(),
            list_state: ListState::default(),
            loading: false,
            loaded: false,
            error: None,
        }
    }

    pub fn selected(&self) -> Option<&crate::api::CommunityViewResponse> {
        self.list_state
            .selected()
            .and_then(|i| self.communities.get(i))
    }
}

/// Community chat state for the selected repo's default channel.
pub struct ChatState {
    pub channels: Vec<Channel>,
    pub selected_channel_index: usize,
    pub messages: Vec<Message>,
    pub list_state: ListState,
    pub message_textarea: TextArea<'static>,
    pub loading: bool,
    pub loaded: bool,
    pub loading_older: bool,
    pub pending_older_load: bool,
    pub at_history_start: bool,
    pub error: Option<String>,
}

impl ChatState {
    pub fn selected_channel(&self) -> Option<&Channel> {
        self.channels.get(self.selected_channel_index)
    }
}

/// Starred GitHub repos annotated with Fido community state.
pub struct CommunityBrowserState {
    pub show: bool,
    pub repos: Vec<crate::api::BrowseCommunityResponse>,
    /// Incremental fuzzy filter typed by the user.
    pub filter: String,
    /// Indices into `repos` that survive `filter`, best match first. Empty
    /// filter means every repo in its original order.
    pub visible: Vec<usize>,
    pub list_state: ListState,
    pub loading: bool,
    pub loaded: bool,
    pub joining: bool,
    pub error: Option<String>,
    pub message: Option<String>,
}

impl CommunityBrowserState {
    pub fn new() -> Self {
        Self {
            show: false,
            repos: Vec::new(),
            filter: String::new(),
            visible: Vec::new(),
            list_state: ListState::default(),
            loading: false,
            loaded: false,
            joining: false,
            error: None,
            message: None,
        }
    }

    pub fn selected(&self) -> Option<&crate::api::BrowseCommunityResponse> {
        self.list_state
            .selected()
            .and_then(|row| self.visible.get(row))
            .and_then(|&index| self.repos.get(index))
    }

    /// Rows currently on screen, in display order.
    pub fn visible_repos(&self) -> impl Iterator<Item = &crate::api::BrowseCommunityResponse> {
        self.visible
            .iter()
            .filter_map(|&index| self.repos.get(index))
    }

    /// Recompute `visible` from `filter` and move the selection to the top.
    /// Called on every keystroke and whenever `repos` is replaced.
    pub fn apply_filter(&mut self) {
        self.visible = fuzzy_filter(&self.repos, &self.filter);
        if self.visible.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }
}

/// Rank repos against `filter` by fuzzy match on their full name. Ties break on
/// original order so the list stays stable while typing.
fn fuzzy_filter(repos: &[crate::api::BrowseCommunityResponse], filter: &str) -> Vec<usize> {
    if filter.trim().is_empty() {
        return (0..repos.len()).collect();
    }

    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(filter, CaseMatching::Ignore, Normalization::Smart);
    let mut scored: Vec<(usize, u32)> = repos
        .iter()
        .enumerate()
        .filter_map(|(index, repo)| {
            let mut buf = Vec::new();
            let haystack = Utf32Str::new(&repo.full_name, &mut buf);
            pattern.score(haystack, &mut matcher).map(|s| (index, s))
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    scored.into_iter().map(|(index, _)| index).collect()
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

/// DM selection state - cleaner than Option<usize> with magic values
#[derive(Debug, Clone, PartialEq)]
pub enum DMSelection {
    /// "New Conversation" button is selected
    NewConversation,
    /// A pending draft conversation (not yet sent)
    PendingDraft,
    /// A pending incoming DM request at the given index
    Request(usize),
    /// An existing conversation at the given index
    Conversation(usize),
}

impl DMSelection {
    /// Check if the "New Conversation" button is selected
    pub fn is_new_conversation(&self) -> bool {
        matches!(self, DMSelection::NewConversation)
    }

    /// Check if a pending draft is selected
    pub fn is_pending_draft(&self) -> bool {
        matches!(self, DMSelection::PendingDraft)
    }

    /// Get the conversation index if one is selected
    pub fn conversation_index(&self) -> Option<usize> {
        match self {
            DMSelection::Conversation(idx) => Some(*idx),
            _ => None,
        }
    }
}

/// A pending incoming DM request (someone who messaged us before we shared a community)
#[derive(Debug, Clone)]
pub struct DmRequest {
    pub from_user_id: uuid::Uuid,
    pub from_username: String,
}

/// DMs tab state
pub struct DMsState {
    pub conversations: Vec<Conversation>,
    pub conversations_loaded: bool, // Distinguish "loaded but empty" from "not loaded yet"
    pub selection: DMSelection,     // Clean enum-based selection
    pub messages: Vec<fido_types::DirectMessage>,
    pub loading: bool,
    pub error: Option<String>,
    pub message_textarea: TextArea<'static>, // TextArea for message input
    pub show_new_conversation_modal: bool,
    pub new_conversation_username: String,
    pub pending_conversation_username: Option<String>, // Username for new conversation not yet created
    pub unread_counts: std::collections::HashMap<uuid::Uuid, usize>, // user_id -> unread count
    pub current_conversation_user: Option<uuid::Uuid>, // Track open conversation
    pub needs_message_load: bool,                      // Flag to trigger message loading
    /// Search results for starting a new DM conversation
    pub new_conversation_results: Vec<UserSearchResult>,
    /// Whether the new conversation search is loading
    pub new_conversation_loading: bool,
    /// Selected index in new conversation modal
    pub new_conversation_selected_index: usize,
    /// Search mode for new conversation modal
    pub new_conversation_search_mode: bool,
    /// Search query for new conversation modal
    pub new_conversation_search_query: String,
    /// Pending incoming DM requests awaiting accept/decline
    pub pending_requests: Vec<DmRequest>,
}

impl DMsState {
    /// Navigate down in the conversation list.
    /// Order: NewConversation -> PendingDraft (if any) -> Request(0..n) -> Conversation(0..m)
    pub fn navigate_down(&mut self) {
        match &self.selection {
            DMSelection::NewConversation => {
                if self.pending_conversation_username.is_some() {
                    self.selection = DMSelection::PendingDraft;
                } else if !self.pending_requests.is_empty() {
                    self.selection = DMSelection::Request(0);
                } else if !self.conversations.is_empty() {
                    self.selection = DMSelection::Conversation(0);
                }
                // If nothing else, stay on NewConversation
            }
            DMSelection::PendingDraft => {
                if !self.pending_requests.is_empty() {
                    self.selection = DMSelection::Request(0);
                } else if !self.conversations.is_empty() {
                    self.selection = DMSelection::Conversation(0);
                }
            }
            DMSelection::Request(idx) => {
                if *idx < self.pending_requests.len().saturating_sub(1) {
                    self.selection = DMSelection::Request(idx + 1);
                } else if !self.conversations.is_empty() {
                    self.selection = DMSelection::Conversation(0);
                }
            }
            DMSelection::Conversation(idx) => {
                // Move to next conversation if not at end
                if *idx < self.conversations.len().saturating_sub(1) {
                    self.selection = DMSelection::Conversation(idx + 1);
                }
            }
        }
    }

    /// Navigate up in the conversation list.
    pub fn navigate_up(&mut self) {
        match &self.selection {
            DMSelection::NewConversation => {
                // Already at top, do nothing
            }
            DMSelection::PendingDraft => {
                // Go back to "New Conversation"
                self.selection = DMSelection::NewConversation;
            }
            DMSelection::Request(idx) => {
                if *idx == 0 {
                    if self.pending_conversation_username.is_some() {
                        self.selection = DMSelection::PendingDraft;
                    } else {
                        self.selection = DMSelection::NewConversation;
                    }
                } else {
                    self.selection = DMSelection::Request(idx - 1);
                }
            }
            DMSelection::Conversation(idx) => {
                if *idx == 0 {
                    // At first conversation, go to last request, pending draft, or "New Conversation"
                    if !self.pending_requests.is_empty() {
                        self.selection = DMSelection::Request(self.pending_requests.len() - 1);
                    } else if self.pending_conversation_username.is_some() {
                        self.selection = DMSelection::PendingDraft;
                    } else {
                        self.selection = DMSelection::NewConversation;
                    }
                } else {
                    self.selection = DMSelection::Conversation(idx - 1);
                }
            }
        }
    }

    /// Re-select the request list after accepting/declining the request at
    /// `acted_idx` (the conversation reload resets selection). Keeps the user
    /// in the requests section while requests remain; otherwise leaves the
    /// reload's default selection.
    pub fn restore_request_selection(&mut self, acted_idx: usize) {
        if !self.pending_requests.is_empty() {
            self.selection = DMSelection::Request(acted_idx.min(self.pending_requests.len() - 1));
        }
    }
}

/// Conversation summary
#[derive(Debug, Clone)]
pub struct Conversation {
    pub other_user_id: uuid::Uuid,
    pub other_username: String,
    pub last_message: String,
    pub unread_count: i32,
    pub state: fido_types::DmConversationState,
    pub initiated_by_me: bool,
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
    pub user_id: uuid::Uuid,
    pub username: String,
    pub bio: Option<String>,
    pub join_date: String,
    pub follower_count: usize,
    pub following_count: usize,
    pub post_count: usize,
    pub relationship: fido_types::RelationshipStatus,
    pub error: Option<String>,
}

/// Filter type for posts
#[derive(Debug, Clone, PartialEq)]
pub enum PostFilter {
    All,
    Multi {
        hashtags: Vec<String>,
        users: Vec<String>,
    },
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
    /// Track if at end of feed (for "End of Feed" indicator)
    pub at_end_of_feed: bool,
    /// Admin queue for pending top-level community threads.
    pub pending_threads: Vec<Post>,
    pub pending_threads_list_state: ListState,
    pub show_approval_queue: bool,
    pub pending_threads_loading: bool,
    pub pending_threads_loaded: bool,
    pub pending_threads_error: Option<String>,
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
                children_map.entry(parent_id).or_default().push(reply.id);
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
    Chat,
    DMs,
    Profile,
    Settings,
}

impl Tab {
    pub fn next(&self) -> Self {
        match self {
            Tab::Posts => Tab::Chat,
            Tab::Chat => Tab::DMs,
            Tab::DMs => Tab::Profile,
            Tab::Profile => Tab::Settings,
            Tab::Settings => Tab::Posts,
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            Tab::Posts => Tab::Settings,
            Tab::Chat => Tab::Posts,
            Tab::DMs => Tab::Chat,
            Tab::Profile => Tab::DMs,
            Tab::Settings => Tab::Profile,
        }
    }
}
