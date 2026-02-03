use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::super::components::footer::render_footer;
use super::super::components::modal::{render_modal_container, ModalConfig};
use super::super::components::tab_bar::{render_tab_bar, TabBarConfig};
use super::super::theme::get_theme_colors;
use crate::app::App;

/// Render filter modal
pub fn render_filter_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    // Render semi-transparent background overlay
    let background = Block::default().style(Style::default().bg(theme.background));
    frame.render_widget(background, area);

    let config = ModalConfig::new(" Filter Posts ").with_size(70, 80);
    let inner = render_modal_container(frame, area, &config, &theme);

    // Modal layout
    let modal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab selector
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Instructions (needs 3 for border + text)
        ])
        .split(inner);

    // Tab selector - match shared tab style
    let selected_tab_idx = match app.posts_state.filter_modal_state.selected_tab {
        crate::app::FilterTab::All => 0,
        crate::app::FilterTab::Hashtags => 1,
        crate::app::FilterTab::Users => 2,
    };

    let tab_config = TabBarConfig {
        tabs: &["All", "Hashtags", "Users"],
        selected_index: selected_tab_idx,
    };
    render_tab_bar(frame, modal_chunks[0], &tab_config, &theme);

    // Content based on selected tab
    let content_lines: Vec<Line> = if app.posts_state.filter_modal_state.search_mode {
        // Show search results
        if app.posts_state.filter_modal_state.search_results.is_empty() {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No results found",
                    Style::default().fg(theme.warning),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Try a different search term",
                    Style::default().fg(theme.text_dim),
                )),
            ]
        } else {
            let mut lines = vec![Line::from("")];
            for (i, hashtag) in app
                .posts_state
                .filter_modal_state
                .search_results
                .iter()
                .enumerate()
            {
                let is_selected = i == app.posts_state.filter_modal_state.selected_index;
                let prefix = if is_selected { "▶ " } else { "  " };
                let style = if is_selected {
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                lines.push(Line::from(Span::styled(
                    format!("{}#{}", prefix, hashtag),
                    style,
                )));
            }
            lines
        }
    } else {
        match app.posts_state.filter_modal_state.selected_tab {
            crate::app::FilterTab::All => {
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Show all posts",
                        Style::default().fg(theme.text),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press Enter to view all posts",
                        Style::default().fg(theme.text_dim),
                    )),
                ]
            }
            crate::app::FilterTab::Hashtags => {
                // Check if in add hashtag input mode
                if app.posts_state.filter_modal_state.show_add_hashtag_input {
                    vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "Enter hashtag name to follow:",
                            Style::default().add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("#{}", app.posts_state.filter_modal_state.add_hashtag_input),
                            Style::default().fg(theme.primary),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            "Enter: Follow | Esc: Cancel",
                            Style::default().fg(theme.text_dim),
                        )),
                    ]
                } else if app.posts_state.filter_modal_state.hashtag_list.is_empty() {
                    vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "No followed hashtags yet",
                            Style::default().fg(theme.warning),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled(
                                "▶ ",
                                Style::default()
                                    .fg(theme.success)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled("+ ", Style::default().fg(theme.success)),
                            Span::styled(
                                "Add Hashtag",
                                Style::default()
                                    .fg(theme.success)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]),
                    ]
                } else {
                    let mut lines = vec![Line::from("")];
                    for (i, hashtag) in app
                        .posts_state
                        .filter_modal_state
                        .hashtag_list
                        .iter()
                        .enumerate()
                    {
                        let is_selected = i == app.posts_state.filter_modal_state.selected_index;
                        let is_checked = app
                            .posts_state
                            .filter_modal_state
                            .checked_hashtags
                            .contains(hashtag);

                        let checkbox = if is_checked { "[x]" } else { "[ ]" };
                        let prefix = if is_selected { "▶ " } else { "  " };

                        let style = if is_selected {
                            Style::default()
                                .fg(theme.success)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(theme.text)
                        };

                        lines.push(Line::from(Span::styled(
                            format!("{}{} #{}", prefix, checkbox, hashtag),
                            style,
                        )));
                    }

                    // Add "Add Hashtag" option at bottom
                    let is_selected = app.posts_state.filter_modal_state.selected_index
                        == app.posts_state.filter_modal_state.hashtag_list.len();
                    let prefix = if is_selected { "▶ " } else { "  " };
                    let style = if is_selected {
                        Style::default()
                            .fg(theme.success)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };

                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled(prefix, style),
                        Span::styled("+ ", Style::default().fg(theme.success)),
                        Span::styled(
                            "Add Hashtag",
                            Style::default()
                                .fg(theme.success)
                                .add_modifier(if is_selected {
                                    Modifier::BOLD
                                } else {
                                    Modifier::empty()
                                }),
                        ),
                    ]));

                    lines
                }
            }
            crate::app::FilterTab::Users => {
                if app.posts_state.filter_modal_state.user_list.is_empty() {
                    vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "No friends yet",
                            Style::default().fg(theme.warning),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            "To add a friend:",
                            Style::default().fg(theme.text_dim),
                        )),
                        Line::from(Span::styled(
                            "1. Note a username from posts (e.g. @alice)",
                            Style::default().fg(theme.text_dim),
                        )),
                        Line::from(Span::styled(
                            "2. Use API: POST /friends/add/:username",
                            Style::default().fg(theme.text_dim),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            "(UI for adding friends coming soon!)",
                            Style::default()
                                .fg(theme.text_dim)
                                .add_modifier(Modifier::ITALIC),
                        )),
                    ]
                } else {
                    let mut lines = vec![Line::from("")];
                    for (i, username) in app
                        .posts_state
                        .filter_modal_state
                        .user_list
                        .iter()
                        .enumerate()
                    {
                        let is_selected = i == app.posts_state.filter_modal_state.selected_index;
                        let is_checked = app
                            .posts_state
                            .filter_modal_state
                            .checked_users
                            .contains(username);

                        let checkbox = if is_checked { "[x]" } else { "[ ]" };
                        let prefix = if is_selected { "▶ " } else { "  " };

                        let style = if is_selected {
                            Style::default()
                                .fg(theme.success)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(theme.text)
                        };

                        lines.push(Line::from(Span::styled(
                            format!("{}{} @{}", prefix, checkbox, username),
                            style,
                        )));
                    }
                    lines
                }
            }
        }
    };

    let content = Paragraph::new(content_lines)
        .style(Style::default().bg(theme.background))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.background)),
        );
    frame.render_widget(content, modal_chunks[1]);

    // Instructions - context-sensitive shortcuts
    let instructions_text = if app.posts_state.filter_modal_state.show_add_hashtag_input {
        "Enter: Follow | Esc: Cancel"
    } else {
        match app.posts_state.filter_modal_state.selected_tab {
            crate::app::FilterTab::All => "Enter: Show All Posts | Esc: Cancel",
            crate::app::FilterTab::Hashtags => {
                "↑/↓/j/k: Navigate | Space: Toggle | Enter: Apply | X: Unfollow | Tab: Switch | Esc: Cancel"
            }
            crate::app::FilterTab::Users => {
                "↑/↓/j/k: Navigate | Space: Toggle | Enter: Apply | Tab: Switch | Esc: Cancel"
            }
        }
    };

    render_footer(frame, modal_chunks[2], instructions_text, &theme);
}
