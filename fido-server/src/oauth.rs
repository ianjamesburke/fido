use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

/// GitHub OAuth configuration
#[derive(Debug, Clone)]
pub struct GitHubOAuthConfig {
    client_id: String,
}

/// GitHub user information from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

/// GitHub Device Flow - Device code response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: i64,
    pub interval: i64,
}

/// GitHub Device Flow - Access token response
#[derive(Debug, Deserialize)]
struct DeviceTokenResponse {
    access_token: Option<String>,
    #[allow(dead_code)]
    token_type: Option<String>,
    #[allow(dead_code)]
    scope: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

impl GitHubOAuthConfig {
    /// Load OAuth configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let client_id = env::var("GITHUB_CLIENT_ID")
            .context("GITHUB_CLIENT_ID environment variable not set")?;
        
        Ok(Self {
            client_id,
        })
    }

    /// Request a device code from GitHub (Device Flow step 1)
    pub async fn request_device_code(&self) -> Result<DeviceCodeResponse> {
        let client = reqwest::Client::new();
        
        let params = [
            ("client_id", self.client_id.as_str()),
            ("scope", "user:email"),
        ];
        
        let response = client
            .post("https://github.com/login/device/code")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .context("Failed to request device code from GitHub")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "GitHub device code request failed with status {}: {}",
                status,
                body
            ));
        }
        
        let device_code_response: DeviceCodeResponse = response
            .json()
            .await
            .context("Failed to parse GitHub device code response")?;
        
        Ok(device_code_response)
    }

    /// Poll for access token (Device Flow step 2)
    /// Returns Ok(Some(token)) if authorized, Ok(None) if still pending, Err if failed
    pub async fn poll_device_token(&self, device_code: &str) -> Result<Option<String>> {
        let client = reqwest::Client::new();
        
        let params = [
            ("client_id", self.client_id.as_str()),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ];
        
        let response = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .context("Failed to poll for device token")?;
        
        let token_response: DeviceTokenResponse = response
            .json()
            .await
            .context("Failed to parse GitHub token response")?;
        
        // Check for errors
        if let Some(error) = token_response.error {
            match error.as_str() {
                "authorization_pending" => {
                    // User hasn't authorized yet, continue polling
                    return Ok(None);
                }
                "slow_down" => {
                    // We're polling too fast, but continue
                    return Ok(None);
                }
                "expired_token" => {
                    return Err(anyhow!("Device code expired"));
                }
                "access_denied" => {
                    return Err(anyhow!("User denied authorization"));
                }
                _ => {
                    return Err(anyhow!(
                        "GitHub authorization error: {} - {}",
                        error,
                        token_response.error_description.unwrap_or_default()
                    ));
                }
            }
        }
        
        // Success - return the access token
        Ok(token_response.access_token)
    }

    /// Fetch GitHub user profile using access token
    pub async fn get_user(&self, access_token: String) -> Result<GitHubUser> {
        let client = reqwest::Client::new();
        
        let response = client
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "Fido-Social")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .context("Failed to send user profile request to GitHub")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "GitHub user profile fetch failed with status {}: {}",
                status,
                body
            ));
        }
        
        let user: GitHubUser = response
            .json()
            .await
            .context("Failed to parse GitHub user response")?;
        
        Ok(user)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_config_from_env() {
        std::env::set_var("GITHUB_CLIENT_ID", "test_client_id");
        
        let config = GitHubOAuthConfig::from_env().unwrap();
        assert_eq!(config.client_id, "test_client_id");
        
        std::env::remove_var("GITHUB_CLIENT_ID");
    }
}
