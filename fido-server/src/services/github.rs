use std::env;

use aes_gcm_siv::{
    aead::{Aead, KeyInit},
    Aes256GcmSiv, Nonce,
};
use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{Duration, Utc};
use fido_types::{ActivityItem, ActivityKind, ActivityState};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::repositories::Repositories;

const GITHUB_API_ACCEPT: &str = "application/vnd.github+json";
const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_USER_AGENT: &str = "Fido-Social";
const NONCE_SIZE: usize = 12;

#[derive(Clone)]
pub struct GithubService {
    repos: Repositories,
    client: Client,
    cipher: TokenCipher,
    api_base: String,
    oauth_client_id: Option<String>,
    oauth_client_secret: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GithubRepo {
    pub id: i64,
    pub owner: GithubRepoOwner,
    pub name: String,
    pub full_name: String,
    pub private: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GithubRepoOwner {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RepoPermission {
    pub admin: bool,
    pub maintain: bool,
    pub push: bool,
    pub triage: bool,
    pub pull: bool,
}

#[derive(Debug, Deserialize)]
struct GithubRepoWithPermissions {
    permissions: Option<RepoPermission>,
}

#[derive(Debug, Deserialize)]
struct GithubContributor {
    login: String,
}

#[derive(Debug, Serialize)]
struct RevokeGrantRequest<'a> {
    access_token: &'a str,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GithubIssue {
    id: i64,
    number: i64,
    title: String,
    state: String,
    html_url: String,
    created_at: chrono::DateTime<Utc>,
    user: GithubIssueUser,
    pull_request: Option<GithubIssuePullRequest>,
}

#[derive(Debug, Deserialize)]
struct GithubIssueUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GithubIssuePullRequest {
    merged_at: Option<chrono::DateTime<Utc>>,
}

pub(crate) fn map_issue_to_activity(issue: GithubIssue) -> ActivityItem {
    let (kind, state) = match &issue.pull_request {
        Some(pr) if pr.merged_at.is_some() => (ActivityKind::PullRequest, ActivityState::Merged),
        Some(_) => (
            ActivityKind::PullRequest,
            if issue.state == "open" {
                ActivityState::Open
            } else {
                ActivityState::Closed
            },
        ),
        None => (
            ActivityKind::Issue,
            if issue.state == "open" {
                ActivityState::Open
            } else {
                ActivityState::Closed
            },
        ),
    };
    ActivityItem {
        github_id: issue.id,
        kind,
        number: issue.number,
        title: issue.title,
        author_login: issue.user.login,
        state,
        created_at: issue.created_at,
        html_url: issue.html_url,
    }
}

#[derive(Clone)]
struct TokenCipher {
    cipher: Aes256GcmSiv,
}

impl GithubService {
    pub fn from_env(repos: Repositories) -> Result<Self> {
        let cipher = TokenCipher::from_env()?;
        let client = Client::builder()
            .build()
            .context("Failed to build GitHub HTTP client")?;

        Ok(Self {
            repos,
            client,
            cipher,
            api_base: env::var("GITHUB_API_BASE")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| GITHUB_API_BASE.to_string()),
            oauth_client_id: env::var("GITHUB_CLIENT_ID")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            oauth_client_secret: env::var("GITHUB_CLIENT_SECRET")
                .ok()
                .filter(|v| !v.trim().is_empty()),
        })
    }

    pub fn store_token(&self, user_id: Uuid, access_token: &str) -> Result<()> {
        let encrypted_token = self
            .cipher
            .encrypt(access_token)
            .with_context(|| format!("Failed to encrypt GitHub token for user {}", user_id))?;
        self.repos
            .github_tokens
            .upsert_token(user_id, &encrypted_token, Utc::now())
            .with_context(|| format!("Failed to store GitHub token for user {}", user_id))
    }

    pub async fn starred_repos(&self, user_id: Uuid) -> Result<Vec<GithubRepo>> {
        let result = self
            .get_with_token(
                user_id,
                format!("{}/user/starred", self.api_base),
                "starred_repos",
            )
            .await;
        self.log_result("starred_repos", user_id, Some("GET /user/starred"), &result);
        result
    }

    /// Resolve a repo by owner/name. Uses the caller's stored token when present
    /// (required for private repos), otherwise queries unauthenticated.
    /// Returns Ok(None) when GitHub reports the repo does not exist (404).
    pub async fn get_repo(
        &self,
        user_id: Uuid,
        owner: &str,
        name: &str,
    ) -> Result<Option<GithubRepo>> {
        let url = format!("{}/repos/{}/{}", self.api_base, owner, name);
        let token = self.load_token(user_id)?;

        let mut request = self
            .client
            .get(&url)
            .header("User-Agent", GITHUB_USER_AGENT)
            .header("Accept", GITHUB_API_ACCEPT);
        if let Some(token) = token.as_deref() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let result = async {
            let response = request.send().await.with_context(|| {
                format!("GitHub get_repo request failed for {}/{}", owner, name)
            })?;
            if response.status() == StatusCode::NOT_FOUND {
                return Ok(None);
            }
            let repo = response
                .error_for_status()
                .with_context(|| {
                    format!(
                        "GitHub get_repo returned non-success status for {}/{}",
                        owner, name
                    )
                })?
                .json::<GithubRepo>()
                .await
                .with_context(|| {
                    format!(
                        "Failed to parse GitHub repo response for {}/{}",
                        owner, name
                    )
                })?;
            Ok(Some(repo))
        }
        .await;

        let op = format!("GET /repos/{}/{}", owner, name);
        self.log_result("get_repo", user_id, Some(&op), &result);
        result
    }

    pub async fn repo_permission(
        &self,
        user_id: Uuid,
        owner: &str,
        name: &str,
    ) -> Result<RepoPermission> {
        let url = format!("{}/repos/{}/{}", self.api_base, owner, name);
        let result = self
            .get_with_token::<GithubRepoWithPermissions>(user_id, url, "repo_permission")
            .await
            .and_then(|repo| {
                repo.permissions.ok_or_else(|| {
                    anyhow!(
                        "GitHub repo permission response missing permissions object for {}/{}",
                        owner,
                        name
                    )
                })
            });

        let op = format!("GET /repos/{}/{}", owner, name);
        self.log_result("repo_permission", user_id, Some(&op), &result);
        result
    }

    pub async fn is_contributor(&self, user_id: Uuid, owner: &str, name: &str) -> Result<bool> {
        let user = self
            .repos
            .users
            .get_by_id(&user_id)
            .with_context(|| format!("Failed to load user {}", user_id))?
            .ok_or_else(|| anyhow!("User {} not found", user_id))?;
        let github_login = user.username;
        let url = format!("{}/repos/{}/{}/contributors", self.api_base, owner, name);
        let result = self
            .get_public::<Vec<GithubContributor>>(url, "is_contributor")
            .await
            .map(|contributors| contributors.iter().any(|c| c.login == github_login));

        let op = format!("GET /repos/{}/{}/contributors", owner, name);
        self.log_result("is_contributor", user_id, Some(&op), &result);
        result
    }

    pub async fn revoke_and_delete_token(&self, user_id: Uuid) -> Result<()> {
        let maybe_token = self.load_token(user_id)?;
        if let Some(token) = maybe_token.as_deref() {
            self.try_revoke_remote_token(token).await;
        }

        self.repos
            .github_tokens
            .delete_token(user_id)
            .with_context(|| {
                format!("Failed to delete stored GitHub token for user {}", user_id)
            })?;
        Ok(())
    }

    /// Fetch issues + PRs updated in the last 14 days (one request; GitHub's
    /// issues endpoint includes PRs). Uses the caller's stored token when
    /// present, unauthenticated otherwise.
    pub async fn repo_activity(
        &self,
        user_id: Uuid,
        owner: &str,
        name: &str,
    ) -> Result<Vec<ActivityItem>> {
        let since = (Utc::now() - Duration::days(14)).to_rfc3339();
        let url = format!(
            "{}/repos/{}/{}/issues?state=all&since={}&per_page=100&sort=created&direction=desc",
            self.api_base, owner, name, since
        );

        let result = match self.load_token(user_id)? {
            Some(_) => {
                self.get_with_token::<Vec<GithubIssue>>(user_id, url, "repo_activity")
                    .await
            }
            None => self.get_public::<Vec<GithubIssue>>(url, "repo_activity").await,
        }
        .map(|issues| issues.into_iter().map(map_issue_to_activity).collect());

        let op = format!("GET /repos/{}/{}/issues", owner, name);
        self.log_result("repo_activity", user_id, Some(&op), &result);
        result
    }

    async fn get_with_token<T>(&self, user_id: Uuid, url: String, operation: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let token = self
            .load_token(user_id)?
            .ok_or_else(|| anyhow!("No stored GitHub token for user {}", user_id))?;

        self.client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", GITHUB_USER_AGENT)
            .header("Accept", GITHUB_API_ACCEPT)
            .send()
            .await
            .with_context(|| format!("GitHub {} request failed", operation))?
            .error_for_status()
            .with_context(|| format!("GitHub {} returned non-success status", operation))?
            .json::<T>()
            .await
            .with_context(|| format!("Failed to parse GitHub {} response", operation))
    }

    async fn get_public<T>(&self, url: String, operation: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.client
            .get(url)
            .header("User-Agent", GITHUB_USER_AGENT)
            .header("Accept", GITHUB_API_ACCEPT)
            .send()
            .await
            .with_context(|| format!("GitHub {} request failed", operation))?
            .error_for_status()
            .with_context(|| format!("GitHub {} returned non-success status", operation))?
            .json::<T>()
            .await
            .with_context(|| format!("Failed to parse GitHub {} response", operation))
    }

    fn load_token(&self, user_id: Uuid) -> Result<Option<String>> {
        self.repos
            .github_tokens
            .get_token(user_id)?
            .map(|record| self.cipher.decrypt(&record.encrypted_token))
            .transpose()
            .with_context(|| format!("Failed to decrypt GitHub token for user {}", user_id))
    }

    async fn try_revoke_remote_token(&self, token: &str) {
        let Some(client_id) = self.oauth_client_id.as_deref() else {
            tracing::debug!("Skipping GitHub token revocation: GITHUB_CLIENT_ID not configured");
            return;
        };
        let Some(client_secret) = self.oauth_client_secret.as_deref() else {
            tracing::debug!(
                "Skipping GitHub token revocation: GITHUB_CLIENT_SECRET not configured"
            );
            return;
        };

        let revoke_url = format!("{}/applications/{}/grant", self.api_base, client_id);
        let response = self
            .client
            .delete(revoke_url)
            .basic_auth(client_id, Some(client_secret))
            .header("User-Agent", GITHUB_USER_AGENT)
            .header("Accept", GITHUB_API_ACCEPT)
            .json(&RevokeGrantRequest {
                access_token: token,
            })
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() || resp.status() == StatusCode::NOT_FOUND => {}
            Ok(resp) => {
                tracing::warn!(
                    status = %resp.status(),
                    "Best-effort GitHub token revocation returned non-success status"
                );
            }
            Err(error) => {
                tracing::warn!(%error, "Best-effort GitHub token revocation failed");
            }
        }
    }

