use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::components::action_bar::action_bar_text;
use super::components::banners::{render_error_banner, render_message_banner};
use super::components::empty_state::render_empty_state_block;
use super::components::footer::render_footer_with_style;
use super::components::layout::{auth_layout, banner_layout, main_layout};
use super::components::list::styled_list;
use super::components::modal::centered_rect;
use super::components::panel::render_panel_lines;
use super::formatting::*;
use super::modals::*;
use super::theme::{get_theme_colors, ThemeColors};
use crate::app::{App, Conversation, DMSelection, FeedEntry};
use crate::{log_modal_state, log_rendering};

pub fn render_auth_screen(frame: &mut Frame, app: &mut App) {
    let theme = get_theme_colors(app);
    let layout = auth_layout(frame);

    // Header
    let header = Paragraph::new("Fido - Terminal Social Platform")
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, layout.header);

    // Main content - ASCII logo
    let mut lines = vec![Line::from("")];

    // Fido ASCII art logo with theme colors
    const LOGO_LINES: &[&str] = &[
        "  _____ _     _       ",
        " |  ___(_) __| | ___  ",
        " | |_  | |/ _` |/ _ \\ ",
        " |  _| | | (_| | (_) |",
        " |_|   |_|\\__,_|\\___/ ",
    ];

    for logo_line in LOGO_LINES {
        lines.push(Line::from(Span::styled(
            *logo_line,
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(""));

    if app.auth_state.github_auth_in_progress {
        // Show GitHub Device Flow in progress
        lines.push(Line::from(Span::styled(
            "GitHub Device Authorization",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        if let Some(user_code) = &app.auth_state.github_user_code {
            lines.push(Line::from(Span::styled(
                "Step 1: Copy this code:",
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                user_code.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED),
            )));
            lines.push(Line::from(""));
        }

        if let Some(uri) = &app.auth_state.github_verification_uri {
            lines.push(Line::from(Span::styled(
                "Step 2: Open this link in your browser:",
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(Span::styled(
                uri.clone(),
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Press 'o' to open this link automatically",
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(""));
        }

        lines.push(Line::from(Span::styled(
            "Step 3: Paste the code on GitHub and approve access",
            Style::default().fg(Color::White),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Esc to cancel",
            Style::default().fg(Color::White),
        )));
    } else if app.auth_state.loading {
        lines.push(Line::from(Span::styled(
            "Loading...",
            Style::default().fg(Color::White),
        )));
    } else if let Some(error) = &app.auth_state.error {
        lines.push(Line::from(Span::styled(
            error.clone(),
            Style::default().fg(theme.error),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press any key to continue",
            Style::default().fg(Color::White),
        )));
    } else if app.auth_state.test_users.is_empty() {
        lines.push(Line::from(Span::styled(
            "Choose authentication method:",
            Style::default().fg(Color::White),
        )));
        lines.push(Line::from(""));

        if app.auth_state.show_github_option {
            lines.push(Line::from(Span::styled(
                "Press 'g' to login with GitHub",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
        }

        lines.push(Line::from(Span::styled(
            "Press 'l' to load test users (development only)",
            Style::default().fg(Color::White),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Select a test user (development only):",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        // Show only first 3 test users (alice, bob, charlie)
        for (i, user) in app.auth_state.test_users.iter().take(3).enumerate() {
            let style = if i == app.auth_state.selected_index {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if i == app.auth_state.selected_index {
                "▶ "
            } else {
                "  "
            };
            let bio = user.bio.as_deref().unwrap_or("No bio");
            lines.push(Line::from(Span::styled(
                format!("{}{} - {}", prefix, user.username, bio),
                style,
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Enter to login with test user",
            Style::default().fg(Color::White),
        )));

        if app.auth_state.show_github_option {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Press 'g' to login with GitHub instead",
                Style::default().fg(Color::White),
            )));
        }
    }

    let content = Paragraph::new(lines).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Authentication"),
    );
    frame.render_widget(content, layout.content);

    // Footer
    let footer_text = if app.auth_state.github_auth_in_progress {
        "Esc: Cancel | q: Quit"
    } else if app.auth_state.test_users.is_empty() {
        if app.auth_state.show_github_option {
            "g: GitHub Login | l: Load test users | q/Esc: Quit"
        } else {
            "l: Load test users | q/Esc: Quit"
        }
    } else if app.auth_state.show_github_option {
        "↑/k: Up | ↓/j: Down | Enter: Login | g: GitHub | q/Esc: Quit"
    } else {
        "↑/k: Up | ↓/j: Down | Enter: Login | q/Esc: Quit"
    };

    render_footer_with_style(
        frame,
        layout.footer,
        footer_text,
        &theme,
        Style::default().fg(Color::White),
    );
}

/// Render the main screen with a persistent Discord-style rail.
pub fn render_main_screen(frame: &mut Frame, app: &mut App) {
    let layout = main_layout(frame, app);
    let area = frame.area();

    render_workspace_header(frame, app, layout.header);

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(0)])
        .split(layout.content);

    render_left_rail(frame, app, content_chunks[0]);

    // Render selected rail content.
    match app.current_tab {
        crate::app::Tab::Posts => {
            render_posts_tab_with_data(frame, app, content_chunks[1]);
        }
        crate::app::Tab::Chat => render_chat_tab(frame, app, content_chunks[1]),
        crate::app::Tab::DMs => render_dms_tab(frame, app, content_chunks[1]),
        crate::app::Tab::Profile => render_profile_tab(frame, app, content_chunks[1]),
        crate::app::Tab::Settings => render_settings_tab(frame, app, content_chunks[1]),
    }

    // Render page-specific actions bar (NEW)
    render_page_actions(frame, app, layout.actions);

    // Render global footer
    render_global_footer(frame, app, layout.footer);

    // Render modals (in priority order - LAST rendered = TOP of stack)

    // ============================================================================
    // MODAL RENDERING PATTERN - CRITICAL FOR CORRECT LAYERING
    // ============================================================================
    //
    // This section implements the correct modal layering pattern that ensures:
    // 1. Background modals remain visible when foreground modals open on top
    // 2. Keyboard input is only handled by the topmost modal
    // 3. No flickering or visual glitches during modal transitions
    //
    // KEY PRINCIPLES:
    // - Modals are rendered in Z-order (bottom to top)
    // - Each modal renders independently based on its state flag
    // - NO conditional rendering based on other modal states
    // - Dimmed backgrounds are NOT used (they interfere with layering)
    //
    // RENDERING ORDER (bottom to top):
    // 1. Thread/Post detail modal (background)
    // 2. Delete confirmation modal (if active)
    // 3. Composer modal (foreground - new post, reply, edit)
    // 4. Other modals (friends, filters, help, etc.)
    //
    // WHY THIS WORKS:
    // - Thread modal renders when show_full_post_modal=true, regardless of composer state
    // - Composer modal renders AFTER thread modal, so it appears on top
    // - Each modal uses Clear widget to ensure clean rendering
    // - Ratatui's rendering order ensures later renders appear on top
    //
    // PREVIOUS BUG:
    // - Thread modal rendering was conditional on !composer_state.is_open()
    // - This caused thread modal to disappear when composer opened
    // - Dimmed background logic was interfering with modal visibility
    //
    // FIX APPLIED:
    // - Removed conditional rendering based on composer state
    // - Removed dimmed background (it was causing the thread modal to be skipped)
    // - Each modal now renders independently based only on its own state
    //
    // REFERENCE IMPLEMENTATION:
    // - Profile modal (render_user_profile_view) works correctly as a reference
    // - It renders after composer and appears on top without issues
    // ============================================================================

    // Log modal state before rendering (for debugging)
    let composer_mode = if let Some(mode) = &app.composer_state.mode {
        format!("{:?}", mode)
    } else {
        "None".to_string()
    };
    log_modal_state!(
        app.log_config,
        "viewing_post_detail={}, show_full_post_modal={}, composer_open={}, composer_mode={}",
        app.viewing_post_detail,
        app.post_detail_state
            .as_ref()
            .map(|s| s.show_full_post_modal)
            .unwrap_or(false),
        app.composer_state.is_open(),
        composer_mode
    );

    // ============================================================================
    // LAYER 1: Thread/Post Detail Modal (Background)
    // ============================================================================
    // Renders the full post modal for viewing nested reply threads.
    // This modal MUST render regardless of composer state to remain visible
    // in the background when the composer opens on top.
    let show_full_post_modal = app
        .post_detail_state
        .as_ref()
        .map(|s| s.show_full_post_modal)
        .unwrap_or(false);

    if show_full_post_modal {
        log_rendering!(app.log_config, "Rendering thread modal (full post modal)");
        render_full_post_modal(frame, app, area);
    }

    // ============================================================================
    // LAYER 2: Delete Confirmation Modal
    // ============================================================================
    // Renders AFTER thread modal so it appears on top when active.
    let show_delete_confirmation = app
        .post_detail_state
        .as_ref()
        .map(|s| s.show_delete_confirmation)
        .unwrap_or(false);
    if show_delete_confirmation {
        render_delete_confirmation_modal(frame, app, area);
    }

    // ============================================================================
    // LAYER 3: Composer Modal (Foreground)
    // ============================================================================
    // Unified composer for new posts, replies, edits, and bio editing.
    // Renders AFTER thread modal to appear on top, allowing users to see
    // the thread context while composing a reply.
    if app.composer_state.is_open() {
        log_rendering!(
            app.log_config,
            "Rendering composer modal (mode: {})",
            composer_mode
        );
        render_unified_composer_modal(frame, app, area);
    }

    if app.dms_state.show_new_conversation_modal {
        render_new_conversation_modal(frame, app, area);
    }

    // Render save confirmation modal (before help modal)
    if app.settings_state.show_save_confirmation {
        render_save_confirmation_modal(frame, app, area);
    }

    // Render friends modal
    if app.friends_state.show_friends_modal {
        render_friends_modal(frame, app, area);
    }

    // Render user search modal
    if app.user_search_state.show_modal {
        render_user_search_modal(frame, app, area);
    }

    // Render user profile view
    if app.user_profile_view.is_some() {
        render_user_profile_view(frame, app, area);
    }

    // Render community settings modal
    if app.show_community_modal {
        render_community_modal(frame, app, area);
    }

    if app.community_browser_state.show {
        render_community_browser(frame, app, area);
    }

    // Render help modal (highest priority - render last)
    if app.show_help {
        render_help_modal(frame, app, area);
    }
}

fn render_workspace_header(frame: &mut Frame, app: &App, area: Rect) {
    let theme = get_theme_colors(app);
    let title = match &app.community {
        Some(c) => format!(" Fido · {} ", c.full_name()),
        None if app.launch_repo.is_none() => " Fido · Home ".to_string(),
        None => " Fido · Repo community ".to_string(),
    };
    let status = format!(
        "{} · {}",
        app.auth_state
            .current_user
            .as_ref()
            .map(|user| format!("@{}", user.username))
            .unwrap_or_else(|| "signed out".to_string()),
        app.realtime_state.status.label()
    );
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            title,
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(status, Style::default().fg(theme.text_dim)),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, area);
}

fn render_left_rail(frame: &mut Frame, app: &App, area: Rect) {
    let theme = get_theme_colors(app);
    let total_unread: usize = app.dms_state.unread_counts.values().sum();
    let request_count = app.dms_state.pending_requests.len();

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        "COMMUNITIES",
        Style::default()
            .fg(theme.text_dim)
            .add_modifier(Modifier::BOLD),
    )));

    let board_label = if let Some(community) = &app.community {
        format!("# {}", community.full_name())
    } else if app.is_home_list_active() {
        "Home".to_string()
    } else {
        "Board".to_string()
    };
    lines.push(rail_line(
        app.current_tab == crate::app::Tab::Posts,
        &board_label,
        theme.success,
        &theme,
    ));

    let chat_label = app
        .chat_state
        .selected_channel()
        .map(|channel| format!("# {}", channel.name))
        .unwrap_or_else(|| "# chat".to_string());
    lines.push(rail_line(
        app.current_tab == crate::app::Tab::Chat,
        &chat_label,
        theme.secondary,
        &theme,
    ));

    for community in app.home_state.communities.iter().take(6) {
        let label = format!(
            "  {}/{}",
            community.community.owner, community.community.name
        );
        lines.push(Line::from(Span::styled(
            label,
            Style::default().fg(theme.text_dim),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "DIRECT MESSAGES",
        Style::default()
            .fg(theme.text_dim)
            .add_modifier(Modifier::BOLD),
    )));
    let dm_label = if total_unread > 0 {
        format!("DMs ({})", total_unread)
    } else if request_count > 0 {
        format!(
            "DMs · {} request{}",
            request_count,
            if request_count == 1 { "" } else { "s" }
        )
    } else {
        "DMs".to_string()
    };
    lines.push(rail_line(
        app.current_tab == crate::app::Tab::DMs,
        &dm_label,
        theme.accent,
        &theme,
    ));
    for conversation in app.dms_state.conversations.iter().take(5) {
        let unread = app
            .dms_state
            .unread_counts
            .get(&conversation.other_user_id)
            .copied()
            .unwrap_or(0);
        let label = if unread > 0 {
            format!("  @{} ({})", conversation.other_username, unread)
        } else {
            format!("  @{}", conversation.other_username)
        };
        lines.push(Line::from(Span::styled(
            label,
            Style::default().fg(theme.text_dim),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "TOOLS",
        Style::default()
            .fg(theme.text_dim)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(rail_line(
        app.current_tab == crate::app::Tab::Profile,
        "Profile",
        theme.primary,
        &theme,
    ));
    lines.push(rail_line(
        app.current_tab == crate::app::Tab::Settings,
        "Settings",
        theme.primary,
        &theme,
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "b Browse starred repos",
        Style::default().fg(theme.text_dim),
    )));

    let rail = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(" Fido "));
    frame.render_widget(rail, area);
}

fn rail_line(selected: bool, label: &str, accent: Color, theme: &ThemeColors) -> Line<'static> {
    let prefix = if selected { ">" } else { " " };
    let style = if selected {
        Style::default()
            .fg(accent)
            .bg(theme.highlight_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text)
    };
    Line::from(Span::styled(format!("{} {}", prefix, label), style))
}

fn render_community_browser(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);
    let popup_area = centered_rect(76, 76, area);
    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(popup_area);

    let items: Vec<ListItem> = if app.community_browser_state.loading {
        vec![ListItem::new(Line::from(Span::styled(
            "Loading starred repositories...",
            Style::default().fg(theme.text_dim),
        )))]
    } else if let Some(error) = &app.community_browser_state.error {
        vec![ListItem::new(Line::from(Span::styled(
            error.clone(),
            Style::default().fg(theme.error),
        )))]
    } else if app.community_browser_state.repos.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "No starred repositories found.",
            Style::default().fg(theme.text_dim),
        )))]
    } else {
        app.community_browser_state
            .repos
            .iter()
            .map(|repo| {
                let status = if repo.private {
                    "private"
                } else if repo.membership.is_some() {
                    "joined"
                } else if repo.community.is_some() {
                    "available"
                } else {
                    "new"
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<38}", repo.full_name),
                        Style::default().fg(theme.text),
                    ),
                    Span::styled(status, Style::default().fg(theme.text_dim)),
                ]))
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Browse starred repos "),
        )
        .highlight_style(
            Style::default()
                .fg(theme.success)
                .bg(theme.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, chunks[0], &mut app.community_browser_state.list_state);

    let footer = if app.community_browser_state.joining {
        "Joining..."
    } else {
        "Enter: Open/Join | r: Reload | b/Esc: Close"
    };
    render_footer_with_style(
        frame,
        chunks[1],
        footer,
        &theme,
        Style::default().fg(theme.text_dim),
    );
}

/// Render tab header
#[allow(dead_code)]
pub fn render_tab_header(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    // Calculate total unread count for DMs
    let total_unread: usize = app.dms_state.unread_counts.values().sum();

    let board_label = if app.is_home_list_active() {
        "Home"
    } else {
        "Board"
    };
    let tabs = [board_label, "Chat", "DMs", "Profile", "Settings"];
    let current_index = match app.current_tab {
        crate::app::Tab::Posts => 0,
        crate::app::Tab::Chat => 1,
        crate::app::Tab::DMs => 2,
        crate::app::Tab::Profile => 3,
        crate::app::Tab::Settings => 4,
    };

    let mut tab_spans = vec![];
    for (i, tab) in tabs.iter().enumerate() {
        let style = if i == current_index {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.text_dim)
        };

        // Add unread badge for DMs tab
        let tab_text = if i == 2 && total_unread > 0 {
            format!(" {} ({}) ", tab, total_unread)
        } else {
            format!(" {} ", tab)
        };

        tab_spans.push(Span::styled(tab_text, style));
        if i < tabs.len() - 1 {
            tab_spans.push(Span::raw(" | "));
        }
    }

    let header = Paragraph::new(Line::from(tab_spans))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, area);
}

