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
    ///
    /// Uses `INSERT ... ON CONFLICT(username) DO NOTHING` then a re-select, so
    /// concurrent callers race safely to one row instead of one of them hitting
    /// the username UNIQUE constraint and 500ing.
    pub fn get_or_create_system_user(&self, username: &str) -> Result<User> {
        if let Some(user) = self.get_by_username(username)? {
            return Ok(user);
        }

        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO users (id, username, bio, join_date, is_test_user, is_admin)
             VALUES (?, ?, NULL, ?, 0, 0)
             ON CONFLICT(username) DO NOTHING",
            (
                Uuid::new_v4().to_string(),
                username,
                Utc::now().to_rfc3339(),
            ),
        )
        .context("Failed to upsert system user")?;
        drop(conn);

        self.get_by_username(username)?
            .with_context(|| format!("System user '{username}' missing after upsert"))
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

    /// Current GitHub login for a user, if any. Unlike `username` (frozen at
    /// first signup), this tracks GitHub renames on re-login.
    pub fn get_github_login(&self, user_id: &Uuid) -> Result<Option<String>> {
        let conn = self.pool.get()?;
        let login = conn
            .query_row(
                "SELECT github_login FROM users WHERE id = ?",
                [user_id.to_string()],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        Ok(login)
    }

    /// Derive a username not already taken, appending `-2`, `-3`, ... on
    /// collision. GitHub logins are unique, so a collision here means an
    /// existing local (possibly stale) username, which must not block signup.
    fn unique_username(&self, base: &str) -> Result<String> {
        if self.get_by_username(base)?.is_none() {
            return Ok(base.to_string());
        }
        for suffix in 2..=1000 {
            let candidate = format!("{base}-{suffix}");
            if self.get_by_username(&candidate)?.is_none() {
                return Ok(candidate);
            }
        }
        Err(anyhow::anyhow!(
            "Could not derive a unique username from '{base}'"
        ))
    }

    /// Create or update user from GitHub OAuth.
    ///
    /// New-user creation is idempotent: `INSERT ... ON CONFLICT(github_id) DO
    /// NOTHING` followed by a re-select, so concurrent first-time OAuth
    /// callbacks converge on one row instead of one of them hitting the
    /// `github_id`/`username` UNIQUE constraint and 500ing. A login that
    /// collides with an existing local username is disambiguated rather than
    /// rejected.
    pub fn create_or_update_from_github(
        &self,
        github_id: i64,
        github_login: &str,
        name: Option<&str>,
    ) -> Result<User> {
        // Existing user: refresh the stored login and display username on a
        // GitHub rename. The username is only re-pointed when the new login is
        // not held by a different local user (it has a UNIQUE constraint); the
        // github_login always tracks the rename regardless.
        if let Some(mut existing_user) = self.get_by_github_id(github_id)? {
            let username_is_free = match self.get_by_username(github_login)? {
                Some(other) => other.id == existing_user.id,
                None => true,
            };

            let conn = self.pool.get()?;
            if username_is_free && !existing_user.username.eq_ignore_ascii_case(github_login) {
                conn.execute(
                    "UPDATE users SET github_login = ?, username = ? WHERE id = ?",
                    [github_login, github_login, &existing_user.id.to_string()],
                )
                .context("Failed to update user GitHub identity")?;
                existing_user.username = github_login.to_string();
            } else {
                conn.execute(
                    "UPDATE users SET github_login = ? WHERE id = ?",
                    [github_login, &existing_user.id.to_string()],
                )
                .context("Failed to update user GitHub login")?;
            }

            return Ok(existing_user);
        }

        // New user: pick a free username and insert idempotently.
        let username = self.unique_username(github_login)?;
        let user_id = Uuid::new_v4();
        let join_date = Utc::now();
        let bio = name.map(|s| s.to_string());

        {
            let conn = self.pool.get()?;
            conn.execute(
                // Bare ON CONFLICT DO NOTHING (no target) swallows a conflict on
                // EITHER unique index: a concurrent callback for the same
                // github_id also picked this username, so the loser would
                // otherwise trip the username UNIQUE. The re-select below
                // recovers the winning row.
                "INSERT INTO users (id, username, bio, join_date, is_test_user, github_id, github_login)
                 VALUES (?, ?, ?, ?, 0, ?, ?)
                 ON CONFLICT DO NOTHING",
                (
                    user_id.to_string(),
                    &username,
                    bio.as_deref(),
                    join_date.to_rfc3339(),
                    github_id,
                    github_login,
                ),
            )
            .context("Failed to create user from GitHub")?;
        }

        // Re-select by github_id: returns our row, or the row a concurrent
        // callback inserted first.
        self.get_by_github_id(github_id)?
            .with_context(|| format!("GitHub user {github_id} missing after upsert"))
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

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::db::Database;

    fn repo() -> Result<(Database, UserRepository)> {
        let db = Database::in_memory()?;
        db.initialize()?;
        let repo = UserRepository::new(db.pool.clone());
        Ok((db, repo))
    }

    #[test]
    fn get_or_create_system_user_is_idempotent() -> Result<()> {
        let (_db, repo) = repo()?;
        let first = repo.get_or_create_system_user("system-bot")?;
        let second = repo.get_or_create_system_user("system-bot")?;
        assert_eq!(first.id, second.id);
        assert_eq!(second.username, "system-bot");
        Ok(())
    }

    #[test]
    fn create_or_update_from_github_is_idempotent_by_github_id() -> Result<()> {
        let (_db, repo) = repo()?;
        let first = repo.create_or_update_from_github(4242, "octocat", Some("Octo Cat"))?;
        // A repeat callback for the same github_id returns the same row, no 500.
        let second = repo.create_or_update_from_github(4242, "octocat", Some("Octo Cat"))?;
        assert_eq!(first.id, second.id);
        assert_eq!(second.username, "octocat");
        Ok(())
    }

    #[test]
    fn github_signup_disambiguates_colliding_username() -> Result<()> {
        let (_db, repo) = repo()?;
        // A pre-existing local user already holds the plain login.
        repo.create(&User {
            id: Uuid::new_v4(),
            username: "octocat".to_string(),
            bio: None,
            join_date: Utc::now(),
            is_test_user: true,
            is_admin: false,
        })?;

        // A new GitHub login colliding with that username still signs up.
        let user = repo.create_or_update_from_github(99, "octocat", None)?;
        assert_eq!(user.username, "octocat-2");
        assert_eq!(repo.get_by_github_id(99)?.map(|u| u.username).as_deref(), Some("octocat-2"));
        Ok(())
    }

    #[test]
    fn github_rename_updates_login_and_username() -> Result<()> {
        let (_db, repo) = repo()?;
        let created = repo.create_or_update_from_github(555, "old-login", None)?;
        assert_eq!(created.username, "old-login");

        // Same github_id, new login: both github_login and the display username
        // track the rename when the new login is free.
        let renamed = repo.create_or_update_from_github(555, "new-login", None)?;
        assert_eq!(renamed.id, created.id);
        assert_eq!(renamed.username, "new-login");
        assert_eq!(repo.get_github_login(&created.id)?.as_deref(), Some("new-login"));
        Ok(())
    }

    #[test]
    fn github_rename_keeps_username_when_new_login_is_taken() -> Result<()> {
        let (_db, repo) = repo()?;
        // Someone else already holds "taken".
        repo.create(&User {
            id: Uuid::new_v4(),
            username: "taken".to_string(),
            bio: None,
            join_date: Utc::now(),
            is_test_user: true,
            is_admin: false,
        })?;
        let user = repo.create_or_update_from_github(556, "original", None)?;

        // Rename to a login another user already holds: login updates, username
        // stays (UNIQUE constraint) so the row is never orphaned.
        let renamed = repo.create_or_update_from_github(556, "taken", None)?;
        assert_eq!(renamed.id, user.id);
        assert_eq!(renamed.username, "original");
        assert_eq!(repo.get_github_login(&user.id)?.as_deref(), Some("taken"));
        Ok(())
    }

    #[test]
    fn concurrent_github_signup_returns_one_row() -> Result<()> {
        use std::collections::HashSet;
        use std::sync::Arc;
        use std::thread;

        // Real file DB so a pool shared across threads sees one database.
        let tmp = std::env::temp_dir().join(format!("fido-user-race-{}.sqlite", Uuid::new_v4()));
        let db = Database::new(&tmp)?;
        db.initialize()?;
        let repo = Arc::new(UserRepository::new(db.pool.clone()));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let r = Arc::clone(&repo);
                thread::spawn(move || r.create_or_update_from_github(7777, "racecat", None))
            })
            .collect();

        let mut ids = HashSet::new();
        for handle in handles {
            // No callback may 500; each returns the shared row.
            let user = handle.join().expect("thread panicked")?;
            ids.insert(user.id);
        }
        assert_eq!(ids.len(), 1, "concurrent callbacks must converge on one row");
        assert_eq!(repo.get_by_github_id(7777)?.map(|u| u.id), ids.into_iter().next());

        let _ = std::fs::remove_file(&tmp);
        Ok(())
    }
}
