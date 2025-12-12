use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::super::theme::get_theme_colors;
use super::utils::centered_rect;
use crate::app::App;

/// Render help modal
pub fn render_help_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    // Create centered modal area (80% width, 85% height)
    let modal_area = centered_rect(80, 85, area);

    // Clear background
    frame.render_widget(Clear, modal_area);

    // Get context-specific shortcuts
    let shortcuts = get_shortcuts_for_context(app);

    // Create help content
    let mut lines = vec![Line::from("")];

    for (category, items) in shortcuts {
        lines.push(Line::from(Span::styled(
            category,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for (key, description) in items {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<15}", key), Style::default().fg(theme.success)),
                Span::styled(description, Style::default().fg(theme.text)),
            ]));
        }

        lines.push(Line::from(""));
    }

    let help_content = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                )
                .title(" Keyboard Shortcuts ")
                .title_alignment(Alignment::Center)
                .style(Style::default().bg(theme.background)),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });

    frame.render_widget(help_content, modal_area);
}

/// Render save confirmation modal
pub fn render_save_confirmation_modal(frame: &mut Frame, app: &App, area: Rect) {
    let theme = get_theme_colors(app);

    // Create centered modal area (50% width, 25% height)
    let modal_area = centered_rect(50, 25, area);

    // Clear background
    frame.render_widget(Clear, modal_area);

    // Create content with message and instructions together
    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "You have unsaved changes in Settings.",
            Style::default().fg(theme.text),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Do you want to save before leaving?",
            Style::default().fg(theme.text),
        )),
        Line::from(""),
        Line::from("─".repeat(46)).style(Style::default().fg(theme.border)),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Y",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(": Save  ", Style::default().fg(theme.text)),
            Span::styled(
                "N",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(": Discard  ", Style::default().fg(theme.text)),
            Span::styled(
                "Esc",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(": Cancel", Style::default().fg(theme.text)),
        ]),
    ];

    let modal = Paragraph::new(content).alignment(Alignment::Center).block(
        Block::default()
            .title(" Unsaved Changes ")
            .borders(Borders::ALL)
            .border_style(
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(theme.background)),
    );

    frame.render_widget(modal, modal_area);
}

/// Get shortcuts relevant to current context
pub fn get_shortcuts_for_context(
    app: &mut App,
) -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
    let mut shortcuts = vec![];

    // Global shortcuts (always shown)
    shortcuts.push((
        "Global",
        vec![("q / Esc", "Quit application"), ("?", "Toggle this help")],
    ));

    // Add logout for main screen
    if matches!(app.current_screen, crate::app::Screen::Main) {
        shortcuts.push(("Account", vec![("Shift+L", "Logout")]));
    }

    match app.current_screen {
        crate::app::Screen::Auth => {
            add_auth_shortcuts(&mut shortcuts);
        }
        crate::app::Screen::Main => {
            add_main_screen_shortcuts(app, &mut shortcuts);
        }
    }

    shortcuts
}