/// Get context-appropriate action text for the current view
/// Render page-specific actions bar (centered, with wrapping support)
pub fn render_page_actions(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    // Clear the area first to prevent text bleeding from previous renders.
    // This is especially important when terminal is resized or content changes.
    frame.render_widget(Clear, area);

    // Fill with background color to ensure complete clearing
    let background = Block::default().style(Style::default().bg(theme.background));
    frame.render_widget(background, area);

    let text = action_bar_text(app);
    let widget = Paragraph::new(text)
        .style(Style::default().fg(theme.text).bg(theme.background))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

/// Render global footer with global shortcuts only
pub fn render_global_footer(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    // Clear the area first to prevent text bleeding
    frame.render_widget(Clear, area);

    let footer_text = format!(
        "RT: {} | Tab/Shift+Tab: Rail | b: Browse repos | Shift+L: Logout | ?: Help | q/Esc: Quit",
        app.realtime_state.status.label()
    );
    render_footer_with_style(
        frame,
        area,
        &footer_text,
        &theme,
        Style::default().fg(theme.text_dim),
    );
}

/// Board title: repo community plus the caller's standing in it.
fn community_board_title(app: &App) -> String {
    match &app.community {
        Some(c) => {
            let role = c
                .role
                .map(|r| r.as_str().to_string())
                .unwrap_or_else(|| "not a member".to_string());
            let members = if c.member_count == 1 {
                "1 member".to_string()
            } else {
                format!("{} members", c.member_count)
            };
            let mut title = format!(" {} · {} · {} ", c.full_name(), role, members);
            if !c.claimed {
                title.push_str("· unclaimed ");
            }
            title
        }
        None => "Board".to_string(),
    }
}

/// Repo mode: the launch repo's community could not be opened.
fn render_community_error(frame: &mut Frame, app: &mut App, area: Rect, error: &str) {
    let theme = get_theme_colors(app);
    let widget = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            error.to_string(),
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press 'r' to retry.",
            Style::default().fg(theme.text_dim),
        )),
    ])
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true })
    .block(Block::default().borders(Borders::ALL).title(" Community "));
    frame.render_widget(widget, area);
}

