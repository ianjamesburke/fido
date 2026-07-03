use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use fido_types::MembershipRole;

use super::super::components::modal::{render_modal_container, ModalConfig};
use super::super::theme::get_theme_colors;
use crate::app::App;

const MAX_ADMINS_SHOWN: usize = 5;

/// Community settings modal: the caller's standing in the community and the
/// tucked-away actions (claiming admin; future moderation settings).
pub fn render_community_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme_colors(app);

    let Some(community) = app.community.clone() else {
        return;
    };

    let config = ModalConfig::new(" Community ").with_size(55, 60);
    let inner = render_modal_container(frame, area, &config, &theme);

    let role = community
        .role
        .map(|r| r.as_str().to_string())
        .unwrap_or_else(|| "not a member".to_string());
    let claimed_status = if community.claimed {
        "claimed"
    } else {
        "unclaimed"
    };

    let label_style = Style::default().fg(theme.text_dim);
    let value_style = Style::default().fg(theme.text).add_modifier(Modifier::BOLD);

    let admins: Vec<&str> = app
        .community_members
        .iter()
        .filter(|m| m.role == MembershipRole::Admin)
        .map(|m| m.username.as_str())
        .collect();
    let admins_value = if admins.is_empty() {
        "unclaimed — none".to_string()
    } else if admins.len() > MAX_ADMINS_SHOWN {
        format!(
            "{}, +{} more",
            admins[..MAX_ADMINS_SHOWN].join(", "),
            admins.len() - MAX_ADMINS_SHOWN
        )
    } else {
        admins.join(", ")
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            community.full_name(),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Owner:      ", label_style),
            Span::styled(community.owner.clone(), value_style),
        ]),
        Line::from(vec![
            Span::styled("Your role:  ", label_style),
            Span::styled(role, value_style),
        ]),
        Line::from(vec![
            Span::styled("Members:    ", label_style),
            Span::styled(community.member_count.to_string(), value_style),
        ]),
        Line::from(vec![
            Span::styled("Admin:      ", label_style),
            Span::styled(claimed_status.to_string(), value_style),
        ]),
        Line::from(vec![
            Span::styled("Admins:     ", label_style),
            Span::styled(admins_value, value_style),
        ]),
        Line::from(""),
    ];

    if !community.claimed {
        lines.push(Line::from(Span::styled(
            "c: Claim admin (requires GitHub admin/maintain on the repo)",
            Style::default().fg(theme.success),
        )));
    }
    lines.push(Line::from(Span::styled(
        "Esc: Close",
        Style::default().fg(theme.text_dim),
    )));

    let content = Paragraph::new(lines)
        .style(Style::default().fg(theme.text))
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(content, inner);
}
