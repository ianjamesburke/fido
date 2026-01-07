use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::theme::ThemeColors;

// Layout constants
pub const BORDER_PADDING: u16 = 4; // Total horizontal padding from borders (2 per side)

/// Format timestamp for display
pub fn format_timestamp(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    timestamp.format("%Y-%m-%d %H:%M").to_string()
}

/// Format post content with hashtag highlighting and text wrapping
#[allow(dead_code)]
pub fn format_post_content(
    content: &str,
    is_selected: bool,
    theme: &ThemeColors,
) -> Vec<Line<'static>> {
    format_post_content_with_width(content, is_selected, theme, 80)
}

/// Format post content with hashtag highlighting and text wrapping with specified width
pub fn format_post_content_with_width(
    content: &str,
    is_selected: bool,
    theme: &ThemeColors,
    max_width: usize,
) -> Vec<Line<'static>> {
    let mut lines = vec![];
    let wrap_width = max_width.saturating_sub(4);

    for line in content.lines() {
        let wrapped = textwrap::wrap(line, wrap_width);

        for wrapped_line in wrapped {
            let mut spans = vec![Span::raw("  ")]; // Indent

            let line_str = wrapped_line.to_string();
            let mut current_word = String::new();
            let mut whitespace_buffer = String::new();

            for ch in line_str.chars() {
                if ch.is_whitespace() {
                    if !current_word.is_empty() {
                        push_styled_word(&mut spans, &current_word, is_selected, theme);
                        current_word.clear();
                    }
                    whitespace_buffer.push(ch);
                } else {
                    if !whitespace_buffer.is_empty() {
                        spans.push(Span::raw(std::mem::take(&mut whitespace_buffer)));
                    }
                    current_word.push(ch);
                }
            }

            if !current_word.is_empty() {
                push_styled_word(&mut spans, &current_word, is_selected, theme);
            }
            if !whitespace_buffer.is_empty() {
                spans.push(Span::raw(whitespace_buffer));
            }

            lines.push(Line::from(spans));
        }
    }

    lines
}

/// Push a styled word to spans with appropriate formatting
fn push_styled_word(
    spans: &mut Vec<Span<'static>>,
    word: &str,
    is_selected: bool,
    theme: &ThemeColors,
) {
    let (color, should_bold) = if word.starts_with('#') {
        (
            if is_selected {
                theme.accent
            } else {
                theme.secondary
            },
            true,
        )
    } else if word.starts_with('@') {
        (theme.primary, true)
    } else {
        (theme.text, is_selected)
    };

    let mut style = Style::default().fg(color);
    if should_bold {
        style = style.add_modifier(Modifier::BOLD);
    }

    spans.push(Span::styled(word.to_string(), style));
}

/// Format post content for input box (no indent, simpler formatting)
#[allow(dead_code)]
pub fn format_post_content_for_input(content: &str) -> Vec<Line<'static>> {
    content
        .lines()
        .map(|line| Line::from(line.to_string()))
        .collect()
}

/// Format bio content with wrapping
#[allow(dead_code)]
pub fn format_bio_content_with_width(
    content: &str,
    max_width: usize,
    theme: &ThemeColors,
) -> Vec<Line<'static>> {
    let mut lines = vec![];

    for line in content.lines() {
        let wrapped = textwrap::wrap(line, max_width);
        for wrapped_line in wrapped {
            lines.push(Line::from(Span::styled(
                wrapped_line.to_string(),
                Style::default().fg(theme.text),
            )));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(""));
    }

    lines
}