/// Home mode: joined-communities list (launched outside a GitHub repo).
fn render_home_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);
    let title = " Your Communities ";

    if app.home_state.loading && app.home_state.communities.is_empty() {
        render_panel_lines(
            frame,
            area,
            title,
            create_loading_display("Loading communities...", &theme),
            &theme,
        );
        return;
    }

    if let Some(error) = &app.home_state.error {
        let widget = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                error.clone(),
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press 'r' to retry.",
                Style::default().fg(theme.text_dim),
            )),
        ])
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(widget, area);
        return;
    }

    if app.home_state.communities.is_empty() {
        let widget = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No communities yet",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Launch fido inside a GitHub repo directory to join its community.",
                Style::default().fg(theme.text_dim),
            )),
        ])
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(widget, area);
        return;
    }

    let items: Vec<ListItem> = app
        .home_state
        .communities
        .iter()
        .map(|view| {
            let role = view
                .membership
                .as_ref()
                .map(|m| m.role.as_str())
                .unwrap_or("member");
            let members = if view.member_count == 1 {
                "1 member".to_string()
            } else {
                format!("{} members", view.member_count)
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{}/{}", view.community.owner, view.community.name),
                    Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {} · {}", role, members),
                    Style::default().fg(theme.text_dim),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, area, &mut app.home_state.list_state);
}

/// Admin mode: pending top-level threads for the current community.
fn render_approval_queue(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);
    let title = " Pending Threads ";

    if app.posts_state.pending_threads_loading {
        render_panel_lines(
            frame,
            area,
            title,
            create_loading_display("Loading pending threads...", &theme),
            &theme,
        );
        return;
    }

    if let Some(error) = &app.posts_state.pending_threads_error {
        render_panel_lines(
            frame,
            area,
            title,
            create_error_display(error, Some("Press Esc to return to the board"), &theme),
            &theme,
        );
        return;
    }

    if app.posts_state.pending_threads.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No pending threads",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "New threads that require approval will appear here.",
                Style::default().fg(theme.text_dim),
            )),
        ])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(empty, area);
        return;
    }

    let selected = app.posts_state.pending_threads_list_state.selected();
    let width = area.width.saturating_sub(6) as usize;
    let items: Vec<ListItem> = app
        .posts_state
        .pending_threads
        .iter()
        .enumerate()
        .map(|(index, post)| {
            let is_selected = selected == Some(index);
            let header_style = if is_selected {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.primary)
            };
            let prefix = if is_selected { "> " } else { "  " };
            let mut lines = vec![Line::from(vec![
                Span::styled(prefix, header_style),
                Span::styled(format!("@{}", post.author_username), header_style),
                Span::raw(" • "),
                Span::styled(
                    format_timestamp(&post.created_at),
                    Style::default().fg(theme.text_dim),
                ),
            ])];
            for content_line in post.content.lines() {
                for wrapped in textwrap::wrap(content_line, width) {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", wrapped),
                        Style::default().fg(theme.text),
                    )));
                }
            }
            lines.push(Line::from(Span::styled(
                "  a: approve  x: reject",
                Style::default().fg(theme.text_dim),
            )));
            lines.push(Line::from(""));
            ListItem::new(lines)
        })
        .collect();

    let list =
        styled_list(items, &theme, None).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_stateful_widget(list, area, &mut app.posts_state.pending_threads_list_state);
}

