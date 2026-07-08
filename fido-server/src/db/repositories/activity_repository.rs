use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use crate::db::DbPool;

#[derive(Debug, Clone)]
pub struct ActivityCacheRecord {
    pub payload: String,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct ActivityRepository {
    pool: DbPool,
}

impl ActivityRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn get(&self, community_id: Uuid) -> Result<Option<ActivityCacheRecord>> {
        let conn = self.pool.get()?;
        let row: Option<(String, String)> = conn
            .query_row(
                "SELECT payload, fetched_at FROM community_activity WHERE community_id = ?1",
                rusqlite::params![community_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        match row {
            Some((payload, fetched_at)) => Ok(Some(ActivityCacheRecord {
                payload,
                fetched_at: DateTime::parse_from_rfc3339(&fetched_at)?.with_timezone(&Utc),
            })),
            None => Ok(None),
        }
    }

    pub fn upsert(
        &self,
        community_id: Uuid,
        payload: &str,
        fetched_at: DateTime<Utc>,
    ) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO community_activity (community_id, payload, fetched_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(community_id) DO UPDATE SET
                payload = excluded.payload,
                fetched_at = excluded.fetched_at",
            rusqlite::params![community_id.to_string(), payload, fetched_at.to_rfc3339()],
        )?;
        Ok(())
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
            "INSERT INTO communities (id, github_repo_id, owner, name, claimed_by, require_thread_approval, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "650e8400-e29b-41d4-a716-446655440001",
                1i64,
                "octocat",
                "hello",
                Option::<String>::None,
                0,
                Utc::now().to_rfc3339(),
            ],
        )
        .expect("Failed to insert test community");

        db
    }

    #[test]
    fn upsert_and_get_activity_cache() {
        let db = setup_db();
        let repo = ActivityRepository::new(db.pool.clone());
        let community_id = Uuid::parse_str("650e8400-e29b-41d4-a716-446655440001").unwrap();
        let now = Utc::now();

        assert!(repo.get(community_id).unwrap().is_none());

        repo.upsert(community_id, r#"[{"fake":"payload"}]"#, now)
            .unwrap();
        let rec = repo.get(community_id).unwrap().unwrap();
        assert_eq!(rec.payload, r#"[{"fake":"payload"}]"#);
        assert_eq!(rec.fetched_at, now);

        let later = now + chrono::Duration::minutes(11);
        repo.upsert(community_id, "[]", later).unwrap();
        let rec = repo.get(community_id).unwrap().unwrap();
        assert_eq!(rec.payload, "[]");
        assert_eq!(rec.fetched_at, later);
    }
}
