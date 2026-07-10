use anyhow::{Context, Result};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use fido_types::{ColorScheme, SortOrder, UserConfig};

use crate::db::{row, DbPool};

#[derive(Clone)]
pub struct ConfigRepository {
    pool: DbPool,
}

impl ConfigRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get user configuration
    pub fn get(&self, user_id: &Uuid) -> Result<UserConfig> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT user_id, color_scheme, sort_order, max_posts_display, emoji_enabled
             FROM user_configs
             WHERE user_id = ?",
        )?;

        let config = stmt
            .query_row([user_id.to_string()], |row| {
                let color_scheme_str: String = row.get(1)?;
                let sort_order_str: String = row.get(2)?;

                let user_id_str: String = row.get(0)?;
                Ok(UserConfig {
                    user_id: row::parse_uuid(&user_id_str, 0)?,
                    color_scheme: ColorScheme::parse(&color_scheme_str).unwrap_or_default(),
                    sort_order: SortOrder::parse(&sort_order_str).unwrap_or_default(),
                    max_posts_display: row.get(3)?,
                    emoji_enabled: row.get::<_, i32>(4)? == 1,
                })
            })
            .optional()?;

        // Return default config if not found
        Ok(config.unwrap_or_else(|| UserConfig {
            user_id: *user_id,
            ..Default::default()
        }))
    }

    /// Update user configuration
    pub fn update(&self, config: &UserConfig) -> Result<()> {
        let conn = self.pool.get()?;

        conn.execute(
            "INSERT INTO user_configs (user_id, color_scheme, sort_order, max_posts_display, emoji_enabled)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(user_id) 
             DO UPDATE SET 
                color_scheme = excluded.color_scheme,
                sort_order = excluded.sort_order,
                max_posts_display = excluded.max_posts_display,
                emoji_enabled = excluded.emoji_enabled",
            (
                config.user_id.to_string(),
                config.color_scheme.as_str(),
                config.sort_order.as_str(),
                config.max_posts_display,
                if config.emoji_enabled { 1 } else { 0 },
            ),
        ).context("Failed to update user config")?;

        Ok(())
    }
}