    fn log_result<T>(
        &self,
        operation: &str,
        user_id: Uuid,
        inputs: Option<&str>,
        result: &Result<T>,
    ) {
        if let Err(error) = result {
            tracing::warn!(
                operation,
                %user_id,
                inputs = inputs.unwrap_or(""),
                %error,
                "GitHub service operation failed"
            );
        }
    }
}

/// Validate that `FIDO_TOKEN_KEY` is present and correctly formed (base64 of
/// exactly 32 bytes) without logging the key. Called at startup to fail fast
/// instead of only failing lazily on the first OAuth token operation.
pub fn validate_token_key() -> Result<()> {
    TokenCipher::from_env().map(|_| ())
}

impl TokenCipher {
    fn from_env() -> Result<Self> {
        let key_material =
            env::var("FIDO_TOKEN_KEY").context("FIDO_TOKEN_KEY environment variable not set")?;
        let key_bytes = STANDARD
            .decode(key_material.trim())
            .context("FIDO_TOKEN_KEY must be base64-encoded")?;
        if key_bytes.len() != 32 {
            bail!(
                "FIDO_TOKEN_KEY must decode to exactly 32 bytes, got {}",
                key_bytes.len()
            );
        }

        let cipher = Aes256GcmSiv::new_from_slice(&key_bytes)
            .map_err(|_| anyhow!("Invalid GitHub token cipher key"))?;
        Ok(Self { cipher })
    }

