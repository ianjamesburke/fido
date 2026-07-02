use anyhow::{Context, Result};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::path::Path;

use super::schema::{SCHEMA, TEST_DATA};

/// SQLite in-memory database identifier
const MEMORY_DB_PATH: &str = ":memory:";

/// Current schema generation, stamped into `PRAGMA user_version`.
/// v2 is the community rewrite (communities/channels/messages/notifications).
const SCHEMA_VERSION: i32 = 2;

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
        let path_str = path.as_ref().to_string_lossy();
        if !path_str.trim().eq_ignore_ascii_case(MEMORY_DB_PATH) {
            Self::archive_if_legacy(path.as_ref())?;
        }
        let manager = Self::create_connection_manager(&path)?;
        let pool = Pool::new(manager).context("Failed to create database connection pool")?;
        Ok(Self { pool })
    }

    /// The v2 rewrite ships a fresh schema with no migration path (design
    /// decision, stint 0003: no production data preserved). A pre-v2 database
    /// file cannot be initialized in place, so archive it aside and let a
    /// fresh v2 database be created at the original path.
    fn archive_if_legacy(path: &Path) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        let conn =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .with_context(|| {
                    format!("Failed to open existing database at {}", path.display())
                })?;

        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .context("Failed to read database user_version")?;
        if version >= SCHEMA_VERSION {
            return Ok(());
        }

        // user_version 0 is also what a brand-new empty file reports; only
        // treat the file as legacy if it actually contains a pre-v2 schema
        // (a posts table without community scoping, or hashtag tables).
        let posts_missing_community: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='posts')
                 AND NOT EXISTS(SELECT 1 FROM pragma_table_info('posts') WHERE name='community_id')",
                [],
                |row| row.get(0),
            )
            .context("Failed to inspect posts table shape")?;
        let has_hashtag_tables: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='hashtags')",
                [],
                |row| row.get(0),
            )
            .context("Failed to inspect hashtag tables")?;
        drop(conn);

        if !posts_missing_community && !has_hashtag_tables {
            return Ok(());
        }

        let backup = path.with_extension("v1.bak");
        if backup.exists() {
            anyhow::bail!(
                "Legacy v1 database found at {} but backup target {} already exists; move it aside manually",
                path.display(),
                backup.display()
            );
        }
        tracing::warn!(
            "Legacy v1 database detected at {}; archiving to {} and starting fresh (v2 has no migration path)",
            path.display(),
            backup.display()
        );
        std::fs::rename(path, &backup).with_context(|| {
            format!("Failed to archive legacy database to {}", backup.display())
        })?;
        for suffix in ["-wal", "-shm"] {
            let sidecar = std::path::PathBuf::from(format!("{}{}", path.display(), suffix));
            if sidecar.exists() {
                let sidecar_backup =
                    std::path::PathBuf::from(format!("{}{}", backup.display(), suffix));
                std::fs::rename(&sidecar, &sidecar_backup)
                    .with_context(|| format!("Failed to archive sidecar {}", sidecar.display()))?;
            }
        }
        Ok(())
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
                last_activity TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);",
        );

        // Migration: Add last_activity column to sessions table if it doesn't exist
        // Default value is set to created_at for existing sessions
        let _ = conn.execute("ALTER TABLE sessions ADD COLUMN last_activity TEXT", []);
        // Update existing sessions to set last_activity = created_at where NULL
        let _ = conn.execute(
            "UPDATE sessions SET last_activity = created_at WHERE last_activity IS NULL",
            [],
        );

        // Add GitHub authentication fields to users table
        let _ = conn.execute("ALTER TABLE users ADD COLUMN github_id INTEGER", []);
        let _ = conn.execute("ALTER TABLE users ADD COLUMN github_login TEXT", []);
        let _ = conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_users_github_id ON users(github_id)",
            [],
        );

        let _ = conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS github_tokens (
                user_id TEXT PRIMARY KEY,
                encrypted_token TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_github_tokens_updated_at ON github_tokens(updated_at);",
        );

        conn.pragma_update(None, "user_version", SCHEMA_VERSION)
            .context("Failed to stamp schema version")?;

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

#[cfg(all(test, feature = "sqlite-tests"))]
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
        assert!(tables.contains(&"communities".to_string()));
        assert!(tables.contains(&"channels".to_string()));
        assert!(tables.contains(&"messages".to_string()));
        assert!(tables.contains(&"memberships".to_string()));
        assert!(tables.contains(&"notifications".to_string()));
        assert!(tables.contains(&"dm_conversations".to_string()));
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
    fn test_legacy_v1_database_is_archived_and_rebuilt() {
        let dir = std::env::temp_dir().join(format!("fido-legacy-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let db_path = dir.join("fido.db");

        // Fabricate a v1 database: posts without community_id, plus hashtags.
        {
            let conn = rusqlite::Connection::open(&db_path).expect("open v1 db");
            conn.execute_batch(
                "CREATE TABLE users (id TEXT PRIMARY KEY, username TEXT);
                 CREATE TABLE posts (id TEXT PRIMARY KEY, author_id TEXT, content TEXT);
                 CREATE TABLE hashtags (id TEXT PRIMARY KEY, name TEXT);
                 INSERT INTO users VALUES ('u1', 'old-user');",
            )
            .expect("create v1 schema");
        }

        let db = Database::new(&db_path).expect("open archives legacy db");
        db.initialize().expect("fresh v2 schema initializes");

        let backup = db_path.with_extension("v1.bak");
        assert!(backup.exists(), "legacy file should be archived");

        let conn = db.connection().expect("connection");
        let has_community_id: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('posts') WHERE name='community_id')",
                [],
                |row| row.get(0),
            )
            .expect("inspect posts");
        assert!(has_community_id, "rebuilt posts table is v2-shaped");
        let old_users: i32 = conn
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
            .expect("count users");
        assert_eq!(old_users, 0, "fresh database has no v1 rows");
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("read version");
        assert_eq!(version, SCHEMA_VERSION);

        // Re-opening the rebuilt database must NOT archive it again.
        drop(conn);
        let db2 = Database::new(&db_path).expect("reopen v2 db");
        db2.initialize().expect("reinitialize is idempotent");
        assert!(
            !db_path.with_extension("v1.bak.v1.bak").exists(),
            "v2 database must never be re-archived"
        );

        let _ = std::fs::remove_dir_all(&dir);
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
        assert!(
            columns.contains(&"last_activity".to_string()),
            "sessions table should have last_activity column"
        );

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

        let github_tokens_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='github_tokens'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to check github_tokens table");
        assert_eq!(github_tokens_exists, 1, "github_tokens table should exist");
    }
}
