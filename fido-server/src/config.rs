use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub server: Server,
    pub database: Database,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        // 1. Try to load from settings.toml (optional for deployment)
        let config_file_name = "settings.toml";
        
        // Check in current directory
        let current_dir_path = PathBuf::from(config_file_name);
        if current_dir_path.exists() {
            builder = builder.add_source(File::from(current_dir_path).required(false));
        }

        // Check in fido-server directory (for development)
        let dev_path = PathBuf::from("fido-server").join(config_file_name);
        if dev_path.exists() {
            builder = builder.add_source(File::from(dev_path).required(false));
        }

        // 2. Override with environment variables (highest priority)
        // Default to 0.0.0.0 for production deployment (allows external connections)
        // Can be overridden with HOST environment variable for local development
        builder = builder
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 3000)?
            .set_default("database.path", "fido.db")?;

        // Read from environment variables
        if let Ok(db_path) = std::env::var("DATABASE_PATH") {
            builder = builder.set_override("database.path", db_path)?;
        }
        if let Ok(port) = std::env::var("PORT") {
            builder = builder.set_override("server.port", port)?;
        }
        if let Ok(host) = std::env::var("HOST") {
            builder = builder.set_override("server.host", host)?;
        }

        let s = builder.build()?;
        s.try_deserialize()
    }
}
