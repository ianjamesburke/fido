use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use fido_types::{ActivityKind, ActivityState, Post};

use super::theme::ThemeColors;

// Layout constants
pub const BORDER_PADDING: u16 = 4; // Total horizontal padding from borders (2 per side)

/// Format timestamp for display
pub fn format_timestamp(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    timestamp.format("%Y-%m-%d %H:%M").to_string()
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

/// Text for one row of the repo-activity feed, e.g.
/// `⊙ #7 Fix login · issue opened by alice` or
/// `⇄ #9 Dark mode · merged · bob`.
pub fn activity_line_text(post: &Post) -> String {
    let Some(kind) = post.github_kind else {
        return post.content.clone();
    };
    match kind {
        ActivityKind::Issue => {
            let mut line = format!("⊙ #{} {}", post.github_id.unwrap_or_default(), post.content);
            if post.github_state == Some(ActivityState::Closed) {
                line.push_str(" · closed");
            }
            line
        }
        ActivityKind::PullRequest => {
            let status = match post.github_state {
                Some(ActivityState::Merged) => "merged",
                Some(ActivityState::Closed) => "closed",
                _ => "open",
            };
            format!(
                "⇄ #{} {} · {}",
                post.github_id.unwrap_or_default(),
                post.content,
                status
            )
        }
    }
}

/// Color for the activity row's leading glyph, based on its state.
pub fn activity_glyph_color(post: &Post, theme: &ThemeColors) -> Color {
    match post.github_state {
        Some(ActivityState::Open) => theme.success,
        Some(ActivityState::Closed) => theme.error,
        Some(ActivityState::Merged) => Color::Magenta,
        None => theme.text_dim,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn github_post(
        kind: ActivityKind,
        state: ActivityState,
        number: i64,
        title: &str,
        _author_login: &str,
    ) -> Post {
        Post {
            id: uuid::Uuid::new_v4(),
            author_id: uuid::Uuid::new_v4(),
            author_username: "github".to_string(),
            community_id: uuid::Uuid::new_v4(),
            content: title.to_string(),
            created_at: chrono::Utc::now(),
            upvotes: 0,
            downvotes: 0,
            approved: true,
            hashtags: Vec::new(),
            user_vote: None,
            parent_post_id: None,
            reply_count: 0,
            reply_to_user_id: None,
            reply_to_username: None,
            github_id: Some(number),
            github_kind: Some(kind),
            github_state: Some(state),
            github_html_url: None,
        }
    }

    #[test]
    fn activity_line_formats_issue_and_merged_pr() {
        let issue = github_post(
            ActivityKind::Issue,
            ActivityState::Open,
            7,
            "Fix login",
            "alice",
        );
        let line = activity_line_text(&issue);
        assert_eq!(line, "⊙ #7 Fix login");

        let pr = github_post(
            ActivityKind::PullRequest,
            ActivityState::Merged,
            9,
            "Dark mode",
            "bob",
        );
        assert_eq!(activity_line_text(&pr), "⇄ #9 Dark mode · merged");
    }

    #[test]
    fn activity_line_appends_closed_for_issues() {
        let issue = github_post(
            ActivityKind::Issue,
            ActivityState::Closed,
            3,
            "Old bug",
            "carol",
        );
        assert_eq!(activity_line_text(&issue), "⊙ #3 Old bug · closed");
    }
}
