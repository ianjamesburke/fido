use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use fido_types::ActivityItem;
use uuid::Uuid;

use crate::api::{ApiError, ApiResult};
use crate::db::repositories::Repositories;
use crate::services::github::GithubService;

pub const ACTIVITY_CACHE_TTL_MINUTES: i64 = 10;

#[derive(Debug, Clone)]
pub struct CommunityActivity {
    pub items: Vec<ActivityItem>,
    pub fetched_at: DateTime<Utc>,
}

pub(crate) fn cache_is_fresh(fetched_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now - fetched_at < Duration::minutes(ACTIVITY_CACHE_TTL_MINUTES)
}

pub struct ActivityService {
    repos: Repositories,
    github: GithubService,
}

impl ActivityService {
    pub fn new(repos: Repositories, github: GithubService) -> Self {
        Self { repos, github }
    }

    /// Cached repo activity for a community. Fresh cache: served as-is.
    /// Stale/missing: refetch from GitHub and upsert. GitHub failure with a
    /// stale cache present: serve stale with a warning. Failure with no
    /// cache: propagate.
    pub async fn get_activity(
        &self,
        user_id: Uuid,
        community_id: Uuid,
    ) -> ApiResult<CommunityActivity> {
        let now = Utc::now();
        let cached = self.repos.activity.get(community_id)?;

        if let Some(record) = &cached {
            if cache_is_fresh(record.fetched_at, now) {
                return Ok(decode(record.payload.as_str(), record.fetched_at)?);
            }
        }

        let community = self
            .repos
            .communities
            .get_by_id(&community_id)?
            .ok_or_else(|| ApiError::NotFound("Community not found".to_string()))?;

        match self
            .github
            .repo_activity(user_id, &community.owner, &community.name)
            .await
        {
            Ok(items) => {
                let payload = serde_json::to_string(&items)
                    .context("Failed to serialize activity payload")?;
                self.repos.activity.upsert(community_id, &payload, now)?;
                Ok(CommunityActivity {
                    items,
                    fetched_at: now,
                })
            }
            Err(error) => {
                if let Some(record) = cached {
                    tracing::warn!(%community_id, %error, "Serving stale activity cache after GitHub fetch failure");
                    return Ok(decode(record.payload.as_str(), record.fetched_at)?);
                }
                Err(error)
                    .with_context(|| {
                        format!("Failed to fetch repo activity for community {}", community_id)
                    })
                    .map_err(ApiError::from)
            }
        }
    }
}

fn decode(payload: &str, fetched_at: DateTime<Utc>) -> Result<CommunityActivity> {
    let items: Vec<ActivityItem> =
        serde_json::from_str(payload).context("Failed to decode cached activity payload")?;
    Ok(CommunityActivity { items, fetched_at })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn cache_is_fresh_within_ttl_and_stale_after() {
        let now = Utc::now();
        assert!(cache_is_fresh(now - Duration::minutes(9), now));
        assert!(!cache_is_fresh(now - Duration::minutes(10), now));
        assert!(!cache_is_fresh(now - Duration::hours(2), now));
    }
}
