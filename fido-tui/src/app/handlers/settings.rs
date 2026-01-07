use crate::app::state::{App, SettingsField};
use crate::log_settings;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_settings_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    // Debug logging - log ALL keys
    log_settings!(
        app.log_config,
        "handle_settings_keys START: key={:?}",
        key.code
    );

    if app.settings_state.show_save_confirmation {
        log_settings!(app.log_config, "In save confirmation mode");

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {}
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.confirm_discard_changes();
            }
            _ => {}
        }
        return Ok(());
    }

    match key.code {
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            app.settings_state.selected_field = match app.settings_state.selected_field {
                SettingsField::ColorScheme => SettingsField::SortOrder,
                SettingsField::SortOrder => SettingsField::MaxPosts,
                SettingsField::MaxPosts => SettingsField::MaxPosts,
            };
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            app.settings_state.selected_field = match app.settings_state.selected_field {
                SettingsField::ColorScheme => SettingsField::ColorScheme,
                SettingsField::SortOrder => SettingsField::ColorScheme,
                SettingsField::MaxPosts => SettingsField::SortOrder,
            };
        }
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Left => {
            log_settings!(
                app.log_config,
                "Matched h/H/Left pattern, field: {:?}",
                app.settings_state.selected_field
            );
            match app.settings_state.selected_field {
                SettingsField::ColorScheme => app.cycle_color_scheme_backward(),
                SettingsField::SortOrder => app.cycle_sort_order_backward(),
                SettingsField::MaxPosts => app.decrement_max_posts(),
            }
        }
        KeyCode::Char('l') | KeyCode::Char('L') | KeyCode::Right | KeyCode::Enter => {
            match app.settings_state.selected_field {
                SettingsField::ColorScheme => app.cycle_color_scheme(),
                SettingsField::SortOrder => app.cycle_sort_order(),
                SettingsField::MaxPosts => app.increment_max_posts(),
            }
        }
        KeyCode::Backspace if app.settings_state.selected_field == SettingsField::MaxPosts => {
            app.remove_digit_from_max_posts();
        }
        KeyCode::Char(c) if app.settings_state.selected_field == SettingsField::MaxPosts => {
            app.add_digit_to_max_posts(c);
        }
        KeyCode::Char('s') => {}
        _ => {}
    }
    Ok(())
}
