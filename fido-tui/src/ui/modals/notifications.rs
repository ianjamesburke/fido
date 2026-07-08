use chrono::{DateTime, Utc};
use fido_types::{Notification, NotificationType};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

use super::social_components::{render_loading_state, render_modal_container, ModalConfig};
use crate::app::App;
use crate::ui::components::empty_state::render_empty_state;
use crate::ui::components::footer::render_footer;
use crate::ui::theme::{get_theme_colors, ThemeColors};

pub fn render_notifications_panel(frame: &mut Frame, app: &App, area: Rect) {
    let theme = get_theme_colors(app);
    let unread = app.total_unread_notifications();
    let title = if unread > 0 {
        format!(" Notifications ({}) ", unread)
    } else {
        " Notifications ".to_string()
    };
    let config = ModalConfig::new(&title).with_size(64, 76);
    let inner = render_modal_container(frame, area, &config, &theme);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(inner);

    let summary = if app.notifications_state.loading {
        "Loading recent notifications..."
    } else if app.notifications_state.notifications.is_empty() {
        "No notifications"
    } else {
        "Recent notifications"
    };
    frame.render_widget(
        Paragraph::new(summary).style(Style::default().fg(theme.text_dim)),
        chunks[0],
    );

    if app.notifications_state.loading {
        render_loading_state(frame, chunks[1], "Loading notifications...", &theme);
    } else if let Some(error) = &app.notifications_state.error {
        render_empty_state(frame, chunks[1], error.as_str(), &theme);
    } else if app.notifications_state.notifications.is_empty() {
        render_empty_state(frame, chunks[1], "You're caught up", &theme);
    } else {
        render_notification_list(frame, app, chunks[1], &theme);
    }

    render_footer(
        frame,
        chunks[2],
        "↑/↓/j/k: Navigate | Enter: Open + mark read | a: Mark all read | v/Esc: Close",
        &theme,
    );
}

fn render_notification_list(frame: &mut Frame, app: &App, area: Rect, theme: &ThemeColors) {
    let items: Vec<ListItem> = app
        .notifications_state
        .notifications
        .iter()
        .map(|notification| render_notification_item(notification, theme))
        .collect();

    let list = List::new(items).highlight_symbol(">> ").highlight_style(
        Style::default()
            .bg(theme.highlight_bg)
            .add_modifier(Modifier::BOLD),
    );

    let mut state = ListState::default();
    state.select(Some(
        app.notifications_state.selected_index.min(
            app.notifications_state
                .notifications
                .len()
                .saturating_sub(1),
        ),
    ));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_notification_item(notification: &Notification, theme: &ThemeColors) -> ListItem<'static> {
    let unread = if notification.read { " " } else { "*" };
    let (kind, label) = notification_label(notification.notification_type);
    let age = relative_time(notification.created_at);
    let style = if notification.read {
        Style::default().fg(theme.text_dim)
    } else {
        Style::default().fg(theme.text)
    };

    let source = match notification.subject_type.as_str() {
        "community" => "community board".to_string(),
        "dm_conversation" => "direct messages".to_string(),
        other => other.to_string(),
    };

    ListItem::new(vec![
        Line::from(vec![
            Span::styled(unread, Style::default().fg(theme.warning)),
            Span::raw(" "),
            Span::styled(kind, Style::default().fg(theme.accent)),
            Span::raw(" "),
            Span::styled(label, style),
            Span::raw(" "),
            Span::styled(age, Style::default().fg(theme.text_dim)),
        ]),
        Line::from(Span::styled(
            format!("  {}", source),
            Style::default().fg(theme.text_dim),
        )),
    ])
}

fn notification_label(kind: NotificationType) -> (&'static str, &'static str) {
    match kind {
        NotificationType::Mention => ("@", "Mentioned you"),
        NotificationType::Reply => ("R", "Replied to you"),
        NotificationType::DmRequest => ("DM", "Sent you a DM request"),
        NotificationType::ThreadApproved => ("OK", "Approved your thread"),
        NotificationType::ThreadRejected => ("X", "Rejected your thread"),
    }
}

fn relative_time(created_at: DateTime<Utc>) -> String {
    let age = Utc::now().signed_duration_since(created_at);
    if age.num_seconds() < 60 {
        "now".to_string()
    } else if age.num_minutes() < 60 {
        format!("{}m ago", age.num_minutes())
    } else if age.num_hours() < 24 {
        format!("{}h ago", age.num_hours())
    } else {
        format!("{}d ago", age.num_days())
    }
}
