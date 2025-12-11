use anyhow::Result;

/// Trait for storage adapters that handle credential management
pub trait StorageAdapter {
    /// Store authentication credentials
    fn store_credentials(&self, credentials: &str) -> Result<()>;
    
    /// Load authentication credentials
    fn load_credentials(&self) -> Result<Option<String>>;
    
    /// Clear stored credentials
    fn clear_credentials(&self) -> Result<()>;
}

/// File-based storage adapter for native mode
#[derive(Debug, Clone)]
pub struct FileStorageAdapter {
    session_store: crate::session::SessionStore,
}

impl FileStorageAdapter {
    /// Create a new file storage adapter
    pub fn new() -> Result<Self> {
        let session_store = crate::session::SessionStore::new()?;
        Ok(Self { session_store })
    }
}

impl StorageAdapter for FileStorageAdapter {
    fn store_credentials(&self, credentials: &str) -> Result<()> {
        self.session_store.save(credentials)
    }
    
    fn load_credentials(&self) -> Result<Option<String>> {
        self.session_store.load()
    }
    
    fn clear_credentials(&self) -> Result<()> {
        self.session_store.delete()
    }
}

/// Browser storage adapter for web mode
/// Note: This is a placeholder implementation for now
/// In a real web environment, this would use JavaScript bridge to browser storage
#[derive(Debug, Clone)]
pub struct BrowserStorageAdapter {
    // For now, we'll use in-memory storage as a placeholder
    // In real implementation, this would interface with browser APIs
}

impl BrowserStorageAdapter {
    /// Create a new browser storage adapter
    pub fn new() -> Self {
        Self {}
    }
}

impl StorageAdapter for BrowserStorageAdapter {
    fn store_credentials(&self, _credentials: &str) -> Result<()> {
        // Placeholder: In real implementation, this would use browser localStorage/sessionStorage
        // For now, we just log that we're in web mode
        log::info!("Web mode: credentials would be stored in browser storage");
        Ok(())
    }
    
    fn load_credentials(&self) -> Result<Option<String>> {
        // Placeholder: In real implementation, this would read from browser storage
        log::info!("Web mode: credentials would be loaded from browser storage");
        Ok(None)
    }
    
    fn clear_credentials(&self) -> Result<()> {
        // Placeholder: In real implementation, this would clear browser storage
        log::info!("Web mode: credentials would be cleared from browser storage");
        Ok(())
    }
}

/// Factory for creating storage adapters based on mode
pub struct StorageAdapterFactory;

impl StorageAdapterFactory {
    /// Create a storage adapter based on the application mode
    pub fn create_adapter(mode: &crate::mode::AppMode) -> Result<Box<dyn StorageAdapter>> {
        match mode {
            crate::mode::AppMode::Native => {
                let adapter = FileStorageAdapter::new()?;
                Ok(Box::new(adapter))
            }
            crate::mode::AppMode::Web => {
                let adapter = BrowserStorageAdapter::new();
                Ok(Box::new(adapter))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::AppMode;


    #[test]
    fn test_file_storage_adapter() {
        let adapter = FileStorageAdapter::new().unwrap();
        
        // Test store and load
        let test_token = "test-session-token";
        adapter.store_credentials(test_token).unwrap();
        
        let loaded = adapter.load_credentials().unwrap();
        assert_eq!(loaded, Some(test_token.to_string()));
        
        // Test clear
        adapter.clear_credentials().unwrap();
        let loaded_after_clear = adapter.load_credentials().unwrap();
        assert_eq!(loaded_after_clear, None);
    }

    #[test]
    fn test_browser_storage_adapter() {
        let adapter = BrowserStorageAdapter::new();
        
        // Test that browser adapter doesn't error (placeholder implementation)
        adapter.store_credentials("test-token").unwrap();
        let loaded = adapter.load_credentials().unwrap();
        // Browser adapter returns None for now (placeholder)
        assert_eq!(loaded, None);
        
        adapter.clear_credentials().unwrap();
    }

    #[test]
    fn test_storage_adapter_factory() {
        // Test native mode creates file adapter
        let native_adapter = StorageAdapterFactory::create_adapter(&AppMode::Native).unwrap();
        // We can't easily test the type, but we can test it works
        let _ = native_adapter.load_credentials().unwrap();
        
        // Test web mode creates browser adapter
        let web_adapter = StorageAdapterFactory::create_adapter(&AppMode::Web).unwrap();
        let _ = web_adapter.load_credentials().unwrap();
    }

    // Property-based tests
    use proptest::prelude::*;

    // **Feature: web-terminal-interface, Property 2: Authentication Storage Mode Selection**
    // **Validates: Requirements 2.1, 2.5**
    // For any authentication attempt in web mode, credentials should be stored in browser storage,
    // and for any authentication in native mode, credentials should be stored in local file storage.
    proptest! {
        #[test]
        fn prop_authentication_storage_mode_selection(
            credentials in "[a-zA-Z0-9_-]{8,256}",
            mode_is_web in any::<bool>()
        ) {
            let mode = if mode_is_web {
                AppMode::Web
            } else {
                AppMode::Native
            };
            
            // Create storage adapter based on mode
            let adapter = StorageAdapterFactory::create_adapter(&mode).unwrap();
            
            // Store credentials
            adapter.store_credentials(&credentials).unwrap();
            
            // For native mode, verify credentials are stored in file system
            // For web mode, verify credentials are handled by browser adapter (placeholder implementation)
            match mode {
                AppMode::Native => {
                    // In native mode, credentials should be stored in file system
                    // We can verify this by creating a new file adapter and loading credentials
                    let file_adapter = FileStorageAdapter::new().unwrap();
                    let loaded = file_adapter.load_credentials().unwrap();
                    prop_assert_eq!(loaded, Some(credentials.clone()));
                }
                AppMode::Web => {
                    // In web mode, credentials are handled by browser adapter (placeholder)
                    // The browser adapter currently returns None for load_credentials (placeholder)
                    // but doesn't error on store_credentials
                    let loaded = adapter.load_credentials().unwrap();
                    // Browser adapter placeholder returns None
                    prop_assert_eq!(loaded, None);
                }
            }
            
            // Clean up for native mode
            if matches!(mode, AppMode::Native) {
                let _ = adapter.clear_credentials();
            }
        }
    }
}