/// Render Posts tab with global feed
pub fn render_posts_tab_with_data(frame: &mut Frame, app: &mut App, area: Rect) {
    // Log at start of render
    log_rendering!(app.log_config, "render_posts_tab_with_data: START");

    let theme = get_theme_colors(app);

    // Check if we need to show message or error banners
    // Don't show error banner if composer is open (it displays errors internally)
    let has_message = app.posts_state.message.is_some();
    let has_error = app.posts_state.error.is_some() && !app.composer_state.is_open();

    // Layout: Message banner (if present), Error banner (if present), posts feed
    let layout = banner_layout(area, has_message, has_error);

    // Message banner (success messages - auto-clear after 3 seconds)
    if let Some((message, _)) = &app.posts_state.message {
        if let Some(area) = layout.message {
            render_message_banner(frame, area, message, &theme);
        }
    }

    // Error banner (if present and not suppressed by composer)
    if has_error {
        if let Some(error) = &app.posts_state.error {
            if let Some(area) = layout.error {
                render_error_banner(frame, area, error, &theme);
            }
        }
    }

    // Main posts area (no inline compose box - use 'n' to open modal)
    let posts_area = layout.content;

    // Repo mode: joining the launch repo's community failed.
    if let Some(error) = app.community_error.clone() {
        render_community_error(frame, app, posts_area, &error);
        return;
    }

    // Home mode: show the joined-communities list instead of a board.
    if app.is_home_list_active() {
        render_home_list(frame, app, posts_area);
        return;
    }

    if app.posts_state.show_approval_queue {
        render_approval_queue(frame, app, posts_area);
        return;
    }

    let community_title = community_board_title(app);

    // Only show full-page loading on initial load (when there are no posts yet)
    if app.posts_state.loading && app.posts_state.posts.is_empty() {
        render_panel_lines(
            frame,
            posts_area,
            &community_title,
            create_loading_display("Loading posts...", &theme),
            &theme,
        );

        return;
    }

    if app.posts_state.feed_entries.is_empty()
        && !app.posts_state.loading
        && !app.posts_state.activity_loading
    {
        let theme = get_theme_colors(app);
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No posts yet",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press 'n' to create the first post.",
                Style::default().fg(theme.text_dim),
            )),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(community_title.as_str()),
        );
        frame.render_widget(empty, posts_area);

        return;
    }

    // Get theme colors
    let theme = get_theme_colors(app);

    let mut items: Vec<ListItem> = Vec::new();

    let available_width = posts_area.width.saturating_sub(BORDER_PADDING) as usize;

    // Add loading spinner at top if refreshing (when posts already exist)
    if app.posts_state.loading && !app.posts_state.posts.is_empty() {
        let style = Style::default()
            .fg(theme.warning)
            .add_modifier(Modifier::BOLD);
        let loading_item = create_centered_indicator("⟳ Loading...", style, available_width);
        items.push(ListItem::new(loading_item));
    }

    // Activity loading row (rendered right after the posts spinner, if any -
    // items_before_posts() counts this in exactly this position)
    if app.posts_state.activity_loading {
        let style = Style::default().fg(theme.text_dim);
        items.push(ListItem::new(create_centered_indicator(
            "⊙ loading repo activity...",
            style,
            available_width,
        )));
    }

    // Repo activity error - ambient dim line, not a banner
    if let Some(activity_error) = &app.posts_state.activity_error {
        let style = Style::default().fg(theme.text_dim);
        items.push(ListItem::new(vec![
            Line::from(Span::styled(format!("⊙ {}", activity_error), style)),
            Line::from(""),
        ]));
    }

    // Calculate available width for post content
    let post_width = (posts_area.width as usize).saturating_sub(4);

    // Get the currently selected feed entry (post or activity item)
    let selected_feed_entry = app.posts_state.selected_feed_entry();
    let selected_post_index = app
        .posts_state
        .list_state
        .selected()
        .and_then(|list_idx| app.posts_state.list_index_to_post_index(list_idx));

    let entry_count = app.posts_state.feed_entries.len();

    // Add posts and repo activity, interleaved as merged in feed_entries
    let feed_items: Vec<ListItem> = app
        .posts_state
        .feed_entries
        .iter()
        .enumerate()
        .flat_map(|(entry_idx, entry)| {
            let is_last_entry = entry_idx == entry_count - 1;

            match entry {
                FeedEntry::Post(i) => {
                    let post = &app.posts_state.posts[*i];
                    // Check if THIS post is the selected one
                    let is_selected = selected_post_index == Some(*i);

                    let mut post_lines: Vec<Line> = Vec::new();

                    // Post header with username and timestamp
                    let header_style = if is_selected {
                        Style::default()
                            .fg(theme.success)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.primary)
                    };

                    let prefix = if is_selected { "▶ " } else { "  " };
                    let timestamp = format_timestamp(&post.created_at);

                    post_lines.push(Line::from(vec![
                        Span::styled(prefix, header_style),
                        Span::styled(format!("@{}", post.author_username), header_style),
                        Span::raw(" • "),
                        Span::styled(timestamp, Style::default().fg(theme.text_dim)),
                    ]));

                    // Post content with hashtag highlighting and wrapping
                    let content_lines = format_post_content_with_width(
                        &post.content,
                        is_selected,
                        &theme,
                        post_width,
                    );
                    post_lines.extend(content_lines);

                    // Vote counts with highlighting for user's vote
                    let user_voted_up = post.user_vote.as_deref() == Some("up");
                    let user_voted_down = post.user_vote.as_deref() == Some("down");

                    let upvote_style = if user_voted_up {
                        Style::default()
                            .fg(theme.success)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.text_dim)
                    };

                    let downvote_style = if user_voted_down {
                        Style::default()
                            .fg(theme.error)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.text_dim)
                    };

                    post_lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(format!("↑ {} ", post.upvotes), upvote_style),
                        Span::styled(format!("↓ {} ", post.downvotes), downvote_style),
                        Span::styled(
                            format!("💬 {}", post.reply_count),
                            Style::default().fg(theme.text_dim),
                        ),
                    ]));

                    // Separator
                    if !is_last_entry {
                        post_lines.push(Line::from(""));
                    }

                    vec![ListItem::new(post_lines)]
                }
                FeedEntry::Activity(i) => {
                    let Some(activity) = app.posts_state.activity_items.get(*i) else {
                        return vec![];
                    };
                    let is_selected = selected_feed_entry == Some(FeedEntry::Activity(*i));

                    let rest_color = if is_selected {
                        theme.text
                    } else {
                        theme.text_dim
                    };
                    let prefix = if is_selected { "▶ " } else { "  " };
                    let glyph_color = activity_glyph_color(activity, &theme);

                    let text = truncated_activity_line(activity, available_width);
                    let mut chars = text.chars();
                    let glyph = chars.next().unwrap_or(' ');
                    let rest: String = chars.collect();

                    let mut lines = vec![Line::from(vec![
                        Span::styled(prefix, Style::default().fg(rest_color)),
                        Span::styled(glyph.to_string(), Style::default().fg(glyph_color)),
                        Span::styled(rest, Style::default().fg(rest_color)),
                    ])];

                    if !is_last_entry {
                        lines.push(Line::from(""));
                    }

                    vec![ListItem::new(lines)]
                }
            }
        })
        .collect();

    items.extend(feed_items);

    // Add end-of-feed message
    if !app.posts_state.feed_entries.is_empty() {
        let end_of_feed = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "─── End of feed ───",
                Style::default()
                    .fg(theme.text_dim)
                    .add_modifier(Modifier::DIM),
            )),
        ];
        items.push(ListItem::new(end_of_feed));
    }

    let posts_widget = styled_list(items, &theme, None).block(
        Block::default()
            .borders(Borders::ALL)
            .title(community_title),
    );

    frame.render_stateful_widget(posts_widget, posts_area, &mut app.posts_state.list_state);
}