/// Add authentication screen shortcuts
pub fn add_auth_shortcuts(shortcuts: &mut Vec<(&'static str, Vec<(&'static str, &'static str)>)>) {
    shortcuts.push((
        "Authentication",
        vec![
            ("l", "Load test users"),
            ("↑/k", "Move up"),
            ("↓/j", "Move down"),
            ("Enter", "Login with selected user"),
        ],
    ));
}

/// Add main screen shortcuts (tab navigation + context-specific)
pub fn add_main_screen_shortcuts(
    app: &mut App,
    shortcuts: &mut Vec<(&'static str, Vec<(&'static str, &'static str)>)>,
) {
    // Tab navigation
    shortcuts.push((
        "Tab Navigation",
        vec![("Tab", "Next tab"), ("Shift+Tab", "Previous tab")],
    ));

    // Context-specific shortcuts based on current tab
    match app.current_tab {
        crate::app::Tab::Posts => add_posts_tab_shortcuts(app, shortcuts),
        crate::app::Tab::DMs => add_dms_tab_shortcuts(shortcuts),
        crate::app::Tab::Profile => add_profile_tab_shortcuts(app, shortcuts),
        crate::app::Tab::Settings => add_settings_tab_shortcuts(shortcuts),
    }

    // User Profile View modal (shown when viewing another user's profile)
    if app.user_profile_view.is_some() {
        shortcuts.push((
            "User Profile View",
            vec![
                ("f", "Follow/Unfollow user"),
                ("m", "Open DM conversation (mutual friends only)"),
                ("Esc / q", "Close profile"),
            ],
        ));
    }
}

/// Add Posts tab shortcuts
pub fn add_posts_tab_shortcuts(
    app: &mut App,
    shortcuts: &mut Vec<(&'static str, Vec<(&'static str, &'static str)>)>,
) {
    if app.viewing_post_detail {
        add_post_detail_shortcuts(app, shortcuts);
    } else {
        add_posts_feed_shortcuts(app, shortcuts);
    }
}

/// Add post detail view shortcuts
pub fn add_post_detail_shortcuts(
    app: &mut App,
    shortcuts: &mut Vec<(&'static str, Vec<(&'static str, &'static str)>)>,
) {
    if let Some(detail_state) = &app.post_detail_state {
        shortcuts.push((
            "Post Detail View",
            vec![
                ("Esc", "Close post detail"),
                ("↑/k", "Previous reply"),
                ("↓/j", "Next reply"),
                ("r", "Reply to post"),
                ("u", "Upvote post/reply"),
                ("d", "Downvote post/reply"),
                ("p", "View author profile"),
            ],
        ));

        // Owner-only shortcuts (if viewing own post)
        if let Some(post) = &detail_state.post {
            if let Some(user) = &app.auth_state.current_user {
                if post.author_id == user.id {
                    shortcuts.push(("Post Owner Actions", vec![("x", "Delete post")]));
                }
            }
        }

        // Modal-specific shortcuts
        if detail_state.show_reply_composer {
            shortcuts.push((
                "Reply Composer",
                vec![
                    ("Enter", "Submit reply"),
                    ("Esc", "Cancel reply"),
                    (":emoji:", "Use emoji shortcodes"),
                ],
            ));
        } else if detail_state.show_delete_confirmation {
            shortcuts.push((
                "Delete Confirmation",
                vec![
                    ("y / Y", "Confirm deletion"),
                    ("n / N / Esc", "Cancel deletion"),
                ],
            ));
        }
    }
}

/// Add posts feed shortcuts
pub fn add_posts_feed_shortcuts(
    app: &mut App,
    shortcuts: &mut Vec<(&'static str, Vec<(&'static str, &'static str)>)>,
) {
    shortcuts.push((
        "Posts Tab",
        vec![
            ("↓/j", "Next post"),
            ("↑/k", "Previous post"),
            ("Space/Enter", "Open post detail"),
            ("u", "Upvote selected post"),
            ("d", "Downvote selected post"),
            ("n", "New post"),
            ("f", "Filter posts"),
            ("s", "Search users"),
            ("p", "View author profile"),
        ],
    ));

    if app.posts_state.show_new_post_modal {
        shortcuts.push((
            "New Post Modal",
            vec![
                ("Enter", "Submit post"),
                ("Esc", "Cancel"),
                (":emoji:", "Use emoji shortcodes"),
            ],
        ));
    }

    if app.user_search_state.show_modal {
        shortcuts.push((
            "User Search Modal",
            vec![
                ("Type", "Search for users"),
                ("↓/j", "Next result"),
                ("↑/k", "Previous result"),
                ("Enter", "View profile"),
                ("d", "Send DM"),
                ("Esc", "Close search"),
            ],
        ));
    }
}

/// Add DMs tab shortcuts
pub fn add_dms_tab_shortcuts(
    shortcuts: &mut Vec<(&'static str, Vec<(&'static str, &'static str)>)>,
) {
    shortcuts.push((
        "DMs Tab",
        vec![
            ("↓/j", "Next conversation"),
            ("↑/k", "Previous conversation / New button"),
            ("Enter", "Open conversation / Start new"),
            ("Type", "Compose message"),
            ("Enter", "Send message"),
            ("Esc", "Clear message / Stop typing"),
            (":emoji:", "Use emoji shortcodes"),
        ],
    ));
}

/// Add Profile tab shortcuts
pub fn add_profile_tab_shortcuts(
    app: &mut App,
    shortcuts: &mut Vec<(&'static str, Vec<(&'static str, &'static str)>)>,
) {
    shortcuts.push((
        "Profile Tab",
        vec![
            ("↓/j", "Next post"),
            ("↑/k", "Previous post"),
            ("e", "Edit bio"),
        ],
    ));

    if app.profile_state.show_edit_bio_modal {
        shortcuts.push((
            "Edit Bio Modal",
            vec![
                ("Enter", "Save bio"),
                ("Esc", "Cancel"),
                ("←/→", "Move cursor"),
                ("Home/End", "Start/End of text"),
            ],
        ));
    }
}

/// Add Settings tab shortcuts
pub fn add_settings_tab_shortcuts(
    shortcuts: &mut Vec<(&'static str, Vec<(&'static str, &'static str)>)>,
) {
    shortcuts.push((
        "Settings Tab",
        vec![
            ("↓/j", "Next setting"),
            ("↑/k", "Previous setting"),
            ("←/h / →/l / Enter", "Change value"),
            ("s", "Save settings"),
        ],
    ));
}