    fn encrypt(&self, plaintext: &str) -> Result<String> {
        let nonce_source = Uuid::new_v4();
        let nonce_bytes = &nonce_source.as_bytes()[..NONCE_SIZE];
        let ciphertext = self
            .cipher
            .encrypt(Nonce::from_slice(nonce_bytes), plaintext.as_bytes())
            .map_err(|_| anyhow!("Token encryption failed"))?;

        let mut payload = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        payload.extend_from_slice(nonce_bytes);
        payload.extend_from_slice(&ciphertext);
        Ok(STANDARD.encode(payload))
    }

    fn decrypt(&self, encoded: &str) -> Result<String> {
        let payload = STANDARD
            .decode(encoded)
            .context("Stored GitHub token payload is not valid base64")?;
        if payload.len() <= NONCE_SIZE {
            bail!("Stored GitHub token payload is truncated");
        }

        let (nonce_bytes, ciphertext) = payload.split_at(NONCE_SIZE);
        let plaintext = self
            .cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
            .map_err(|_| anyhow!("Token decryption failed"))?;
        String::from_utf8(plaintext).context("Stored GitHub token is not valid UTF-8")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_issue_pr_and_merged_pr() {
        let raw = include_str!("../../tests/fixtures/github_issues_sample.json");
        let issues: Vec<GithubIssue> = serde_json::from_str(raw).expect("fixture parses");
        let items: Vec<ActivityItem> = issues.into_iter().map(map_issue_to_activity).collect();

        let issue = items
            .iter()
            .find(|i| i.kind == ActivityKind::Issue)
            .expect("has an issue");
        assert_eq!(issue.number, 10);
        assert_eq!(issue.title, "Give the website a makeover");
        assert_eq!(issue.state, ActivityState::Open);
        assert_eq!(issue.author_login, "ianjamesburke");

        let merged = items
            .iter()
            .find(|i| i.state == ActivityState::Merged)
            .expect("has a merged PR");
        assert_eq!(merged.kind, ActivityKind::PullRequest);
        assert_eq!(merged.number, 19);
        assert!(merged.html_url.starts_with("https://"));
        assert!(!merged.author_login.is_empty());

        let open_pr = items
            .iter()
            .find(|i| i.kind == ActivityKind::PullRequest && i.state == ActivityState::Open)
            .expect("has an open PR");
        assert_eq!(open_pr.number, 158719);
        assert_eq!(open_pr.author_login, "jieyouxu");
    }

    #[test]
    fn token_cipher_round_trip() {
        let _guard = crate::test_utils::env_lock().lock().unwrap();
        std::env::set_var("FIDO_TOKEN_KEY", STANDARD.encode([7u8; 32]));

        let cipher = TokenCipher::from_env().expect("cipher config");
        let encrypted = cipher.encrypt("gho_example_token").expect("encrypt");
        assert_ne!(encrypted, "gho_example_token");

        let decrypted = cipher.decrypt(&encrypted).expect("decrypt");
        assert_eq!(decrypted, "gho_example_token");
    }
}