/// Create a formatted error message display with optional help text
///
/// # Arguments
/// * `error_message` - The error message to display
/// * `help_text` - Optional help text (e.g., "Press Esc to go back")
/// * `theme` - The theme colors to use
fn create_error_display(
    error_message: &str,
    help_text: Option<&str>,
    theme: &ThemeColors,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            error_message.to_string(),
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if let Some(help) = help_text {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            help.to_string(),
            Style::default().fg(theme.text_dim),
        )));
    }

    lines
}

/// Create a formatted loading state display
///
/// # Arguments
/// * `message` - The loading message (e.g., "Loading posts...")
/// * `theme` - The theme colors to use
fn create_loading_display(message: &str, theme: &ThemeColors) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("⟳ {}", message),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Please wait",
            Style::default().fg(theme.text_dim),
        )),
    ]
}

/// Create a centered indicator item for the feed
///
/// # Arguments
/// * `text` - The text to display
/// * `style` - The style to apply to the text
/// * `available_width` - The available width for centering
fn create_centered_indicator(
    text: &str,
    style: Style,
    available_width: usize,
) -> Vec<Line<'static>> {
    let padding = (available_width.saturating_sub(text.len())) / 2;
    vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{}{}", " ".repeat(padding), text),
            style,
        )),
        Line::from(""),
    ]
}

/// `activity_line_text`, with the item's title truncated (with an ellipsis)
/// so the full line fits `available_width`.
fn truncated_activity_line(item: &fido_types::ActivityItem, available_width: usize) -> String {
    let full = activity_line_text(item);
    if full.chars().count() <= available_width {
        return full;
    }

    let mut untitled = item.clone();
    untitled.title = String::new();
    let fixed_len = activity_line_text(&untitled).chars().count();

    let max_title_chars = available_width.saturating_sub(fixed_len).saturating_sub(1);
    let truncated_title: String = item.title.chars().take(max_title_chars).collect();

    let mut truncated = item.clone();
    truncated.title = format!("{}…", truncated_title);
    activity_line_text(&truncated)
}

/// Format timestamp for display
fn format_timestamp(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    // Format as date and time
    timestamp.format("%Y-%m-%d %H:%M").to_string()
}

/// Render channel chat for the current community.
pub fn render_chat_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    let has_error = app.chat_state.error.is_some();
    let layout = banner_layout(area, false, has_error);

    if has_error {
        if let Some(error) = &app.chat_state.error {
            if let Some(error_area) = layout.error {
                render_error_banner(frame, error_area, error, &theme);
            }
        }
    }

    if app.community.is_none() {
        render_panel_lines(
            frame,
            layout.content,
            "Chat",
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No community open",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Open a repo community before using chat.",
                    Style::default().fg(theme.text_dim),
                )),
            ],
            &theme,
        );
        return;
    }

    if app.chat_state.loading && app.chat_state.channels.is_empty() {
        render_panel_lines(
            frame,
            layout.content,
            "Chat",
            create_loading_display("Loading chat...", &theme),
            &theme,
        );
        return;
    }

    if app.chat_state.channels.is_empty() {
        render_panel_lines(
            frame,
            layout.content,
            "Chat",
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No channels available",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "This community does not have a chat channel yet.",
                    Style::default().fg(theme.text_dim),
                )),
            ],
            &theme,
        );
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(6)])
        .split(layout.content);

    render_channel_messages(frame, app, chunks[0]);
    render_channel_input(frame, app, chunks[1]);
}

fn render_channel_messages(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);
    let channel_name = app
        .chat_state
        .selected_channel()
        .map(|channel| channel.name.as_str())
        .unwrap_or("chat");
    let title = format!(" #{} ", channel_name);

    if app.chat_state.messages.is_empty() {
        let empty_lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No messages yet",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Type below to start the channel.",
                Style::default().fg(theme.text_dim),
            )),
        ];
        let empty = Paragraph::new(empty_lines)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(empty, area);
        return;
    }

    let viewport_height = area.height.saturating_sub(2) as usize;
    let lines_per_message = 4;
    let messages_per_screen = (viewport_height / lines_per_message).max(1);
    let total_messages = app.chat_state.messages.len();
    let selected = app
        .chat_state
        .list_state
        .selected()
        .unwrap_or(total_messages.saturating_sub(1));
    let start_index = selected.saturating_sub(messages_per_screen.saturating_sub(1));
    let message_width = (area.width as usize).saturating_sub(8);
    let current_user = app.auth_state.current_user.as_ref();
    let current_user_id = current_user.map(|user| user.id);
    let current_username = current_user.map(|user| user.username.as_str());

    let mut items = Vec::new();
    if app.chat_state.loading_older {
        items.push(ListItem::new(Line::from(Span::styled(
            "Loading older messages...",
            Style::default().fg(theme.text_dim),
        ))));
    } else if app.chat_state.at_history_start && start_index == 0 {
        items.push(ListItem::new(Line::from(Span::styled(
            "Start of channel history",
            Style::default().fg(theme.text_dim),
        ))));
    }

    for (index, message) in app
        .chat_state
        .messages
        .iter()
        .enumerate()
        .skip(start_index)
        .take(messages_per_screen)
    {
        let is_selected = index == selected;
        let is_from_me = Some(message.author_id) == current_user_id;
        let author = chat_author_label(message, current_username, is_from_me);
        let timestamp = message.created_at.format("%H:%M").to_string();
        let header_style = if is_from_me {
            Style::default().fg(theme.primary)
        } else {
            Style::default().fg(theme.success)
        };
        let prefix = if is_selected { ">" } else { " " };

        let mut lines = vec![Line::from(vec![
            Span::styled(prefix, Style::default().fg(theme.text_dim)),
            Span::styled(
                format!(" [{}] ", timestamp),
                Style::default().fg(theme.text_dim),
            ),
            Span::styled(author, header_style.add_modifier(Modifier::BOLD)),
        ])];

        for content_line in message.content.lines() {
            for wrapped in textwrap::wrap(content_line, message_width) {
                lines.push(chat_content_line(
                    &wrapped,
                    current_username,
                    is_selected,
                    &theme,
                ));
            }
        }
        lines.push(Line::from(""));
        items.push(ListItem::new(lines));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .fg(theme.success)
                .bg(theme.highlight_bg)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(list, area, &mut app.chat_state.list_state);
}

