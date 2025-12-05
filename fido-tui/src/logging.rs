use log::LevelFilter;
use simplelog::*;
use std::fs::File;
use std::path::PathBuf;

/// Logging configuration for the Fido TUI application
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Master switch to enable/disable all logging
    pub enabled: bool,
    /// Path to the log file
    pub log_file: PathBuf,
    /// Whether to clear the log file on startup
    pub clear_on_startup: bool,
    /// Feature flags for specific logging categories
    pub features: LogFeatures,
    /// Overall log level
    pub level: LevelFilter,
}

/// Feature flags for specific logging categories
#[derive(Debug, Clone)]
pub struct LogFeatures {
    /// Log modal state changes
    pub modal_state: bool,
    /// Log key events
    pub key_events: bool,
    /// Log rendering operations
    pub rendering: bool,
    /// Log API calls
    pub api_calls: bool,
    /// Log settings changes
    pub settings: bool,
    /// Log general debug messages
    pub general: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_file: PathBuf::from("fido_debug.log"),
            clear_on_startup: true,
            features: LogFeatures::default(),
            level: LevelFilter::Debug,
        }
    }
}

impl Default for LogFeatures {
    fn default() -> Self {
        Self {
            modal_state: true,
            key_events: true,
            rendering: true,
            api_calls: true,
            settings: true,
            general: true,
        }
    }
}

impl LogConfig {
    /// Create a new log configuration with all features disabled
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Create a minimal log configuration (only errors and warnings)
    pub fn minimal() -> Self {
        Self {
            enabled: true,
            level: LevelFilter::Warn,
            features: LogFeatures {
                modal_state: false,
                key_events: false,
                rendering: false,
                api_calls: false,
                settings: false,
                general: false,
            },
            ..Default::default()
        }
    }

    /// Create a verbose log configuration (all features enabled)
    pub fn verbose() -> Self {
        Self {
            enabled: true,
            level: LevelFilter::Trace,
            features: LogFeatures {
                modal_state: true,
                key_events: true,
                rendering: true,
                api_calls: true,
                settings: true,
                general: true,
            },
            ..Default::default()
        }
    }
}

/// Initialize the logging system with the given configuration
pub fn init_logging(config: &LogConfig) -> anyhow::Result<()> {
    if !config.enabled {
        // Initialize with no-op logger
        let _ = WriteLogger::init(
            LevelFilter::Off,
            Config::default(),
            std::io::sink(),
        );
        return Ok(());
    }

    // Clear log file if requested
    if config.clear_on_startup {
        let _ = File::create(&config.log_file)?;
    }

    // Open log file
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.log_file)?;

    // Configure log format
    let log_config = ConfigBuilder::new()
        .set_time_format_rfc3339()
        .set_time_offset_to_local()
        .unwrap_or_else(|builder| builder)
        .build();

    // Initialize logger
    WriteLogger::init(config.level, log_config, log_file)?;

    log::info!("Logging initialized: file={}, level={:?}", config.log_file.display(), config.level);
    log::debug!("Log features: {:?}", config.features);

    Ok(())
}

/// Macro for logging modal state changes
#[macro_export]
macro_rules! log_modal_state {
    ($config:expr, $($arg:tt)*) => {
        if $config.enabled && $config.features.modal_state {
            log::debug!(target: "modal_state", $($arg)*);
        }
    };
}

/// Macro for logging key events
#[macro_export]
macro_rules! log_key_event {
    ($config:expr, $($arg:tt)*) => {
        if $config.enabled && $config.features.key_events {
            log::debug!(target: "key_events", $($arg)*);
        }
    };
}

/// Macro for logging rendering operations
#[macro_export]
macro_rules! log_rendering {
    ($config:expr, $($arg:tt)*) => {
        if $config.enabled && $config.features.rendering {
            log::debug!(target: "rendering", $($arg)*);
        }
    };
}

/// Macro for logging API calls
#[macro_export]
macro_rules! log_api_call {
    ($config:expr, $($arg:tt)*) => {
        if $config.enabled && $config.features.api_calls {
            log::debug!(target: "api_calls", $($arg)*);
        }
    };
}

/// Macro for logging settings changes
#[macro_export]
macro_rules! log_settings {
    ($config:expr, $($arg:tt)*) => {
        if $config.enabled && $config.features.settings {
            log::debug!(target: "settings", $($arg)*);
        }
    };
}

/// Macro for general debug logging
#[macro_export]
macro_rules! log_debug {
    ($config:expr, $($arg:tt)*) => {
        if $config.enabled && $config.features.general {
            log::debug!(target: "general", $($arg)*);
        }
    };
}
