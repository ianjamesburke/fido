use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use crate::db::DbPool;

#[derive(Debug, Clone)]
pub struct GitHubTokenRecord {
    pub encrypted_token: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct GitHubTokenRepository {
    pool: DbPool,
}

impl GitHubTokenRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn upsert_token(
        &self,
        user_id: Uuid,
        encrypted_token: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO github_tokens (user_id, encrypted_token, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(user_id) DO UPDATE SET
                encrypted_token = excluded.encrypted_token,
                updated_at = excluded.updated_at",
            rusqlite::params![
                user_id.to_string(),
                encrypted_token,
                now.to_rfc3339(),
                now.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_token(&self, user_id: Uuid) -> Result<Option<GitHubTokenRecord>> {
        let conn = self.pool.get()?;
        let row: Option<(String, String, String)> = conn
            .query_row(
                "SELECT encrypted_token, created_at, updated_at
                 FROM github_tokens
                 WHERE user_id = ?1",
                rusqlite::params![user_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;

        match row {
            Some((encrypted_token, created_at, updated_at)) => Ok(Some(GitHubTokenRecord {
                encrypted_token,
                created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
            })),
            None => Ok(None),
        }
    }

    pub fn delete_token(&self, user_id: Uuid) -> Result<usize> {
        let conn = self.pool.get()?;
        Ok(conn.execute(
            "DELETE FROM github_tokens WHERE user_id = ?1",
            rusqlite::params![user_id.to_string()],
        )?)
    }
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::db::Database;

    fn setup_db() -> Database {
        let db = Database::in_memory().expect("Failed to create test database");
        db.initialize().expect("Failed to initialize test database");

        let conn = db.connection().expect("Failed to get test connection");
        conn.execute(
            "INSERT INTO users (id, username, bio, join_date, is_test_user)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                "550e8400-e29b-41d4-a716-446655440099",
                "token-user",
                "Token user",
                Utc::now().to_rfc3339(),
                0,
            ],
        )
        .expect("Failed to insert test user");

        db
    }

    #[test]
    fn upsert_and_delete_github_token() {
        let db = setup_db();
        let repo = GitHubTokenRepository::new(db.pool.clone());
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();
        let now = Utc::now();

        repo.upsert_token(user_id, "ciphertext-1", now)
            .expect("Failed to insert token");
        let stored = repo
            .get_token(user_id)
            .expect("Failed to fetch token")
            .expect("Token should exist");
        assert_eq!(stored.encrypted_token, "ciphertext-1");
        assert_eq!(stored.created_at, now);

        let updated_at = now + chrono::Duration::minutes(5);
        repo.upsert_token(user_id, "ciphertext-2", updated_at)
            .expect("Failed to update token");
        let updated = repo
            .get_token(user_id)
            .expect("Failed to fetch updated token")
            .expect("Updated token should exist");
        assert_eq!(updated.encrypted_token, "ciphertext-2");
        assert_eq!(updated.created_at, now);
        assert_eq!(updated.updated_at, updated_at);

        let deleted = repo.delete_token(user_id).expect("Failed to delete token");
        assert_eq!(deleted, 1);
        assert!(repo.get_token(user_id).unwrap().is_none());
    }
}
