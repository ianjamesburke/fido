//! Community-related business logic.

use chrono::Utc;
use uuid::Uuid;

use crate::api::{ApiError, ApiResult};
use crate::db::repositories::Repositories;
use crate::services::github::{GithubRepo, GithubService, RepoPermission};
use fido_types::{Channel, Community, Membership, MembershipRole};

pub struct CommunityService {
    repos: Repositories,
    github: GithubService,
}

#[derive(Debug, Clone)]
pub struct BrowseCommunity {
    pub github_repo_id: i64,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub community: Option<Community>,
    pub membership: Option<Membership>,
}

#[derive(Debug, Clone)]
pub struct CommunityView {
    pub community: Community,
    pub membership: Option<Membership>,
    pub member_count: i64,
    pub channels: Vec<Channel>,
}

impl CommunityService {
    pub fn new(repos: Repositories, github: GithubService) -> Self {
        Self { repos, github }
    }

    pub async fn browse_starred(&self, user_id: Uuid) -> ApiResult<Vec<BrowseCommunity>> {
        let starred = self.github.starred_repos(user_id).await?;
        starred
            .into_iter()
            .map(|repo| self.annotate_starred_repo(repo, &user_id))
            .collect()
    }

    pub async fn join(
        &self,
        user_id: Uuid,
        github_repo_id: i64,
        owner: &str,
        name: &str,
    ) -> ApiResult<CommunityView> {
        let community = match self
            .repos
            .communities
            .get_by_github_repo_id(github_repo_id)?
        {
            Some(community) => community,
            None => self.create_community(github_repo_id, owner, name)?,
        };

        let role = if self
            .github
            .is_contributor(user_id, &community.owner, &community.name)
            .await
            .unwrap_or(false)
        {
            MembershipRole::Contributor
        } else {
            MembershipRole::Member
        };

        let membership = Membership {
            community_id: community.id,
            user_id,
            role,
            created_at: Utc::now(),
        };
        self.repos.memberships.insert_if_missing(&membership)?;

        self.get_view(user_id, community.id)
    }

    pub fn list_joined(&self, user_id: Uuid) -> ApiResult<Vec<CommunityView>> {
        self.repos
            .communities
            .list_joined(&user_id)?
            .into_iter()
            .map(|community| self.view_for_community(user_id, community))
            .collect()
    }

    pub fn get_view(&self, user_id: Uuid, community_id: Uuid) -> ApiResult<CommunityView> {
        let community = self
            .repos
            .communities
            .get_by_id(&community_id)?
            .ok_or_else(|| ApiError::NotFound("Community not found".to_string()))?;
        self.view_for_community(user_id, community)
    }

    pub async fn claim(&self, user_id: Uuid, community_id: Uuid) -> ApiResult<CommunityView> {
        let community = self
            .repos
            .communities
            .get_by_id(&community_id)?
            .ok_or_else(|| ApiError::NotFound("Community not found".to_string()))?;

        let permission = self
            .github
            .repo_permission(user_id, &community.owner, &community.name)
            .await?;
        if !can_claim(&permission) {
            return Err(ApiError::Forbidden(
                "GitHub admin or maintain permission required".to_string(),
            ));
        }

        self.repos
            .communities
            .set_claimed_by(&community.id, &user_id)?;
        self.repos.memberships.insert_if_missing(&Membership {
            community_id: community.id,
            user_id,
            role: MembershipRole::Admin,
            created_at: Utc::now(),
        })?;
        self.repos
            .memberships
            .update_role(&community.id, &user_id, MembershipRole::Admin)?;

        self.get_view(user_id, community.id)
    }

    pub fn leave(&self, user_id: Uuid, community_id: Uuid) -> ApiResult<()> {
        self.repos
            .communities
            .get_by_id(&community_id)?
            .ok_or_else(|| ApiError::NotFound("Community not found".to_string()))?;
        self.repos.memberships.delete(&community_id, &user_id)?;
        Ok(())
    }

    #[allow(dead_code)] // Reused by downstream moderation endpoints when those routes land.
    pub fn require_role(
        &self,
        user_id: Uuid,
        community_id: Uuid,
        allowed_roles: &[MembershipRole],
    ) -> ApiResult<Membership> {
        let membership = self
            .repos
            .memberships
            .get(&community_id, &user_id)?
            .ok_or_else(|| ApiError::Forbidden("Community membership required".to_string()))?;
        if !allowed_roles.contains(&membership.role) {
            return Err(ApiError::Forbidden(
                "Insufficient community role".to_string(),
            ));
        }
        Ok(membership)
    }

    fn annotate_starred_repo(
        &self,
        repo: GithubRepo,
        user_id: &Uuid,
    ) -> ApiResult<BrowseCommunity> {
        let community = self.repos.communities.get_by_github_repo_id(repo.id)?;
        let membership = community
            .as_ref()
            .map(|community| self.repos.memberships.get(&community.id, user_id))
            .transpose()?
            .flatten();

        Ok(BrowseCommunity {
            github_repo_id: repo.id,
            owner: repo.owner.login,
            name: repo.name,
            full_name: repo.full_name,
            private: repo.private,
            community,
            membership,
        })
    }

