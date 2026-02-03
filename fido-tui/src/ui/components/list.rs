use ratatui::{
    style::Style,
    widgets::{List, ListItem},
};

use crate::ui::theme::ThemeColors;

/// Build a list with consistent highlight styling.
pub fn styled_list<'a>(
    items: Vec<ListItem<'a>>,
    theme: &ThemeColors,
    highlight_symbol: Option<&'a str>,
) -> List<'a> {
    let mut list = List::new(items).highlight_style(Style::default().bg(theme.highlight_bg));
    if let Some(symbol) = highlight_symbol {
        list = list.highlight_symbol(symbol);
    }
    list
}