fn chat_author_label(
    message: &fido_types::Message,
    current_username: Option<&str>,
    is_from_me: bool,
) -> String {
    if is_from_me {
        return current_username
            .map(|username| format!("@{}", username))
            .unwrap_or_else(|| "you".to_string());
    }

    let id = message.author_id.to_string();
    format!("user:{}", &id[..8])
}

fn chat_content_line(
    content: &str,
    current_username: Option<&str>,
    selected: bool,
    theme: &ThemeColors,
) -> Line<'static> {
    let mention = current_username
        .map(|username| content.contains(&format!("@{}", username)))
        .unwrap_or(false);
    let style = if mention {
        Style::default()
            .fg(theme.warning)
            .add_modifier(Modifier::BOLD)
    } else if selected {
        Style::default().fg(theme.text)
    } else {
        Style::default().fg(theme.text_dim)
    };
    Line::from(Span::styled(format!("  {}", content), style))
}

fn render_channel_input(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);
    let channel_name = app
        .chat_state
        .selected_channel()
        .map(|channel| channel.name.as_str())
        .unwrap_or("chat");
    let title = if app.input_mode == crate::app::InputMode::Typing {
        format!(" Message #{} (Enter to send) ", channel_name)
    } else {
        format!(" Message #{} ", channel_name)
    };

    app.chat_state
        .message_textarea
        .set_style(Style::default().fg(theme.primary));
    app.chat_state
        .message_textarea
        .set_cursor_style(Style::default().fg(theme.background).bg(theme.primary));
    app.chat_state
        .message_textarea
        .set_cursor_line_style(Style::default());
    app.chat_state
        .message_textarea
        .set_block(Block::default().borders(Borders::ALL).title(title));

    frame.render_widget(&app.chat_state.message_textarea, area);
}

/// Render DMs tab
pub fn render_dms_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    if app.dms_state.loading {
        render_panel_lines(
            frame,
            area,
            "Direct Messages",
            create_loading_display("Loading conversations...", &theme),
            &theme,
        );
        return;
    }

    if let Some(error) = &app.dms_state.error {
        let error_lines =
            create_error_display(error, Some("Press Esc to go back to conversations"), &theme);
        render_panel_lines(frame, area, "Direct Messages", error_lines, &theme);
        return;
    }

    // Split into conversations list and messages (no footer - now in page actions bar)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Conversations list
            Constraint::Percentage(70), // Messages
        ])
        .split(area);

    // Render conversations list
    render_conversations_list(frame, app, chunks[0]);

    // Render messages and input
    render_messages_view(frame, app, chunks[1]);

    // Render new conversation modal if open
    if app.dms_state.show_new_conversation_modal {
        render_new_conversation_modal(frame, app, area);
    }
}