    fn create_community(
        &self,
        github_repo_id: i64,
        owner: &str,
        name: &str,
    ) -> ApiResult<Community> {
        let community = Community {
            id: Uuid::new_v4(),
            github_repo_id,
            owner: owner.to_string(),
            name: name.to_string(),
            claimed_by: None,
            require_thread_approval: false,
            created_at: Utc::now(),
        };
        self.repos.communities.create(&community)?;
        self.repos.channels.create(&Channel {
            id: Uuid::new_v4(),
            community_id: community.id,
            name: "general".to_string(),
            created_at: Utc::now(),
        })?;
        Ok(community)
    }

    fn view_for_community(&self, user_id: Uuid, community: Community) -> ApiResult<CommunityView> {
        Ok(CommunityView {
            membership: self.repos.memberships.get(&community.id, &user_id)?,
            member_count: self.repos.communities.member_count(&community.id)?,
            channels: self.repos.channels.list_by_community(&community.id)?,
            community,
        })
    }
}

fn can_claim(permission: &RepoPermission) -> bool {
    permission.admin || permission.maintain
}

#[cfg(all(test, feature = "sqlite-tests"))]
mod tests {
    use super::*;
    use axum::{routing::get, Json, Router};
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use serde_json::json;

    use crate::db::Database;

    async fn fixture_server() -> String {
        async fn starred() -> Json<serde_json::Value> {
            Json(json!([
                {
                    "id": 1296269,
                    "name": "Hello-World",
                    "full_name": "octocat/Hello-World",
                    "private": false,
                    "owner": { "login": "octocat" }
                }
            ]))
        }

        async fn repo() -> Json<serde_json::Value> {
            Json(json!({
                "id": 1296269,
                "name": "Hello-World",
                "full_name": "octocat/Hello-World",
                "private": false,
                "owner": { "login": "octocat" },
                "permissions": {
                    "admin": false,
                    "maintain": true,
                    "push": true,
                    "triage": true,
                    "pull": true
                }
            }))
        }

        async fn contributors() -> Json<serde_json::Value> {
            Json(json!([
                { "login": "alice", "id": 1 },
                { "login": "octocat", "id": 583231 }
            ]))
        }

        let app = Router::new()
            .route("/user/starred", get(starred))
            .route("/repos/octocat/Hello-World", get(repo))
            .route("/repos/octocat/Hello-World/contributors", get(contributors));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fixture listener");
        let addr = listener.local_addr().expect("fixture addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("fixture server");
        });
        format!("http://{}", addr)
    }

    fn setup_user(repos: &Repositories) -> Uuid {
        let user_id = Uuid::new_v4();
        repos
            .users
            .create(&fido_types::User {
                id: user_id,
                username: "alice".to_string(),
                bio: None,
                join_date: Utc::now(),
                is_test_user: true,
                is_admin: false,
            })
            .expect("create user");
        user_id
    }

    #[tokio::test]
    async fn browse_join_and_claim_use_recorded_github_shapes() {
        let api_base = fixture_server().await;
        let db = Database::in_memory().expect("db");
        db.initialize().expect("schema");
        let repos = Repositories::new(db.pool.clone());
        let user_id = setup_user(&repos);
        let service = {
            let _guard = crate::test_utils::env_lock().lock().unwrap();
            std::env::set_var("FIDO_TOKEN_KEY", STANDARD.encode([9u8; 32]));
            std::env::set_var("GITHUB_API_BASE", api_base);
            let github = GithubService::from_env(repos.clone()).expect("github service");
            std::env::remove_var("GITHUB_API_BASE");
            std::env::remove_var("FIDO_TOKEN_KEY");

            github
                .store_token(user_id, "gho_recorded_fixture")
                .expect("store token");
            CommunityService::new(repos.clone(), github)
        };

        let browse = service.browse_starred(user_id).await.expect("browse");
        assert_eq!(browse.len(), 1);
        assert_eq!(browse[0].full_name, "octocat/Hello-World");
        assert!(browse[0].community.is_none());
        assert!(browse[0].membership.is_none());

        let joined = service
            .join(user_id, 1296269, "octocat", "Hello-World")
            .await
            .expect("join");
        assert_eq!(joined.community.github_repo_id, 1296269);
        assert_eq!(joined.channels.len(), 1);
        assert_eq!(joined.channels[0].name, "general");
        assert_eq!(
            joined.membership.as_ref().map(|m| m.role),
            Some(MembershipRole::Contributor)
        );

        let browse = service
            .browse_starred(user_id)
            .await
            .expect("browse after join");
        assert!(browse[0].community.is_some());
        assert_eq!(
            browse[0].membership.as_ref().map(|m| m.role),
            Some(MembershipRole::Contributor)
        );

        let claimed = service
            .claim(user_id, joined.community.id)
            .await
            .expect("claim");
        assert_eq!(claimed.community.claimed_by, Some(user_id));
        assert_eq!(
            claimed.membership.as_ref().map(|m| m.role),
            Some(MembershipRole::Admin)
        );
    }
}
