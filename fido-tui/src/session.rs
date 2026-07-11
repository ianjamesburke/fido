use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Manages session token storage in the user's home directory.
///
/// Each server's token is stored in `~/.fido/session_<server-hash>` with 0600
/// permissions so one server cannot overwrite another server's session.
#[derive(Debug, Clone)]
pub struct SessionStore {
    file_path: PathBuf,
    legacy_file_path: Option<PathBuf>,
}

impl SessionStore {
    /// Creates a session store scoped to a specific server URL.
    ///
    /// Session tokens are server-side credentials, so a token issued by one
    /// Fido server must not overwrite or invalidate the token for another.
    #[cfg(not(test))]
    pub fn for_server(server_url: &str) -> Result<Self> {
        let home_dir = dirs::home_dir().context("Failed to determine home directory")?;
        Self::for_server_in_dir(home_dir.join(".fido"), server_url)
    }

    #[cfg(test)]
    pub fn for_server(server_url: &str) -> Result<Self> {
        let pid = std::process::id();
        Self::for_server_in_dir(
            std::env::temp_dir().join(format!("fido-test-{}", pid)),
            server_url,
        )
    }

    fn for_server_in_dir(fido_dir: PathBuf, server_url: &str) -> Result<Self> {
        let scope = server_scope_key(server_url);
        let file_path = fido_dir.join(format!("session_{}", scope));
        let legacy_file_path = Some(fido_dir.join("session"));

        Ok(Self {
            file_path,
            legacy_file_path,
        })
    }

    #[cfg(test)]
    fn for_server_in_test_dir(fido_dir: PathBuf, server_url: &str) -> Self {
        Self::for_server_in_dir(fido_dir, server_url).unwrap()
    }

    /// Loads the session token from the file.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(token))` if the file exists and contains a valid token
    /// - `Ok(None)` if the file doesn't exist
    /// - `Err(_)` if the file is corrupted or cannot be read
    pub fn load(&self) -> Result<Option<String>> {
        let file_path = if self.file_path.exists() {
            self.file_path.clone()
        } else if let Some(legacy_file_path) = &self.legacy_file_path {
            if legacy_file_path.exists() {
                legacy_file_path.clone()
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        };

        let content = fs::read_to_string(&file_path).context("Failed to read session file")?;

        // Validate session file format
        let token = content.trim();

        if token.is_empty() {
            log::warn!("Session file is empty, treating as no session");
            return Ok(None);
        }

        // Basic validation: session tokens should be reasonable length
        // UUIDs are 36 chars, but we allow flexibility for different token formats
        if token.len() < 8 || token.len() > 256 {
            log::warn!(
                "Session token has invalid length: {}, treating as corrupted",
                token.len()
            );
            return Ok(None);
        }

        // Check for obviously corrupted content (binary data, control characters, etc.)
        if token
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            log::warn!("Session file contains control characters, treating as corrupted");
            return Ok(None);
        }

        log::debug!(
            "Successfully loaded session token from {}",
            file_path.display()
        );
        let token = token.to_string();

        // Migrate legacy ~/.fido/session into the server-scoped file on first
        // successful read. After this, local/dev and production sessions no
        // longer clobber each other.
        if file_path != self.file_path {
            self.save(&token)?;
            if let Err(e) = fs::remove_file(&file_path) {
                log::warn!(
                    "Failed to remove legacy session file {}: {}",
                    file_path.display(),
                    e
                );
            }
        }

        Ok(Some(token))
    }