/// Render conversations list
pub fn render_conversations_list(frame: &mut Frame, app: &App, area: Rect) {
    let theme = get_theme_colors(app);
    let mut lines = vec![];

    // Add top padding (2 lines for better spacing)
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    // Add "New Conversation" button at the top
    let new_convo_selected = app.dms_state.selection.is_new_conversation();
    let new_convo_style = if new_convo_selected {
        Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.primary)
    };
    let new_convo_prefix = if new_convo_selected { "▶ " } else { "  " };

    lines.push(Line::from(vec![
        Span::styled(new_convo_prefix, new_convo_style),
        Span::styled("+ New Conversation", new_convo_style),
    ]));
    lines.push(Line::from(Span::styled(
        "  Press Enter or N to start",
        Style::default().fg(theme.text_dim),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "─".repeat(area.width.saturating_sub(4) as usize),
        Style::default().fg(theme.text_dim),
    )));
    lines.push(Line::from(""));

    // Show pending conversation at the top of the list if it exists
    if let Some(pending_username) = &app.dms_state.pending_conversation_username {
        let is_selected = app.dms_state.selection.is_pending_draft();

        let style = if is_selected {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        let prefix = if is_selected { "▶ " } else { "  " };

        // Username with draft indicator
        let mut username_spans = vec![Span::styled(prefix, style)];
        username_spans.push(Span::styled(pending_username, style));
        username_spans.push(Span::raw(" "));
        username_spans.push(Span::styled(
            "(Draft)",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::ITALIC),
        ));

        lines.push(Line::from(username_spans));

        // Draft message preview
        lines.push(Line::from(Span::styled(
            "  Type your first message...",
            Style::default()
                .fg(theme.text_dim)
                .add_modifier(Modifier::ITALIC),
        )));

        lines.push(Line::from(""));
    }

    // Incoming pending DM requests — must accept/decline before messaging
    for (i, req) in app.dms_state.pending_requests.iter().enumerate() {
        let is_selected = matches!(app.dms_state.selection, DMSelection::Request(idx) if idx == i);

        let style = if is_selected {
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.warning)
        };

        let prefix = if is_selected { "▶ " } else { "  " };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(format!("✉ @{} wants to chat", req.from_username), style),
        ]));
        lines.push(Line::from(""));
    }

    if !app.dms_state.pending_requests.is_empty() {
        lines.push(Line::from(Span::styled(
            "─".repeat(area.width.saturating_sub(4) as usize),
            Style::default().fg(theme.text_dim),
        )));
        lines.push(Line::from(""));
    }

    for (i, convo) in app.dms_state.conversations.iter().enumerate() {
        let is_selected = app.dms_state.selection.conversation_index() == Some(i);

        let style = if is_selected {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        let prefix = if is_selected { "▶ " } else { "  " };

        // Username with unread indicator
        let mut username_spans = vec![Span::styled(prefix, style)];
        username_spans.push(Span::styled(&convo.other_username, style));

        if convo.unread_count > 0 {
            username_spans.push(Span::raw(" "));
            username_spans.push(Span::styled(
                format!("({})", convo.unread_count),
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        if convo.state == fido_types::DmConversationState::Pending && convo.initiated_by_me {
            username_spans.push(Span::raw(" "));
            username_spans.push(Span::styled(
                "(pending)",
                Style::default().fg(theme.text_dim),
            ));
        }

        lines.push(Line::from(username_spans));

        // Last message preview
        let preview = if convo.last_message.chars().count() > 30 {
            let truncated: String = convo.last_message.chars().take(30).collect();
            format!("  {}", truncated)
        } else {
            format!("  {}", convo.last_message)
        };

        lines.push(Line::from(Span::styled(
            preview,
            Style::default().fg(theme.text_dim),
        )));

        lines.push(Line::from(""));
    }

    let conversations = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Conversations"),
    );
    frame.render_widget(conversations, area);
}

/// The currently-open conversation, if the selection points at one that is loaded.
fn open_outgoing_pending_conversation(app: &App) -> Option<&Conversation> {
    let selected_index = app.dms_state.selection.conversation_index()?;
    let convo = app.dms_state.conversations.get(selected_index)?;
    if app.dms_state.current_conversation_user != Some(convo.other_user_id) {
        return None;
    }
    if convo.state == fido_types::DmConversationState::Pending && convo.initiated_by_me {
        Some(convo)
    } else {
        None
    }
}

/// Render messages view
pub fn render_messages_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let pending_hint = open_outgoing_pending_conversation(app).map(|c| {
        format!(
            "Request sent — waiting for @{} to accept. You can't send more messages until they do.",
            c.other_username
        )
    });

    let constraints = if pending_hint.is_some() {
        vec![
            Constraint::Min(0),    // Messages
            Constraint::Length(1), // Pending hint
            Constraint::Length(6), // Input
        ]
    } else {
        vec![
            Constraint::Min(0),    // Messages
            Constraint::Length(6), // Input (increased from 4 to 6)
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    // Render messages
    render_messages(frame, app, chunks[0]);

    if let Some(hint) = pending_hint {
        let theme = get_theme_colors(app);
        let hint_widget = Paragraph::new(Line::from(Span::styled(
            hint,
            Style::default().fg(theme.text_dim),
        )));
        frame.render_widget(hint_widget, chunks[1]);
        render_message_input(frame, app, chunks[2]);
    } else {
        render_message_input(frame, app, chunks[1]);
    }
}

/// Render messages
pub fn render_messages(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    // Check if there's a pending new conversation
    if let Some(username) = &app.dms_state.pending_conversation_username {
        let empty_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("New conversation with @{}", username),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Type your first message below",
                Style::default().fg(theme.text),
            )),
            Line::from(""),
        ];
        let empty = Paragraph::new(empty_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Messages"));
        frame.render_widget(empty, area);
        return;
    }

    if let DMSelection::Request(idx) = app.dms_state.selection {
        let username = app
            .dms_state
            .pending_requests
            .get(idx)
            .map(|r| r.from_username.as_str())
            .unwrap_or("this user");
        let empty_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("@{} wants to chat", username),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Accept to start a conversation, or decline to dismiss the request.",
                Style::default().fg(theme.text),
            )),
            Line::from(""),
        ];
        let empty = Paragraph::new(empty_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Messages"));
        frame.render_widget(empty, area);
        return;
    }

    // Check if a conversation is selected (show placeholder if on NewConversation button)
    if app.dms_state.selection.is_new_conversation() {
        let empty_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No conversation selected",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Use ↑/↓ to select a conversation",
                Style::default().fg(theme.text),
            )),
            Line::from(Span::styled(
                "or navigate to 'New Conversation' button",
                Style::default().fg(theme.text),
            )),
            Line::from(""),
        ];
        let empty = Paragraph::new(empty_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Messages"));
        frame.render_widget(empty, area);
        return;
    }

    if let Some(selected_index) = app.dms_state.selection.conversation_index() {
        if let Some(conversation) = app.dms_state.conversations.get(selected_index) {
            if app.dms_state.current_conversation_user != Some(conversation.other_user_id) {
                let empty_text = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("@{}", conversation.other_username),
                        Style::default()
                            .fg(theme.success)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press Enter to open this conversation",
                        Style::default().fg(theme.text),
                    )),
                    Line::from(""),
                ];
                let empty = Paragraph::new(empty_text)
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL).title("Messages"));
                frame.render_widget(empty, area);
                return;
            }
        }
    }

    if app.dms_state.messages.is_empty() {
        render_empty_state_block(
            frame,
            area,
            "No messages yet. Start the conversation!",
            &theme,
            "Messages",
        );
        return;
    }

    // Optimized rendering: show most recent messages (auto-scroll to bottom)
    // For large conversation histories, only render visible messages
    let viewport_height = (area.height as usize).saturating_sub(2);
    let lines_per_message = 3; // header + content + blank
    let messages_per_screen = viewport_height / lines_per_message;

    // Always show most recent messages (scroll to bottom by default)
    let total_messages = app.dms_state.messages.len();
    let start_index = total_messages.saturating_sub(messages_per_screen);

    let current_user_id = app.auth_state.current_user.as_ref().map(|u| u.id);

    let mut lines = vec![];

    // Calculate available width for message content (account for borders and indent)
    let message_width = (area.width as usize).saturating_sub(6);

    // Render only visible messages (performance optimization for long conversations)
    for msg in app.dms_state.messages.iter().skip(start_index) {
        let is_from_me = Some(msg.from_user_id) == current_user_id;

        let timestamp = msg.created_at.format("%H:%M").to_string();
        // Use actual username from message
        let sender = &msg.from_username;

        let header_style = if is_from_me {
            Style::default().fg(theme.primary)
        } else {
            Style::default().fg(theme.success)
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("[{}] ", timestamp),
                Style::default().fg(theme.text_dim),
            ),
            Span::styled(sender, header_style.add_modifier(Modifier::BOLD)),
        ]));

        // Message content with wrapping
        for content_line in msg.content.lines() {
            let wrapped = textwrap::wrap(content_line, message_width);
            for wrapped_line in wrapped {
                let prefix = "  ";
                lines.push(Line::from(Span::styled(
                    format!("{}{}", prefix, wrapped_line),
                    Style::default().fg(theme.text),
                )));
            }
        }

        lines.push(Line::from(""));
    }

    let messages =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Messages"));
    frame.render_widget(messages, area);
}

/// Render message input
pub fn render_message_input(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    if let DMSelection::Request(idx) = app.dms_state.selection {
        let username = app
            .dms_state
            .pending_requests
            .get(idx)
            .map(|r| r.from_username.as_str())
            .unwrap_or("this user");
        let input = Paragraph::new(format!(
            "@{} wants to chat — a: Accept | x: Decline",
            username
        ))
        .style(Style::default().fg(theme.warning))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Message Input"),
        );
        frame.render_widget(input, area);
        return;
    }

    // Check if conversation is selected and user can type
    let can_type = app.dms_state.pending_conversation_username.is_some()
        || app.dms_state.selection.conversation_index().is_some();

    if !can_type {
        // Show placeholder when no conversation is selected
        let placeholder = if app.dms_state.selection.is_new_conversation() {
            "Press Enter or N to start a new conversation"
        } else {
            "Select a conversation to send messages"
        };

        let input = Paragraph::new(placeholder)
            .style(Style::default().fg(theme.text_dim))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Message Input"),
            );
        frame.render_widget(input, area);
        return;
    }

    // Set block on textarea before rendering
    let title = if app.dms_state.pending_conversation_username.is_some() {
        "Type your first message (Enter to send)"
    } else {
        "Message Input (Enter to send)"
    };

    // Apply theme styling to textarea - use primary color for text to ensure visibility
    app.dms_state.message_textarea.set_style(
        Style::default().fg(theme.primary), // Use primary color for better visibility
    );
    app.dms_state.message_textarea.set_cursor_style(
        Style::default().fg(theme.background).bg(theme.primary), // Visible cursor
    );
    app.dms_state.message_textarea.set_cursor_line_style(
        Style::default(), // No special cursor line styling
    );

    // Use default border style (no theme.border) to match other boxes
    app.dms_state
        .message_textarea
        .set_block(Block::default().borders(Borders::ALL).title(title));

    // Render TextArea widget
    frame.render_widget(&app.dms_state.message_textarea, area);
}

/// Render Profile tab
pub fn render_profile_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    if app.profile_state.loading {
        render_panel_lines(
            frame,
            area,
            "Profile",
            create_loading_display("Loading profile...", &theme),
            &theme,
        );
        return;
    }

    if let Some(error) = &app.profile_state.error {
        let error_lines = create_error_display(error, None, &theme);
        render_panel_lines(frame, area, "Profile", error_lines, &theme);
        return;
    }

    if let Some(profile) = &app.profile_state.profile {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Profile stats
                Constraint::Min(0),     // User posts (no footer - now in page actions bar)
            ])
            .split(area);

        // Profile stats
        render_profile_stats(frame, app, profile, chunks[0]);

        // User posts
        render_user_posts(frame, app, chunks[1]);
    } else {
        render_empty_state_block(frame, area, "No profile data", &theme, "Profile");
    }
}

