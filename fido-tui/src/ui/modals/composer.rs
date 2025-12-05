use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use super::super::theme::get_theme_colors;
use super::utils::centered_rect;

/// Render unified composer modal (new post, reply, edit post, edit bio)
pub fn render_unified_composer_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    use crate::app::ComposerMode;

    let theme = get_theme_colors(app);

    // Determine modal configuration based on mode
    let (title, has_context, context_lines, max_chars, instructions) =
        match &app.composer_state.mode {
            Some(ComposerMode::NewPost) => (
                "New Post",
                false,
                vec![],
                280,
                "✨ Type to compose | Enter: Submit | Esc: Cancel ✨",
            ),
            Some(ComposerMode::Reply {
                parent_author,
                parent_content,
                ..
            }) => {
                let context_width = area.width.saturating_sub(24).min(66) as usize;
                let truncated_content = if parent_content.chars().count() > context_width {
                    let truncated: String = parent_content.chars().take(context_width.saturating_sub(3)).collect();
                    format!("{}...", truncated)
                } else {
                    parent_content.clone()
                };

                let lines = vec![
                    Line::from(vec![
                        Span::styled("Replying to ", Style::default().fg(theme.text_dim)),
                        Span::styled(
                            format!("@{}", parent_author),
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(Span::styled(
                        truncated_content,
                        Style::default()
                            .fg(theme.text_dim)
                            .add_modifier(Modifier::ITALIC),
                    )),
                ];

                (
                    "Reply to Post",
                    true,
                    lines,
                    280,
                    "Type to compose | Enter: Submit | Esc: Cancel",
                )
            }
            Some(ComposerMode::EditPost { .. }) => (
                "Edit Post",
                false,
                vec![],
                280,
                "Type to edit | Enter: Submit | Esc: Cancel",
            ),
            Some(ComposerMode::EditBio) => {
                ("Edit Bio", false, vec![], 160, "Type to edit | Enter: Submit | Esc: Cancel")
            }
            None => return, // Should never happen
        };

    // Create centered modal area
    // Reply modal is smaller (70% width, 56% height) to show thread context behind it
    // Other modals use standard size (70% width, 80% height)
    let height_percent = match &app.composer_state.mode {
        Some(ComposerMode::Reply { .. }) => 56, // 30% smaller than 80%
        _ => 80,
    };
    let modal_area = centered_rect(70, height_percent, area);

    // Clear background (always clear to ensure clean rendering)
    frame.render_widget(Clear, modal_area);

    let outer_block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.background));

    let inner = outer_block.inner(modal_area);
    frame.render_widget(outer_block, modal_area);

    // Create modal layout
    let constraints = if has_context {
        vec![
            Constraint::Length(4), // Context
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Character counter
            Constraint::Length(3), // Instructions (needs 3 for border + text)
        ]
    } else {
        vec![
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Character counter
            Constraint::Length(3), // Instructions (needs 3 for border + text)
        ]
    };

    let modal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let mut chunk_idx = 0;

    // Context (for replies)
    if has_context {
        let context = Paragraph::new(context_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Context")
                .border_style(Style::default().fg(theme.text_dim)),
        );
        frame.render_widget(context, modal_chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Content area with TextArea widget
    let content_area = modal_chunks[chunk_idx];

    // Create a block for the content area
    let content_block = Block::default()
        .borders(Borders::ALL)
        .title("Content")
        .border_style(Style::default().fg(theme.primary));
    
    let inner_content_area = content_block.inner(content_area);
    frame.render_widget(content_block, content_area);

    // Render TextArea directly - styling should be set when composer opens, not during render
    frame.render_widget(&app.composer_state.textarea, inner_content_area);
    chunk_idx += 1;

    // Character counter
    let char_count = app.composer_state.char_count();
    let counter_style = if char_count >= max_chars {
        Style::default()
            .fg(theme.error)
            .add_modifier(Modifier::BOLD)
    } else if char_count >= (max_chars * 9 / 10) {
        Style::default().fg(theme.warning)
    } else {
        Style::default().fg(theme.success)
    };

    let counter_text = format!("{}/{} characters", char_count, max_chars);
    let counter = Paragraph::new(counter_text)
        .style(counter_style)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    frame.render_widget(counter, modal_chunks[chunk_idx]);
    chunk_idx += 1;

    // Instructions - context-sensitive shortcuts
    let instructions_widget = Paragraph::new(instructions)
        .style(Style::default().fg(theme.text))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    frame.render_widget(instructions_widget, modal_chunks[chunk_idx]);
}
