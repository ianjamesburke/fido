use crate::stores::SessionStore;
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;

/// Database-backed session manager for persistent authentication
///
/// Manages user sessions with token-based authentication, including:
/// - Session creation with UUID v4 tokens
/// - Session validation with expiry checking
/// - Session deletion (logout)
/// - Automatic cleanup of expired sessions
#[derive(Clone)]
pub struct SessionManager {
    store: Arc<dyn SessionStore>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(store: Arc<dyn SessionStore>) -> Self {
        Self { store }
    }

    /// Create a new session for a user
    ///
    /// Generates a cryptographically secure UUID v4 token and stores it in the database
    /// with a 7-day expiry period (reduced from 90 days for security).
    ///
    /// # Arguments
    /// * `user_id` - The UUID of the user to create a session for
    ///
    /// # Returns
    /// * `Result<String>` - The session token on success
    pub fn create_session(&self, user_id: Uuid) -> Result<String> {
        let token = Uuid::new_v4().to_string();
        let created_at = Utc::now();
        let expires_at = created_at + Duration::days(7); // Reduced from 90 days to 7 days for security
        let last_activity = created_at; // Initialize last_activity to creation time

        self.store
            .create_session(&token, user_id, created_at, expires_at, last_activity)
            .context("Failed to create session")?;

        tracing::info!("Created session for user {}", user_id);
        Ok(token)
    }

    /// Validate a session token and return the associated user ID
    ///
    /// Checks if the token exists in the database, has not expired, and is not idle.
    /// Sessions are invalidated if idle for more than 24 hours.
    /// Updates last_activity on successful validation.
    ///
    /// # Arguments
    /// * `token` - The session token to validate
    ///
    /// # Returns
    /// * `Result<Uuid>` - The user ID if the session is valid
    /// * `Err` - If the session is invalid, expired, or idle too long
    pub fn validate_session(&self, token: &str) -> Result<Uuid> {
        let session = self
            .store
            .get_session(token)
            .context("Session lookup failed")?;
        let session = session.context("Session not found")?;

        let now = Utc::now();
        let expires_at = session.expires_at;

        // Check if session has expired
        if now > expires_at {
            // Clean up expired session
            self.delete_session(token)?;
            anyhow::bail!("Session has expired");
        }

        // Check idle timeout (24 hours)
        if let Some(last_activity) = session.last_activity {
            let idle_duration = now - last_activity;
            let max_idle_hours = 24;

            if idle_duration > Duration::hours(max_idle_hours) {
                // Clean up idle session
                self.delete_session(token)?;
                tracing::info!(
                    "Session invalidated due to idle timeout (idle for {} hours)",
                    idle_duration.num_hours()
                );
                anyhow::bail!("Session has been idle too long");
            }
        }

        // Parse user ID
        let user_id = session.user_id;

        // Update last_activity on successful validation
        self.update_activity(token)?;

        Ok(user_id)
    }

    /// Update the last activity timestamp for a session
    ///
    /// This should be called on each validated request to track session activity.
    ///
    /// # Arguments
    /// * `token` - The session token to update
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    pub fn update_activity(&self, token: &str) -> Result<()> {
        self.store
            .update_activity(token, Utc::now())
            .context("Failed to update session activity")?;

        Ok(())
    }

    /// Delete a session (logout)
    ///
    /// Removes the session from the database, effectively logging out the user.
    ///
    /// # Arguments
    /// * `token` - The session token to delete
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    pub fn delete_session(&self, token: &str) -> Result<()> {
        let rows_affected = self
            .store
            .delete_session(token)
            .context("Failed to delete session")?;

        if rows_affected > 0 {
            tracing::info!("Deleted session");
        }

        Ok(())
    }

    /// Clean up expired sessions from the database
    ///
    /// Removes all sessions that have passed their expiry time.
    /// This should be called periodically to prevent database bloat.
    ///
    /// # Returns
    /// * `Result<usize>` - The number of sessions deleted
    pub fn cleanup_expired_sessions(&self) -> Result<usize> {
        let rows_affected = self
            .store
            .cleanup_expired_sessions(Utc::now())
            .context("Failed to cleanup expired sessions")?;

        if rows_affected > 0 {
            tracing::info!("Cleaned up {} expired sessions", rows_affected);
        }

        Ok(rows_affected)
    }

    /// Invalidate all sessions for a user
    ///
    /// Removes all sessions associated with a user from the database.
    /// This should be called before creating a new session on login to ensure
    /// that previous sessions are invalidated for security.
    ///
    /// # Arguments
    /// * `user_id` - The UUID of the user whose sessions should be invalidated
    ///
    /// # Returns
    /// * `Result<usize>` - The number of sessions invalidated
    pub fn invalidate_user_sessions(&self, user_id: Uuid) -> Result<usize> {
        let rows_affected = self
            .store
            .invalidate_user_sessions(user_id)
            .context("Failed to invalidate user sessions")?;

        if rows_affected > 0 {
            tracing::info!(
                "Invalidated {} sessions for user {}",
                rows_affected,
                user_id
            );
        }

        Ok(rows_affected)
    }
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::stores::sqlite::SqliteSessionStore;
    use chrono::DateTime;
    use std::sync::Arc;