/// Render profile stats
pub fn render_profile_stats(
    frame: &mut Frame,
    app: &App,
    profile: &fido_types::UserProfile,
    area: Rect,
) {
    let theme = get_theme_colors(app);
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Username: ", Style::default().fg(theme.primary)),
            Span::styled(
                &profile.username,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Bio: ", Style::default().fg(theme.primary)),
            Span::styled(
                profile.bio.as_deref().unwrap_or("No bio set"),
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Karma: ", Style::default().fg(theme.success)),
            Span::styled(
                profile.karma.to_string(),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("Posts: ", Style::default().fg(theme.secondary)),
            Span::styled(
                profile.post_count.to_string(),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Joined: ", Style::default().fg(theme.text_dim)),
            Span::styled(
                profile.join_date.format("%Y-%m-%d").to_string(),
                Style::default().fg(theme.text),
            ),
        ]),
    ];

    if !profile.recent_hashtags.is_empty() {
        lines.push(Line::from(""));
        let mut hashtag_spans = vec![Span::styled(
            "Recent hashtags: ",
            Style::default().fg(theme.accent),
        )];
        for (i, tag) in profile.recent_hashtags.iter().take(5).enumerate() {
            if i > 0 {
                hashtag_spans.push(Span::raw(", "));
            }
            hashtag_spans.push(Span::styled(
                format!("#{}", tag),
                Style::default().fg(theme.primary),
            ));
        }
        lines.push(Line::from(hashtag_spans));
    }

    let stats = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Profile Stats"),
    );
    frame.render_widget(stats, area);
}

/// Render user posts
pub fn render_user_posts(frame: &mut Frame, app: &mut App, area: Rect) {
    // Get theme colors
    let theme = get_theme_colors(app);

    if app.profile_state.user_posts.is_empty() {
        render_empty_state_block(frame, area, "No posts yet", &theme, "Your Posts");
        return;
    }

    // Calculate available width for post content
    let post_width = (area.width as usize).saturating_sub(4);

    let items: Vec<ListItem> = app
        .profile_state
        .user_posts
        .iter()
        .enumerate()
        .flat_map(|(i, post)| {
            let is_selected = app.profile_state.list_state.selected() == Some(i);

            let mut post_lines: Vec<Line> = Vec::new();

            let prefix = if is_selected { "▶ " } else { "  " };
            let timestamp = post.created_at.format("%Y-%m-%d %H:%M").to_string();

            let header_style = if is_selected {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.primary)
            };

            post_lines.push(Line::from(vec![
                Span::styled(prefix, header_style),
                Span::styled(timestamp, Style::default().fg(theme.text_dim)),
            ]));

            // Post content with wrapping
            let content_lines =
                format_post_content_with_width(&post.content, is_selected, &theme, post_width);
            post_lines.extend(content_lines);

            // Vote counts with highlighting for user's vote
            let user_voted_up = post.user_vote.as_deref() == Some("up");
            let user_voted_down = post.user_vote.as_deref() == Some("down");

            let upvote_style = if user_voted_up {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_dim)
            };

            let downvote_style = if user_voted_down {
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_dim)
            };

            post_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("↑ {} ", post.upvotes), upvote_style),
                Span::styled(format!("↓ {} ", post.downvotes), downvote_style),
                Span::styled(
                    format!("💬 {}", post.reply_count),
                    Style::default().fg(theme.text_dim),
                ),
            ]));

            if i < app.profile_state.user_posts.len() - 1 {
                post_lines.push(Line::from(""));
            }

            vec![ListItem::new(post_lines)]
        })
        .collect();

    let posts_widget = styled_list(items, &theme, None)
        .block(Block::default().borders(Borders::ALL).title("Your Posts"));

    frame.render_stateful_widget(posts_widget, area, &mut app.profile_state.list_state);
}

/// Render Settings tab
pub fn render_settings_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    if app.settings_state.loading {
        render_panel_lines(
            frame,
            area,
            "Settings",
            create_loading_display("Loading settings...", &theme),
            &theme,
        );
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Error/Success message
            Constraint::Min(0), // Settings form (no instructions footer - now in page actions bar)
        ])
        .split(area);

    // Error/Success message
    if let Some(error) = &app.settings_state.error {
        let is_success = error.contains("successfully");
        let style = if is_success {
            Style::default().fg(theme.success)
        } else {
            Style::default().fg(theme.error)
        };

        let message = Paragraph::new(error.clone())
            .style(style)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(message, chunks[0]);
    }

    // Settings form
    if let Some(config) = &app.settings_state.config {
        let mut lines = vec![];

        lines.push(Line::from(""));

        // Color Scheme
        let color_selected =
            app.settings_state.selected_field == crate::app::SettingsField::ColorScheme;
        let color_style = if color_selected {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        lines.push(Line::from(vec![
            Span::styled(if color_selected { "▶ " } else { "  " }, color_style),
            Span::styled("Color Scheme: ", Style::default().fg(theme.primary)),
            Span::styled(config.color_scheme.as_str(), color_style),
            Span::raw("  "),
            Span::styled("(←/→ to change)", Style::default().fg(theme.text_dim)),
        ]));

        lines.push(Line::from(""));

        // Sort Order
        let sort_selected =
            app.settings_state.selected_field == crate::app::SettingsField::SortOrder;
        let sort_style = if sort_selected {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        lines.push(Line::from(vec![
            Span::styled(if sort_selected { "▶ " } else { "  " }, sort_style),
            Span::styled("Sort Order: ", Style::default().fg(theme.primary)),
            Span::styled(config.sort_order.as_str(), sort_style),
            Span::raw("  "),
            Span::styled("(←/→ to change)", Style::default().fg(theme.text_dim)),
        ]));

        lines.push(Line::from(""));

        // Max Posts Display
        let max_posts_selected =
            app.settings_state.selected_field == crate::app::SettingsField::MaxPosts;
        let max_posts_style = if max_posts_selected {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        let max_posts_value =
            if max_posts_selected && !app.settings_state.max_posts_input.is_empty() {
                &app.settings_state.max_posts_input
            } else {
                &config.max_posts_display.to_string()
            };

        lines.push(Line::from(vec![
            Span::styled(
                if max_posts_selected { "▶ " } else { "  " },
                max_posts_style,
            ),
            Span::styled("Max Posts Display: ", Style::default().fg(theme.primary)),
            Span::styled(max_posts_value, max_posts_style),
            Span::raw("  "),
            Span::styled("(←/→ or type number)", Style::default().fg(theme.text_dim)),
        ]));

        lines.push(Line::from(""));

        // Show unsaved changes indicator
        if app.settings_state.has_unsaved_changes {
            lines.push(Line::from(vec![
                Span::styled(
                    "⚠ ",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "You have unsaved changes. Press 's' to save.",
                    Style::default().fg(theme.warning),
                ),
            ]));
        }

        let settings_widget =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Settings"));
        frame.render_widget(settings_widget, chunks[1]);
    } else {
        render_empty_state_block(frame, chunks[1], "No settings loaded", &theme, "Settings");
    }
}