    /// Saves the session token to the file with 0600 permissions.
    ///
    /// This method:
    /// - Creates the `.fido` directory if it doesn't exist
    /// - Removes any old/stale session files
    /// - Uses atomic writes to prevent partial writes
    /// - Sets file permissions to 0600 (owner read/write only)
    ///
    /// # Arguments
    ///
    /// * `token` - The session token to save
    pub fn save(&self, token: &str) -> Result<()> {
        // Ensure the .fido directory exists
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent).context("Failed to create .fido directory")?;
        }

        // Remove any old/stale session files before saving
        self.cleanup_old_files()?;

        // Use atomic write: write to temporary file, then rename
        let temp_path = self.file_path.with_extension("tmp");

        // Write to temporary file
        let mut file =
            fs::File::create(&temp_path).context("Failed to create temporary session file")?;

        file.write_all(token.as_bytes())
            .context("Failed to write session token")?;

        file.sync_all()
            .context("Failed to sync session file to disk")?;

        drop(file);

        // Set permissions to 0600 (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&temp_path, permissions)
                .context("Failed to set session file permissions")?;
        }

        // Atomic rename
        fs::rename(&temp_path, &self.file_path)
            .context("Failed to rename temporary session file")?;

        log::info!(
            "Successfully saved session token to {}",
            self.file_path.display()
        );
        Ok(())
    }

    /// Deletes the session file.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` even if the file doesn't exist.
    pub fn delete(&self) -> Result<()> {
        if self.file_path.exists() {
            fs::remove_file(&self.file_path).context("Failed to delete session file")?;
            log::info!(
                "Successfully deleted session file at {}",
                self.file_path.display()
            );
        } else {
            log::debug!("Session file does not exist, nothing to delete");
        }
        Ok(())
    }

    /// Cleans up any old or stale session files.
    ///
    /// Removes temporary and backup files plus legacy instance-scoped JSON
    /// sessions. Tokens scoped to other servers are preserved.
    fn cleanup_old_files(&self) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            if !parent.exists() {
                return Ok(());
            }

            let entries = fs::read_dir(parent).context("Failed to read .fido directory")?;

            for entry in entries {
                let entry = entry.context("Failed to read directory entry")?;
                let path = entry.path();

                // Skip if it's the current session file
                if path == self.file_path {
                    continue;
                }

                // Remove temporary files, backup files, and old session files
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    let is_stale_session_file = file_name == "session.tmp"
                        || file_name == "session.bak"
                        || file_name == "session.old"
                        || (file_name.starts_with("session_")
                            && (file_name.ends_with(".tmp")
                                || file_name.ends_with(".bak")
                                || file_name.ends_with(".old")
                                || file_name.ends_with(".json")));

                    if is_stale_session_file {
                        log::debug!("Removing old/stale session file: {}", path.display());
                        if let Err(e) = fs::remove_file(&path) {
                            log::warn!(
                                "Failed to remove old session file {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

fn server_scope_key(server_url: &str) -> String {
    let normalized = server_url.trim().trim_end_matches('/').to_ascii_lowercase();
    let mut hash = 0xcbf29ce484222325_u64;

    for byte in normalized.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_store(temp_dir: &TempDir) -> SessionStore {
        let file_path = temp_dir.path().join("session");
        SessionStore {
            file_path,
            legacy_file_path: None,
        }
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(&temp_dir);

        let token = "test-token-12345";
        store.save(token).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded, Some(token.to_string()));
    }

    #[test]
    fn test_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(&temp_dir);

        let loaded = store.load().unwrap();
        assert_eq!(loaded, None);
    }

    #[test]
    fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(&temp_dir);

        let token = "test-token-12345";
        store.save(token).unwrap();
        assert!(store.file_path.exists());

        store.delete().unwrap();
        assert!(!store.file_path.exists());
    }

    #[test]
    fn test_delete_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(&temp_dir);

        // Should not error even if file doesn't exist
        store.delete().unwrap();
    }

    #[test]
    fn test_empty_file_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(&temp_dir);

        // Create empty file
        fs::write(&store.file_path, "").unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded, None);
    }

    #[test]
    fn test_whitespace_only_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(&temp_dir);

        // Create file with only whitespace
        fs::write(&store.file_path, "   \n\t  ").unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded, None);
    }

    #[test]
    fn test_cleanup_old_files() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(&temp_dir);

        // Create some old session files
        fs::write(temp_dir.path().join("session.bak"), "old-token").unwrap();
        fs::write(temp_dir.path().join("session.tmp"), "temp-token").unwrap();
        fs::write(temp_dir.path().join("session.old"), "old-token-2").unwrap();
        fs::write(temp_dir.path().join("session_deadbeef"), "other-server").unwrap();
        fs::write(
            temp_dir.path().join("session_deadbeef.json"),
            "old-instance-session",
        )
        .unwrap();

        // Save a new session (should clean up old files)
        store.save("new-token").unwrap();

        // Check that old files are removed
        assert!(!temp_dir.path().join("session.bak").exists());
        assert!(!temp_dir.path().join("session.tmp").exists());
        assert!(!temp_dir.path().join("session.old").exists());
        assert!(temp_dir.path().join("session_deadbeef").exists());
        assert!(!temp_dir.path().join("session_deadbeef.json").exists());

        // Check that the new session file exists
        assert!(store.file_path.exists());
    }

    #[test]
    fn test_server_scoped_sessions_are_independent() {
        let temp_dir = TempDir::new().unwrap();
        let prod = SessionStore::for_server_in_test_dir(
            temp_dir.path().to_path_buf(),
            "https://fido.example.com",
        );
        let local = SessionStore::for_server_in_test_dir(
            temp_dir.path().to_path_buf(),
            "http://localhost:3000",
        );

        prod.save("prod-token").unwrap();
        local.save("local-token").unwrap();

        assert_eq!(prod.load().unwrap(), Some("prod-token".to_string()));
        assert_eq!(local.load().unwrap(), Some("local-token".to_string()));

        local.delete().unwrap();

        assert_eq!(prod.load().unwrap(), Some("prod-token".to_string()));
        assert_eq!(local.load().unwrap(), None);
    }

    #[test]
    fn test_legacy_session_migrates_to_server_scoped_file() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("session"), "legacy-token").unwrap();

        let store = SessionStore::for_server_in_test_dir(
            temp_dir.path().to_path_buf(),
            "https://fido.example.com/",
        );

        assert_eq!(store.load().unwrap(), Some("legacy-token".to_string()));
        assert!(store.file_path.exists());
        assert!(!temp_dir.path().join("session").exists());
        assert_eq!(
            fs::read_to_string(&store.file_path).unwrap(),
            "legacy-token"
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(&temp_dir);

        store.save("test-token").unwrap();

        let metadata = fs::metadata(&store.file_path).unwrap();
        let permissions = metadata.permissions();

        // Check that permissions are 0600
        assert_eq!(permissions.mode() & 0o777, 0o600);
    }

    #[test]
    fn test_invalid_token_length() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(&temp_dir);

        // Token too short (less than 8 characters)
        fs::write(&store.file_path, "short").unwrap();
        assert_eq!(store.load().unwrap(), None);

        // Token too long (over 256 characters)
        let long_token = "a".repeat(300);
        fs::write(&store.file_path, long_token).unwrap();
        assert_eq!(store.load().unwrap(), None);
    }

    #[test]
    fn test_corrupted_file_with_control_chars() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(&temp_dir);

        // Write binary data with control characters
        fs::write(&store.file_path, b"token\x00with\x01control\x02chars").unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded, None);
    }
}
