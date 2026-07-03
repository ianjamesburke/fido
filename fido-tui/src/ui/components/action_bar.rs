use crate::app::App;

/// Action bar hints for the current view.
pub fn action_bar_text(app: &App) -> &'static str {
    // Don't show page actions when modal is open (modal has its own footer)
    if app.viewing_post_detail {
        if let Some(detail_state) = &app.post_detail_state {
            if detail_state.show_full_post_modal {
                return "";
            }
        }
    }

    match app.current_tab {
        crate::app::Tab::Posts => {
            if app.show_community_modal {
                if app.community.as_ref().map(|c| !c.claimed).unwrap_or(false) {
                    "c: Claim admin | Esc: Close"
                } else {
                    "Esc: Close"
                }
            } else if app.community_error.is_some() {
                "r: Retry"
            } else if app.is_home_list_active() {
                "↑/↓/j/k: Navigate | Enter: Open community"
            } else {
                "u/d: Vote | n: Post | i: Community | f: Filter | s: Search | Space: View"
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
