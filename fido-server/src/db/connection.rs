use anyhow::{Context, Result};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::path::Path;

use super::schema::{SCHEMA, TEST_DATA};

/// SQLite in-memory database identifier
const MEMORY_DB_PATH: &str = ":memory:";

pub type DbPool = Pool<SqliteConnectionManager>;
pub type DbConnection = PooledConnection<SqliteConnectionManager>;

/// Database wrapper with connection pooling support
#[derive(Clone)]
pub struct Database {
    pub pool: DbPool,
}

impl Database {
    /// Create a new database connection pool
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let manager = Self::create_connection_manager(path)?;
        let pool = Pool::new(manager).context("Failed to create database connection pool")?;
        Ok(Self { pool })
    }

    /// Create appropriate connection manager based on path
    ///
    /// # Arguments
    /// * `path` - Database file path or ":memory:" for in-memory database
    ///
    /// # Returns
    /// * `SqliteConnectionManager` configured for file or memory storage
    fn create_connection_manager<P: AsRef<Path>>(path: P) -> Result<SqliteConnectionManager> {
        let path_str = path.as_ref().to_string_lossy();
        let trimmed_path = path_str.trim();

        if trimmed_path.eq_ignore_ascii_case(MEMORY_DB_PATH) {
            Ok(SqliteConnectionManager::memory())
        } else {
            Ok(SqliteConnectionManager::file(path))
        }
    }

    /// Create an in-memory database pool (useful for testing)
    #[allow(dead_code)]
    pub fn in_memory() -> Result<Self> {
        Self::new(MEMORY_DB_PATH)
    }

    /// Initialize the database schema
    pub fn initialize(&self) -> Result<()> {
        let conn = self.connection()?;
        conn.execute_batch(SCHEMA)
            .context("Failed to initialize database schema")?;

        // Migrate existing tables - add new columns if they don't exist
        // This is safe to run multiple times (will fail silently if columns exist)
        let _ = conn.execute(
            "ALTER TABLE direct_messages ADD COLUMN deleted_by_from_user INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE direct_messages ADD COLUMN deleted_by_to_user INTEGER NOT NULL DEFAULT 0",
            [],
        );

        // Add threaded conversation support to posts table
        let _ = conn.execute("ALTER TABLE posts ADD COLUMN parent_post_id TEXT NULL", []);
        let _ = conn.execute(
            "ALTER TABLE posts ADD COLUMN reply_count INTEGER NOT NULL DEFAULT 0",
            [],
        );

        // Create index on parent_post_id for efficient reply queries
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_posts_parent_id ON posts(parent_post_id)",
            [],
        );

        // Add sessions table for authentication
        let _ = conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                token TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);",
        );

        // Add GitHub authentication fields to users table
        let _ = conn.execute("ALTER TABLE users ADD COLUMN github_id INTEGER", []);
        let _ = conn.execute("ALTER TABLE users ADD COLUMN github_login TEXT", []);
        let _ = conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_users_github_id ON users(github_id)",
            [],
        );

        Ok(())
    }

    /// Seed the database with test data
    pub fn seed_test_data(&self) -> Result<()> {
        let conn = self.connection()?;
        conn.execute_batch(TEST_DATA)
            .context("Failed to seed test data")?;
        Ok(())
    }

    /// Get a connection from the pool
    pub fn connection(&self) -> Result<DbConnection> {
        self.pool
            .get()
            .context("Failed to get database connection from pool")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = Database::in_memory().expect("Failed to create database");
        db.initialize().expect("Failed to initialize schema");

        // Verify tables exist
        let conn = db.connection().expect("Failed to get connection");
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .expect("Failed to prepare statement");

        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .expect("Failed to query tables")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect tables");

        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"posts".to_string()));
        assert!(tables.contains(&"hashtags".to_string()));
        assert!(tables.contains(&"votes".to_string()));
        assert!(tables.contains(&"direct_messages".to_string()));
        assert!(tables.contains(&"user_configs".to_string()));
    }

    #[test]
    fn test_seed_test_data() {
        let db = Database::in_memory().expect("Failed to create database");
        db.initialize().expect("Failed to initialize schema");
        db.seed_test_data().expect("Failed to seed test data");

        // Verify test users exist
        let conn = db.connection().expect("Failed to get connection");
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE is_test_user = 1",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count test users");

        assert_eq!(count, 8);
    }

    #[test]
    fn test_memory_database_detection() {
        // Test various memory database path formats
        let memory_paths = [":memory:", " :memory: ", ":MEMORY:", " :Memory: "];

        for path in &memory_paths {
            let db = Database::new(path).expect("Failed to create memory database");
            db.initialize().expect("Failed to initialize schema");

            // Verify it's actually in memory by checking we can create multiple instances
            let db2 = Database::new(path).expect("Failed to create second memory database");
            db2.initialize()
                .expect("Failed to initialize second schema");
        }

        // Test file database path
        let temp_path = "/tmp/test_fido.db";
        let db = Database::new(temp_path).expect("Failed to create file database");
        db.initialize().expect("Failed to initialize file schema");

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_authentication_migrations() {
        let db = Database::in_memory().expect("Failed to create database");
        db.initialize().expect("Failed to initialize schema");

        let conn = db.connection().expect("Failed to get connection");

        // Verify sessions table exists
        let sessions_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sessions'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to check sessions table");
        assert_eq!(sessions_exists, 1, "sessions table should exist");

        // Verify sessions table has correct columns
        let mut stmt = conn
            .prepare("PRAGMA table_info(sessions)")
            .expect("Failed to prepare statement");
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get(1))
            .expect("Failed to query columns")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect columns");

        assert!(columns.contains(&"token".to_string()));
        assert!(columns.contains(&"user_id".to_string()));
        assert!(columns.contains(&"created_at".to_string()));
        assert!(columns.contains(&"expires_at".to_string()));

        // Verify indexes on sessions table
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='sessions'")
            .expect("Failed to prepare statement");
        let indexes: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .expect("Failed to query indexes")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect indexes");

        assert!(
            indexes.iter().any(|idx| idx.contains("user_id")),
            "Should have index on user_id"
        );
        assert!(
            indexes.iter().any(|idx| idx.contains("expires_at")),
            "Should have index on expires_at"
        );

        // Verify GitHub fields were added to users table
        let mut stmt = conn
            .prepare("PRAGMA table_info(users)")
            .expect("Failed to prepare statement");
        let user_columns: Vec<String> = stmt
            .query_map([], |row| row.get(1))
            .expect("Failed to query columns")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect columns");

        assert!(
            user_columns.contains(&"github_id".to_string()),
            "users table should have github_id column"
        );
        assert!(
            user_columns.contains(&"github_login".to_string()),
            "users table should have github_login column"
        );

        // Verify unique index on github_id
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='users'")
            .expect("Failed to prepare statement");
        let user_indexes: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .expect("Failed to query indexes")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect indexes");

        assert!(
            user_indexes.iter().any(|idx| idx.contains("github_id")),
            "Should have index on github_id"
        );
    }
}
