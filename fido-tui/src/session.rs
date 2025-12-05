use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Manages session token storage in the user's home directory.
/// 
/// The session token is stored in `~/.fido/session` with 0600 permissions
/// to ensure only the owner can read/write the file.
#[derive(Debug, Clone)]
pub struct SessionStore {
    file_path: PathBuf,
}

impl SessionStore {
    /// Creates a new SessionStore with the default path `~/.fido/session`.
    /// 
    /// # Returns
    /// 
    /// Returns an error if the home directory cannot be determined.
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir()
            .context("Failed to determine home directory")?;
        
        let fido_dir = home_dir.join(".fido");
        let file_path = fido_dir.join("session");
        
        Ok(Self { file_path })
    }

    /// Loads the session token from the file.
    /// 
    /// # Returns
    /// 
    /// - `Ok(Some(token))` if the file exists and contains a valid token
    /// - `Ok(None)` if the file doesn't exist
    /// - `Err(_)` if the file is corrupted or cannot be read
    pub fn load(&self) -> Result<Option<String>> {
        if !self.file_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&self.file_path)
            .context("Failed to read session file")?;
        
        // Validate session file format
        let token = content.trim();
        
        if token.is_empty() {
            log::warn!("Session file is empty, treating as no session");
            return Ok(None);
        }
        
        // Basic validation: session tokens should be reasonable length
        // UUIDs are 36 chars, but we allow flexibility for different token formats
        if token.len() < 8 || token.len() > 256 {
            log::warn!("Session token has invalid length: {}, treating as corrupted", token.len());
            return Ok(None);
        }
        
        // Check for obviously corrupted content (binary data, control characters, etc.)
        if token.chars().any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t') {
            log::warn!("Session file contains control characters, treating as corrupted");
            return Ok(None);
        }
        
        log::debug!("Successfully loaded session token from {}", self.file_path.display());
        Ok(Some(token.to_string()))
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
            fs::create_dir_all(parent)
                .context("Failed to create .fido directory")?;
        }

        // Remove any old/stale session files before saving
        self.cleanup_old_files()?;

        // Use atomic write: write to temporary file, then rename
        let temp_path = self.file_path.with_extension("tmp");
        
        // Write to temporary file
        let mut file = fs::File::create(&temp_path)
            .context("Failed to create temporary session file")?;
        
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

        log::info!("Successfully saved session token to {}", self.file_path.display());
        Ok(())
    }

    /// Deletes the session file.
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(())` even if the file doesn't exist.
    pub fn delete(&self) -> Result<()> {
        if self.file_path.exists() {
            fs::remove_file(&self.file_path)
                .context("Failed to delete session file")?;
            log::info!("Successfully deleted session file at {}", self.file_path.display());
        } else {
            log::debug!("Session file does not exist, nothing to delete");
        }
        Ok(())
    }

    /// Cleans up any old or stale session files.
    /// 
    /// This ensures only ONE session file exists per user by removing:
    /// - Backup files (*.bak)
    /// - Temporary files (*.tmp)
    /// - Old session files with different extensions
    fn cleanup_old_files(&self) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            if !parent.exists() {
                return Ok(());
            }

            let entries = fs::read_dir(parent)
                .context("Failed to read .fido directory")?;

            for entry in entries {
                let entry = entry.context("Failed to read directory entry")?;
                let path = entry.path();
                
                // Skip if it's the current session file
                if path == self.file_path {
                    continue;
                }

                // Remove temporary files, backup files, and old session files
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with("session") {
                        log::debug!("Removing old/stale session file: {}", path.display());
                        if let Err(e) = fs::remove_file(&path) {
                            log::warn!("Failed to remove old session file {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Returns the path to the session file.
    pub fn path(&self) -> &PathBuf {
        &self.file_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_store(temp_dir: &TempDir) -> SessionStore {
        let file_path = temp_dir.path().join("session");
        SessionStore { file_path }
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
        
        // Save a new session (should clean up old files)
        store.save("new-token").unwrap();
        
        // Check that old files are removed
        assert!(!temp_dir.path().join("session.bak").exists());
        assert!(!temp_dir.path().join("session.tmp").exists());
        assert!(!temp_dir.path().join("session.old").exists());
        
        // Check that the new session file exists
        assert!(store.file_path.exists());
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