    fn setup_test_db() -> Database {
        let db = Database::in_memory().expect("Failed to create test database");
        db.initialize().expect("Failed to initialize database");

        // Create a test user
        let conn = db.connection().expect("Failed to get connection");
        conn.execute(
            "INSERT INTO users (id, username, bio, join_date, is_test_user) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                "550e8400-e29b-41d4-a716-446655440099",
                "testuser",
                "Test user",
                Utc::now().to_rfc3339(),
                1,
            ],
        )
        .expect("Failed to create test user");

        db
    }

    fn setup_manager(db: &Database) -> SessionManager {
        SessionManager::new(Arc::new(SqliteSessionStore::new(db.pool.clone())))
    }

    #[test]
    fn test_create_session() {
        let db = setup_test_db();
        let manager = setup_manager(&db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();

        let token = manager
            .create_session(user_id)
            .expect("Failed to create session");
        assert!(!token.is_empty());
        assert!(
            Uuid::parse_str(&token).is_ok(),
            "Token should be a valid UUID"
        );
    }

    #[test]
    fn test_validate_session() {
        let db = setup_test_db();
        let manager = setup_manager(&db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();

        let token = manager
            .create_session(user_id)
            .expect("Failed to create session");
        let validated_user_id = manager
            .validate_session(&token)
            .expect("Failed to validate session");

        assert_eq!(user_id, validated_user_id);
    }

    #[test]
    fn test_validate_invalid_session() {
        let db = setup_test_db();
        let manager = setup_manager(&db);

        let result = manager.validate_session("invalid-token");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_session() {
        let db = setup_test_db();
        let manager = setup_manager(&db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();

        let token = manager
            .create_session(user_id)
            .expect("Failed to create session");
        manager
            .delete_session(&token)
            .expect("Failed to delete session");

        let result = manager.validate_session(&token);
        assert!(result.is_err(), "Session should be invalid after deletion");
    }

    #[test]
    fn test_cleanup_expired_sessions() {
        let db = setup_test_db();
        let manager = setup_manager(&db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();

        // Create a session
        let token = manager
            .create_session(user_id)
            .expect("Failed to create session");

        // Manually expire the session
        let conn = db.connection().expect("Failed to get connection");
        let expired_time = (Utc::now() - Duration::days(1)).to_rfc3339();
        conn.execute(
            "UPDATE sessions SET expires_at = ?1 WHERE token = ?2",
            rusqlite::params![expired_time, token],
        )
        .expect("Failed to expire session");

        // Cleanup should remove the expired session
        let cleaned = manager
            .cleanup_expired_sessions()
            .expect("Failed to cleanup");
        assert_eq!(cleaned, 1);

        // Session should no longer be valid
        let result = manager.validate_session(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_token_uniqueness() {
        let db = setup_test_db();
        let manager = setup_manager(&db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();

        // Create multiple sessions
        let token1 = manager
            .create_session(user_id)
            .expect("Failed to create session 1");
        let token2 = manager
            .create_session(user_id)
            .expect("Failed to create session 2");
        let token3 = manager
            .create_session(user_id)
            .expect("Failed to create session 3");

        // All tokens should be unique
        assert_ne!(token1, token2);
        assert_ne!(token2, token3);
        assert_ne!(token1, token3);
    }

    #[test]
    fn test_idle_timeout_invalidates_session() {
        let db = setup_test_db();
        let manager = setup_manager(&db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();

        // Create a session
        let token = manager
            .create_session(user_id)
            .expect("Failed to create session");

        // Manually set last_activity to 25 hours ago (beyond 24-hour idle timeout)
        let conn = db.connection().expect("Failed to get connection");
        let old_activity = (Utc::now() - Duration::hours(25)).to_rfc3339();
        conn.execute(
            "UPDATE sessions SET last_activity = ?1 WHERE token = ?2",
            rusqlite::params![old_activity, token],
        )
        .expect("Failed to update last_activity");

        // Session should be invalid due to idle timeout
        let result = manager.validate_session(&token);
        assert!(
            result.is_err(),
            "Session should be invalid after idle timeout"
        );

        // Verify the error message mentions idle
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("idle") || err_msg.contains("Session not found"),
            "Error should mention idle timeout or session not found (deleted)"
        );
    }

    #[test]
    fn test_active_session_not_invalidated() {
        let db = setup_test_db();
        let manager = setup_manager(&db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();

        // Create a session
        let token = manager
            .create_session(user_id)
            .expect("Failed to create session");

        // Set last_activity to 23 hours ago (within 24-hour idle timeout)
        let conn = db.connection().expect("Failed to get connection");
        let recent_activity = (Utc::now() - Duration::hours(23)).to_rfc3339();
        conn.execute(
            "UPDATE sessions SET last_activity = ?1 WHERE token = ?2",
            rusqlite::params![recent_activity, token],
        )
        .expect("Failed to update last_activity");

        // Session should still be valid
        let result = manager.validate_session(&token);
        assert!(
            result.is_ok(),
            "Session should be valid within idle timeout"
        );
        assert_eq!(result.unwrap(), user_id);
    }

    #[test]
    fn test_update_activity() {
        let db = setup_test_db();
        let manager = setup_manager(&db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();

        // Create a session
        let token = manager
            .create_session(user_id)
            .expect("Failed to create session");

        // Get initial last_activity
        let conn = db.connection().expect("Failed to get connection");
        let initial_activity: String = conn
            .query_row(
                "SELECT last_activity FROM sessions WHERE token = ?1",
                rusqlite::params![token],
                |row| row.get(0),
            )
            .expect("Failed to get last_activity");

        // Wait a tiny bit and update activity
        std::thread::sleep(std::time::Duration::from_millis(10));
        manager
            .update_activity(&token)
            .expect("Failed to update activity");

        // Get updated last_activity
        let updated_activity: String = conn
            .query_row(
                "SELECT last_activity FROM sessions WHERE token = ?1",
                rusqlite::params![token],
                |row| row.get(0),
            )
            .expect("Failed to get updated last_activity");

        // Activity should have been updated
        assert_ne!(
            initial_activity, updated_activity,
            "last_activity should be updated"
        );
    }

    #[test]
    fn test_validate_session_updates_activity() {
        let db = setup_test_db();
        let manager = setup_manager(&db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();

        // Create a session
        let token = manager
            .create_session(user_id)
            .expect("Failed to create session");

        // Set last_activity to 1 hour ago
        let conn = db.connection().expect("Failed to get connection");
        let old_activity = (Utc::now() - Duration::hours(1)).to_rfc3339();
        conn.execute(
            "UPDATE sessions SET last_activity = ?1 WHERE token = ?2",
            rusqlite::params![old_activity, token],
        )
        .expect("Failed to update last_activity");

        // Validate session (should update activity)
        manager
            .validate_session(&token)
            .expect("Failed to validate session");

        // Get updated last_activity
        let updated_activity: String = conn
            .query_row(
                "SELECT last_activity FROM sessions WHERE token = ?1",
                rusqlite::params![token],
                |row| row.get(0),
            )
            .expect("Failed to get updated last_activity");

        // Activity should have been updated to recent time
        let updated_time = DateTime::parse_from_rfc3339(&updated_activity)
            .expect("Failed to parse updated activity")
            .with_timezone(&Utc);

        let time_diff = Utc::now() - updated_time;
        assert!(
            time_diff < Duration::seconds(5),
            "last_activity should be updated to recent time"
        );
    }

    #[test]
    fn test_invalidate_user_sessions() {
        let db = setup_test_db();
        let manager = setup_manager(&db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();

        // Create multiple sessions for the user
        let token1 = manager
            .create_session(user_id)
            .expect("Failed to create session 1");
        let token2 = manager
            .create_session(user_id)
            .expect("Failed to create session 2");
        let token3 = manager
            .create_session(user_id)
            .expect("Failed to create session 3");

        // All sessions should be valid
        assert!(manager.validate_session(&token1).is_ok());
        assert!(manager.validate_session(&token2).is_ok());
        assert!(manager.validate_session(&token3).is_ok());

        // Invalidate all sessions for the user
        let invalidated = manager
            .invalidate_user_sessions(user_id)
            .expect("Failed to invalidate sessions");
        assert_eq!(invalidated, 3, "Should have invalidated 3 sessions");

        // All sessions should now be invalid
        assert!(
            manager.validate_session(&token1).is_err(),
            "Session 1 should be invalid after invalidation"
        );
        assert!(
            manager.validate_session(&token2).is_err(),
            "Session 2 should be invalid after invalidation"
        );
        assert!(
            manager.validate_session(&token3).is_err(),
            "Session 3 should be invalid after invalidation"
        );
    }

    #[test]
    fn test_invalidate_user_sessions_no_sessions() {
        let db = setup_test_db();
        let manager = setup_manager(&db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();

        // Invalidate sessions for a user with no sessions
        let invalidated = manager
            .invalidate_user_sessions(user_id)
            .expect("Failed to invalidate sessions");
        assert_eq!(invalidated, 0, "Should have invalidated 0 sessions");
    }
}
