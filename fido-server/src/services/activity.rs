use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use fido_types::{Membership, MembershipRole, Post};
use uuid::Uuid;

use crate::db::repositories::Repositories;
use crate::services::github::GithubService;

pub const ACTIVITY_SYNC_TTL_MINUTES: i64 = 10;
const GITHUB_SYSTEM_USERNAME: &str = "github";

pub(crate) fn sync_is_fresh(last_synced_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now - last_synced_at < Duration::minutes(ACTIVITY_SYNC_TTL_MINUTES)
}

pub struct ActivityService {
    repos: Repositories,
    github: GithubService,
}

impl ActivityService {
    pub fn new(repos: Repositories, github: GithubService) -> Self {
        Self { repos, github }
    }

    /// Best-effort sync: failure is logged so existing posts remain readable.
    pub async fn sync_activity(&self, community_id: Uuid) -> Result<()> {
        let now = Utc::now();
        if let Some(last_synced_at) = self.repos.activity.get_last_synced_at(community_id)? {
            if sync_is_fresh(last_synced_at, now) {
                return Ok(());
            }
        }

        if let Err(error) = self.try_sync(community_id, now).await {
            tracing::warn!(%community_id, %error, "GitHub activity sync failed; serving existing posts");
        }
        Ok(())
    }

    async fn try_sync(&self, community_id: Uuid, now: DateTime<Utc>) -> Result<()> {
        let community = self
            .repos
            .communities
            .get_by_id(&community_id)?
            .ok_or_else(|| anyhow!("Community {} not found", community_id))?;
        let system_user = self
            .repos
            .users
            .get_or_create_system_user(GITHUB_SYSTEM_USERNAME)
            .context("Failed to get or create github system user")?;
        let items = self
            .github
            .repo_activity(system_user.id, &community.owner, &community.name)
            .await
            .context("Failed to fetch repo activity from GitHub")?;

        if !items.is_empty() {
            self.repos.memberships.insert_if_missing(&Membership {
                community_id,
                user_id: system_user.id,
                role: MembershipRole::Member,
                created_at: now,
            })?;
        }

        for item in items {
            let mut content = item.title;
            content.truncate(280);
            // On a conflict the repository deliberately preserves the existing
            // post id, retaining all native votes and replies.
            self.repos.posts.upsert_github_post(&Post {
                id: Uuid::new_v4(),
                author_id: system_user.id,
                author_username: system_user.username.clone(),
                community_id,
                content,
                created_at: item.created_at,
                upvotes: 0,
                downvotes: 0,
                approved: true,
                hashtags: Vec::new(),
                user_vote: None,
                parent_post_id: None,
                reply_count: 0,
                reply_to_user_id: None,
                reply_to_username: None,
                github_id: Some(item.github_id),
                github_kind: Some(item.kind),
                github_state: Some(item.state),
                github_html_url: Some(item.html_url),
            })?;
        }

        self.repos.activity.mark_synced(community_id, now)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_is_fresh_within_ttl_and_stale_after() {
        let now = Utc::now();
        assert!(sync_is_fresh(now - Duration::minutes(9), now));
        assert!(!sync_is_fresh(now - Duration::minutes(10), now));
    }
}
