use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::OptionalExtension;
use uuid::Uuid;

use fido_types::User;

use crate::db::{row, DbPool};

#[derive(Clone)]
pub struct UserRepository {
    pool: DbPool,
}

impl UserRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get all test users
    pub fn get_test_users(&self) -> Result<Vec<User>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, username, bio, join_date, is_test_user, is_admin 
             FROM users 
             WHERE is_test_user = 1
             ORDER BY username",
        )?;

        let users = stmt
            .query_map([], map_user_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(users)
    }

    /// Get user by ID
    pub fn get_by_id(&self, user_id: &Uuid) -> Result<Option<User>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, username, bio, join_date, is_test_user, is_admin 
             FROM users 
             WHERE id = ?",
        )?;

        let user = stmt
            .query_row([user_id.to_string()], map_user_row)
            .optional()?;

        Ok(user)
    }

    /// Get user by username. Case-insensitive: usernames are GitHub logins,
    /// which GitHub treats as case-insensitive.
    pub fn get_by_username(&self, username: &str) -> Result<Option<User>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, username, bio, join_date, is_test_user, is_admin
             FROM users
             WHERE username = ? COLLATE NOCASE",
        )?;

        let user = stmt.query_row([username], map_user_row).optional()?;

        Ok(user)
    }

    /// Update user bio
    pub fn update_bio(&self, user_id: &Uuid, bio: &str) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE users SET bio = ? WHERE id = ?",
            [bio, &user_id.to_string()],
        )
        .context("Failed to update user bio")?;
        Ok(())
    }

    /// Create a new user (for future non-test users)
    pub fn create(&self, user: &User) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO users (id, username, bio, join_date, is_test_user, is_admin) 
             VALUES (?, ?, ?, ?, ?, ?)",
            (
                user.id.to_string(),
                &user.username,
                &user.bio,
                user.join_date.to_rfc3339(),
                if user.is_test_user { 1 } else { 0 },
                if user.is_admin { 1 } else { 0 },
            ),
        )
        .context("Failed to create user")?;
        Ok(())
    }

    /// Get a system user by name, creating it on first use.
    pub fn get_or_create_system_user(&self, username: &str) -> Result<User> {
        if let Some(user) = self.get_by_username(username)? {
            return Ok(user);
        }

        let user = User {
            id: Uuid::new_v4(),
            username: username.to_string(),
            bio: None,
            join_date: Utc::now(),
            is_test_user: false,
            is_admin: false,
        };
        self.create(&user)?;
        Ok(user)
    }

    /// Get all users (for search)
    pub fn list_all(&self) -> Result<Vec<User>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, username, bio, join_date, is_test_user, is_admin 
             FROM users 
             ORDER BY username",
        )?;

        let users = stmt
            .query_map([], map_user_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(users)
    }

    /// Find user by username (alias for get_by_username for consistency)
    #[allow(dead_code)]
    pub fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        self.get_by_username(username)
    }

    /// Get user by GitHub ID
    pub fn get_by_github_id(&self, github_id: i64) -> Result<Option<User>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, username, bio, join_date, is_test_user, is_admin 
             FROM users 
             WHERE github_id = ?",
        )?;

        let user = stmt.query_row([github_id], map_user_row).optional()?;

        Ok(user)
    }

    /// Create or update user from GitHub OAuth
    pub fn create_or_update_from_github(
        &self,
        github_id: i64,
        github_login: &str,
        name: Option<&str>,
    ) -> Result<User> {
        let conn = self.pool.get()?;

        // Check if user already exists
        if let Some(existing_user) = self.get_by_github_id(github_id)? {
            // Update github_login if it changed
            conn.execute(
                "UPDATE users SET github_login = ? WHERE id = ?",
                [github_login, &existing_user.id.to_string()],
            )
            .context("Failed to update user GitHub login")?;

            return Ok(existing_user);
        }

        // Create new user
        let user_id = Uuid::new_v4();
        let join_date = Utc::now();
        let bio = name.map(|s| s.to_string());

        conn.execute(
            "INSERT INTO users (id, username, bio, join_date, is_test_user, github_id, github_login) 
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            (
                user_id.to_string(),
                github_login,
                bio.as_deref(),
                join_date.to_rfc3339(),
                0, // Not a test user
                github_id,
                github_login,
            ),
        ).context("Failed to create user from GitHub")?;

        Ok(User {
            id: user_id,
            username: github_login.to_string(),
            bio,
            join_date,
            is_test_user: false,
            is_admin: false,
        })
    }
}

fn map_user_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
    let id_str: String = row.get(0)?;
    let join_date_str: String = row.get(3)?;

    Ok(User {
        id: row::parse_uuid(&id_str, 0)?,
        username: row.get(1)?,
        bio: row.get(2)?,
        join_date: row::parse_datetime(&join_date_str, 3)?,
        is_test_user: row.get::<_, i32>(4)? == 1,
        is_admin: row.get::<_, i32>(5)? == 1,
    })
}
