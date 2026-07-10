use crate::app::App;

/// Action bar hints for the current view.
pub fn action_bar_text(app: &App) -> &'static str {
    // Don't show page actions when modal is open (modal has its own footer)
    if app.community_browser_state.show {
        return "Enter: Open/Join | r: Reload | b/Esc: Close";
    }

    if app.viewing_post_detail {
        if let Some(detail_state) = &app.post_detail_state {
            if detail_state.show_full_post_modal {
                return "";
            }
        }
    }

    match app.current_tab {
        crate::app::Tab::Posts => {
            if app.posts_state.show_approval_queue {
                "↑/↓/j/k: Navigate | a: Approve | x: Reject | Esc: Board"
            } else if app.show_community_modal {
                if app.community.as_ref().map(|c| !c.claimed).unwrap_or(false) {
                    "c: Claim admin | Esc: Close"
                } else {
                    "Esc: Close"
                }
            } else if app.community_error.is_some() {
                "r: Retry"
            } else if app.is_home_list_active() {
                "↑/↓/j/k: Navigate | Enter: Open community | b: Browse starred"
            } else if app
                .posts_state
                .list_state
                .selected()
                .and_then(|index| app.posts_state.posts.get(index))
                .map(|post| post.github_kind.is_some())
                .unwrap_or(false)
            {
                if app.is_away_from_launch_community() {
                    "u/d: Vote | Space: View | o: Open on GitHub | b: Browse | Esc: Launch repo"
                } else {
                    "u/d: Vote | Space: View | o: Open on GitHub | i: Community | b: Browse"
                }
            } else if app.is_current_community_admin() {
                if app.is_away_from_launch_community() {
                    "u/d: Vote | n: Post | a: Approvals | b: Browse | Esc: Launch repo | Space: View"
                } else {
                    "u/d: Vote | n: Post | a: Approvals | i: Community | b: Browse | Space: View"
                }
            } else if app.is_away_from_launch_community() {
                "u/d: Vote | n: Post | b: Browse | Esc: Launch repo | Space: View"
            } else {
                "u/d: Vote | n: Post | i: Community | b: Browse | Space: View"
            }
        }
        crate::app::Tab::Chat => {
            if app.community.is_none() {
                "Open a community to use chat"
            } else if app.chat_state.channels.is_empty() {
                "No channels available"
            } else if app.input_mode == crate::app::InputMode::Typing {
                "Enter: Send | Esc: Clear | Type: Compose"
            } else {
                "↑/↓/j/k: Navigate | Type: Compose | Enter: Focus input"
            }
        }
        crate::app::Tab::DMs => {
            let has_active_conversation = app.dms_state.selection.conversation_index().is_some();
            let has_pending_draft = app.dms_state.pending_conversation_username.is_some();
            let can_compose = has_active_conversation || has_pending_draft;
            let is_request_selected =
                matches!(app.dms_state.selection, crate::app::DMSelection::Request(_));

            // Only advertise keys that work for the current selection:
            // a/x act only on a selected request; p opens the profile of a
            // selected conversation (both in Navigation mode).
            if is_request_selected {
                "↑/↓/j/k: Navigate | a: Accept | x: Decline"
            } else if app.dms_state.selection.is_new_conversation() {
                "Enter/N: Start New Conversation | ↑/↓/j/k: Navigate | Esc: Back"
            } else if can_compose {
                if has_active_conversation {
                    "↑/↓/j/k: Navigate | p: Profile | Type to compose | Enter: Send | Esc: Clear"
                } else {
                    "↑/↓/j/k: Navigate | Type to compose | Enter: Send | Esc: Clear"
                }
            } else {
                "↑/↓/j/k: Navigate | Enter: Select conversation | N: New Conversation"
            }
        }
        crate::app::Tab::Profile => "e: Edit Bio | f: Friends",
        crate::app::Tab::Settings => "←/→/h/l: Change | s: Save",
    }
}
