use anyhow::{Context, Result};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use fido_types::{ColorScheme, SortOrder, UserConfig};

use crate::db::DbPool;

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

                Ok(UserConfig {
                    user_id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    color_scheme: ColorScheme::parse(&color_scheme_str).unwrap_or_default(),
                    sort_order: SortOrder::parse(&sort_order_str).unwrap_or_default(),
                    max_posts_display: row.get(3)?,
                    emoji_enabled: row.get::<_, i32>(4)? == 1,
                })
            })
            .optional()?;

        // Return default config if not found
        Ok(config.unwrap_or_else(|| {
            let mut default = UserConfig::default();
            default.user_id = *user_id;
            default
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

    /// Create default configuration for a user
    #[allow(dead_code)]
    pub fn create_default(&self, user_id: &Uuid) -> Result<()> {
        let mut config = UserConfig::default();
        config.user_id = *user_id;
        self.update(&config)
    }
}
