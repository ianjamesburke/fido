use fido_server::oauth::GitHubOAuthConfig;
use std::env;

#[test]
fn test_oauth_config_loads_from_env() {
    // Load .env file from parent directory (fido/.env)
    dotenv::from_filename("../.env").ok();
    
    // This test verifies that OAuth config can be loaded from environment
    // It will fail if GITHUB_CLIENT_ID or GITHUB_CLIENT_SECRET are not set
    
    // Debug: print current directory and env vars
    println!("Current directory: {:?}", env::current_dir());
    println!("GITHUB_CLIENT_ID set: {}", env::var("GITHUB_CLIENT_ID").is_ok());
    println!("GITHUB_CLIENT_SECRET set: {}", env::var("GITHUB_CLIENT_SECRET").is_ok());
    
    let result = GitHubOAuthConfig::from_env();
    
    match result {
        Ok(config) => {
            println!("✓ OAuth config loaded successfully");
            
            // Test that we can generate an authorization URL
            let state = "test_state_123";
            let auth_url = config.get_authorization_url(state);
            
            println!("✓ Authorization URL generated: {}", auth_url);
            
            assert!(auth_url.contains("github.com/login/oauth/authorize"));
            assert!(auth_url.contains("client_id="));
            assert!(auth_url.contains("state=test_state_123"));
            assert!(auth_url.contains("redirect_uri="));
            assert!(auth_url.contains("scope="));
            
            println!("✓ Authorization URL format is correct");
            println!("✓ Redirect URI: {}", config.redirect_uri());
        }
        Err(e) => {
            panic!("Failed to load OAuth config: {}\n\nMake sure GITHUB_CLIENT_ID and GITHUB_CLIENT_SECRET are set in fido/fido-server/.env", e);
        }
    }
}
