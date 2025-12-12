use anyhow::{Context, Result};
use fido_types::User;
use std::time::{Duration, Instant};

use crate::api::ApiClient;
use crate::session::SessionStore;

/// Manages the OAuth authentication flow for the TUI client.
///
/// This struct handles:
/// - Checking for existing sessions
/// - Initiating GitHub OAuth flow
/// - Opening the system browser
/// - Polling for session completion
/// - Storing session tokens
pub struct AuthFlow {
    api_client: ApiClient,
    session_store: SessionStore,
}

impl AuthFlow {
    /// Creates a new AuthFlow instance.
    pub fn new(api_client: ApiClient) -> Result<Self> {
        let session_store = SessionStore::new().context("Failed to initialize session store")?;

        Ok(Self {
            api_client,
            session_store,
        })
    }

    /// Checks for an existing session and validates it with the server.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(user))` if a valid session exists
    /// - `Ok(None)` if no session exists or the session is invalid
    /// - `Err(_)` if there's an error communicating with the server
    pub async fn check_existing_session(&mut self) -> Result<Option<User>> {
        // Try to load session token from file
        let token = match self.session_store.load()? {
            Some(t) => t,
            None => {
                log::debug!("No existing session found");
                return Ok(None);
            }
        };

        log::info!("Found existing session token, validating with server");

        // Set the session token in the API client
        self.api_client.set_session_token(Some(token.clone()));

        // Validate the session with the server
        match self.api_client.validate_session().await {
            Ok(response) if response.valid => {
                log::info!("Session is valid for user: {}", response.user.username);
                Ok(Some(response.user))
            }
            Ok(_) => {
                log::warn!("Session validation returned invalid");
                // Clear invalid session
                let _ = self.session_store.delete();
                Ok(None)
            }
            Err(e) => {
                log::warn!("Session validation failed: {}", e);
                // Clear invalid session
                let _ = self.session_store.delete();
                Ok(None)
            }
        }
    }

    /// Initiates the GitHub Device Flow by requesting a device code from the server.
    ///
    /// # Returns
    ///
    /// Returns a tuple of (device_code, user_code, verification_uri, interval) where:
    /// - `device_code` is used for polling
    /// - `user_code` is displayed to the user
    /// - `verification_uri` is where the user enters the code
    /// - `interval` is how often to poll (in seconds)
    pub async fn initiate_github_device_flow(&self) -> Result<(String, String, String, i64)> {
        log::info!("Initiating GitHub Device Flow");

        let response = self
            .api_client
            .github_device_flow()
            .await
            .context("Failed to initiate GitHub Device Flow")?;

        log::debug!(
            "Received device code with user code: {}",
            response.user_code
        );

        Ok((
            response.device_code,
            response.user_code,
            response.verification_uri,
            response.interval,
        ))
    }

    /// Opens the system browser to the given URL.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to open in the browser
    ///
    /// # Returns
    ///
    /// Returns an error if the browser cannot be opened.
    pub fn open_browser(&self, url: &str) -> Result<()> {
        log::info!("Opening browser to: {}", url);

        webbrowser::open(url).context("Failed to open browser")?;

        Ok(())
    }

    /// Polls the server for device flow completion after user authorization.
    ///
    /// This method polls the server at the specified interval for up to 15 minutes,
    /// waiting for the user to enter the code and authorize the device.
    ///
    /// # Arguments
    ///
    /// * `device_code` - The device code from the device flow initiation
    /// * `poll_interval_secs` - How often to poll (in seconds)
    ///
    /// # Returns
    ///
    /// Returns the user and session token once authorized, or an error if:
    /// - The timeout (15 minutes) is reached
    /// - There's a server communication error
    /// - The device flow fails
    pub async fn poll_for_device_authorization(
        &mut self,
        device_code: &str,
        poll_interval_secs: i64,
    ) -> Result<User> {
        log::info!(
            "Polling for device authorization (device_code: {})",
            device_code
        );

        let timeout = Duration::from_secs(900); // 15 minutes
        let poll_interval = Duration::from_secs(poll_interval_secs as u64);
        let start_time = Instant::now();

        loop {
            // Check if we've exceeded the timeout
            if start_time.elapsed() > timeout {
                anyhow::bail!("Device authorization timeout: No response after 15 minutes");
            }

            // Poll the server for authorization status
            match self.api_client.github_device_poll(device_code).await {
                Ok(login_response) => {
                    log::info!(
                        "Device authorization completed successfully for user: {}",
                        login_response.user.username
                    );

                    // Store the session token
                    self.session_store
                        .save(&login_response.session_token)
                        .context("Failed to save session token")?;

                    // Set the session token in the API client
                    self.api_client
                        .set_session_token(Some(login_response.session_token));

                    return Ok(login_response.user);
                }
                Err(e) => {
                    // Check if it's just pending
                    let error_msg = format!("{:?}", e);
                    if error_msg.contains("authorization_pending") {
                        log::debug!("Authorization still pending, continuing to poll");
                    } else {
                        log::error!("Error polling for device authorization: {}", e);
                        anyhow::bail!("Device authorization error: {}", e);
                    }
                }
            }

            // Wait before next poll
            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Saves a session token to the session store.
    ///
    /// This is useful when the session token is obtained through other means
    /// (e.g., test user login).
    pub fn save_session(&self, token: &str) -> Result<()> {
        self.session_store
            .save(token)
            .context("Failed to save session token")
    }

    /// Deletes the current session from the session store.
    pub fn delete_session(&self) -> Result<()> {
        self.session_store
            .delete()
            .context("Failed to delete session")
    }

    /// Gets a reference to the API client.
    pub fn api_client(&self) -> &ApiClient {
        &self.api_client
    }

    /// Gets a mutable reference to the API client.
    pub fn api_client_mut(&mut self) -> &mut ApiClient {
        &mut self.api_client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_flow_creation() {
        let api_client = ApiClient::default();
        let auth_flow = AuthFlow::new(api_client);
        assert!(auth_flow.is_ok());
    }
}
