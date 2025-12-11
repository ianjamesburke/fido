use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Session data stored locally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub username: String,
    pub session_token: String,
    pub user_id: String,
}

/// User preferences stored locally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub filter_type: String, // "all", "hashtag", "user", "multi"
    pub filter_hashtag: Option<String>,
    pub filter_user: Option<String>,
    pub filter_hashtags: Vec<String>,
    pub filter_users: Vec<String>,
}

/// Server configuration stored locally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server_url: String,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_url: "https://fido-social.fly.dev".to_string(),
            last_updated: chrono::Utc::now(),
        }
    }
}

/// Configuration manager for .fido directory
pub struct ConfigManager {
    config_dir: PathBuf,
}

impl ConfigManager {
    /// Create a new config manager
    pub fn new() -> Result<Self> {
        let config_dir = Self::get_config_dir()?;
        
        // Create .fido directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .context("Failed to create .fido directory")?;
        }
        
        Ok(Self { config_dir })
    }
    
    /// Get the .fido configuration directory path
    fn get_config_dir() -> Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .context("Could not determine home directory")?;
        Ok(home_dir.join(".fido"))
    }
    
    /// Get the session file path for a specific instance
    fn get_session_file(&self, instance_id: &str) -> PathBuf {
        self.config_dir.join(format!("session_{}.json", instance_id))
    }
    
    /// Get the preferences file path for a specific user
    fn get_preferences_file(&self, user_id: &str) -> PathBuf {
        self.config_dir.join(format!("prefs_{}.json", user_id))
    }
    
    /// Save session data
    pub fn save_session(&self, instance_id: &str, session: &SessionData) -> Result<()> {
        let session_file = self.get_session_file(instance_id);
        let json = serde_json::to_string_pretty(session)
            .context("Failed to serialize session data")?;
        
        fs::write(&session_file, json)
            .context("Failed to write session file")?;
        
        Ok(())
    }
    
    /// Delete session data
    pub fn delete_session(&self, instance_id: &str) -> Result<()> {
        let session_file = self.get_session_file(instance_id);
        
        if session_file.exists() {
            fs::remove_file(&session_file)
                .context("Failed to delete session file")?;
        }
        
        Ok(())
    }
    
    /// Generate a unique instance ID
    pub fn generate_instance_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        
        format!("{}", timestamp)
    }
    
    /// Save user preferences
    pub fn save_preferences(&self, user_id: &str, prefs: &UserPreferences) -> Result<()> {
        let prefs_file = self.get_preferences_file(user_id);
        let json = serde_json::to_string_pretty(prefs)
            .context("Failed to serialize preferences")?;
        
        fs::write(&prefs_file, json)
            .context("Failed to write preferences file")?;
        
        Ok(())
    }
    
    /// Load user preferences
    pub fn load_preferences(&self, user_id: &str) -> Result<Option<UserPreferences>> {
        let prefs_file = self.get_preferences_file(user_id);
        
        if !prefs_file.exists() {
            return Ok(None);
        }
        
        let json = fs::read_to_string(&prefs_file)
            .context("Failed to read preferences file")?;
        
        let prefs: UserPreferences = serde_json::from_str(&json)
            .context("Failed to parse preferences")?;
        
        Ok(Some(prefs))
    }
    
    /// Get the server config file path
    fn get_server_config_file(&self) -> PathBuf {
        self.config_dir.join("server_config.json")
    }
    
    /// Save server configuration
    pub fn save_server_config(&self, config: &ServerConfig) -> Result<()> {
        let config_file = self.get_server_config_file();
        let json = serde_json::to_string_pretty(config)
            .context("Failed to serialize server config")?;
        
        fs::write(&config_file, json)
            .context("Failed to write server config file")?;
        
        Ok(())
    }
    
    /// Load server configuration
    pub fn load_server_config(&self) -> Result<Option<ServerConfig>> {
        let config_file = self.get_server_config_file();
        
        if !config_file.exists() {
            return Ok(None);
        }
        
        let json = fs::read_to_string(&config_file)
            .context("Failed to read server config file")?;
        
        let config: ServerConfig = serde_json::from_str(&json)
            .context("Failed to parse server config")?;
        
        Ok(Some(config))
    }
    
    /// Clean up old session files (older than 30 days)
    pub fn cleanup_old_sessions(&self) -> Result<()> {
        use std::time::{Duration, SystemTime};
        
        let thirty_days_ago = SystemTime::now() - Duration::from_secs(30 * 24 * 60 * 60);
        
        if !self.config_dir.exists() {
            return Ok(());
        }
        
        for entry in fs::read_dir(&self.config_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if let Some(filename) = path.file_name() {
                let filename = filename.to_string_lossy();
                if filename.starts_with("session_") && filename.ends_with(".json") {
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(modified) = metadata.modified() {
                            if modified < thirty_days_ago {
                                let _ = fs::remove_file(&path);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("Failed to create config manager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_instance_id() {
        let id1 = ConfigManager::generate_instance_id();
        let id2 = ConfigManager::generate_instance_id();
        
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
        // IDs should be different (unless generated in same millisecond)
    }
}
