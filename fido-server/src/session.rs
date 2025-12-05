use crate::db::Database;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
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
    db: Database,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Create a new session for a user
    /// 
    /// Generates a cryptographically secure UUID v4 token and stores it in the database
    /// with a 30-day expiry period.
    /// 
    /// # Arguments
    /// * `user_id` - The UUID of the user to create a session for
    /// 
    /// # Returns
    /// * `Result<String>` - The session token on success
    pub fn create_session(&self, user_id: Uuid) -> Result<String> {
        let token = Uuid::new_v4().to_string();
        let created_at = Utc::now();
        let expires_at = created_at + Duration::days(30);
        
        let conn = self.db.connection()?;
        conn.execute(
            "INSERT INTO sessions (token, user_id, created_at, expires_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                token,
                user_id.to_string(),
                created_at.to_rfc3339(),
                expires_at.to_rfc3339(),
            ],
        )
        .context("Failed to create session")?;
        
        tracing::info!("Created session for user {}", user_id);
        Ok(token)
    }

    /// Validate a session token and return the associated user ID
    /// 
    /// Checks if the token exists in the database and has not expired.
    /// 
    /// # Arguments
    /// * `token` - The session token to validate
    /// 
    /// # Returns
    /// * `Result<Uuid>` - The user ID if the session is valid
    /// * `Err` - If the session is invalid or expired
    pub fn validate_session(&self, token: &str) -> Result<Uuid> {
        let conn = self.db.connection()?;
        
        let (user_id_str, expires_at_str): (String, String) = conn
            .query_row(
                "SELECT user_id, expires_at FROM sessions WHERE token = ?1",
                rusqlite::params![token],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .context("Session not found")?;
        
        // Parse expiry time
        let expires_at = DateTime::parse_from_rfc3339(&expires_at_str)
            .context("Failed to parse expiry time")?
            .with_timezone(&Utc);
        
        // Check if session has expired
        if Utc::now() > expires_at {
            // Clean up expired session
            self.delete_session(token)?;
            anyhow::bail!("Session has expired");
        }
        
        // Parse user ID
        let user_id = Uuid::parse_str(&user_id_str)
            .context("Failed to parse user ID")?;
        
        Ok(user_id)
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
        let conn = self.db.connection()?;
        let rows_affected = conn.execute(
            "DELETE FROM sessions WHERE token = ?1",
            rusqlite::params![token],
        )
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
        let conn = self.db.connection()?;
        let now = Utc::now().to_rfc3339();
        
        let rows_affected = conn.execute(
            "DELETE FROM sessions WHERE expires_at < ?1",
            rusqlite::params![now],
        )
        .context("Failed to cleanup expired sessions")?;
        
        if rows_affected > 0 {
            tracing::info!("Cleaned up {} expired sessions", rows_affected);
        }
        
        Ok(rows_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

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

    #[test]
    fn test_create_session() {
        let db = setup_test_db();
        let manager = SessionManager::new(db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();
        
        let token = manager.create_session(user_id).expect("Failed to create session");
        assert!(!token.is_empty());
        assert!(Uuid::parse_str(&token).is_ok(), "Token should be a valid UUID");
    }

    #[test]
    fn test_validate_session() {
        let db = setup_test_db();
        let manager = SessionManager::new(db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();
        
        let token = manager.create_session(user_id).expect("Failed to create session");
        let validated_user_id = manager.validate_session(&token).expect("Failed to validate session");
        
        assert_eq!(user_id, validated_user_id);
    }

    #[test]
    fn test_validate_invalid_session() {
        let db = setup_test_db();
        let manager = SessionManager::new(db);
        
        let result = manager.validate_session("invalid-token");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_session() {
        let db = setup_test_db();
        let manager = SessionManager::new(db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();
        
        let token = manager.create_session(user_id).expect("Failed to create session");
        manager.delete_session(&token).expect("Failed to delete session");
        
        let result = manager.validate_session(&token);
        assert!(result.is_err(), "Session should be invalid after deletion");
    }

    #[test]
    fn test_cleanup_expired_sessions() {
        let db = setup_test_db();
        let manager = SessionManager::new(db.clone());
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();
        
        // Create a session
        let token = manager.create_session(user_id).expect("Failed to create session");
        
        // Manually expire the session
        let conn = db.connection().expect("Failed to get connection");
        let expired_time = (Utc::now() - Duration::days(1)).to_rfc3339();
        conn.execute(
            "UPDATE sessions SET expires_at = ?1 WHERE token = ?2",
            rusqlite::params![expired_time, token],
        )
        .expect("Failed to expire session");
        
        // Cleanup should remove the expired session
        let cleaned = manager.cleanup_expired_sessions().expect("Failed to cleanup");
        assert_eq!(cleaned, 1);
        
        // Session should no longer be valid
        let result = manager.validate_session(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_token_uniqueness() {
        let db = setup_test_db();
        let manager = SessionManager::new(db);
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440099").unwrap();
        
        // Create multiple sessions
        let token1 = manager.create_session(user_id).expect("Failed to create session 1");
        let token2 = manager.create_session(user_id).expect("Failed to create session 2");
        let token3 = manager.create_session(user_id).expect("Failed to create session 3");
        
        // All tokens should be unique
        assert_ne!(token1, token2);
        assert_ne!(token2, token3);
        assert_ne!(token1, token3);
    }
}
