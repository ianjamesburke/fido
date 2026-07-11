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
    /// Log settings changes
    pub settings: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        // Production default: logging disabled to avoid writing files to user's system
        Self {
            enabled: false,
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
            settings: true,
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

    /// Create a verbose log configuration (all features enabled)
    pub fn verbose() -> Self {
        Self {
            enabled: true,
            level: LevelFilter::Trace,
            features: LogFeatures {
                modal_state: true,
                key_events: true,
                rendering: true,
                settings: true,
            },
            ..Default::default()
        }
    }
}

/// Open the log file 0600 (owner read/write only) on unix. When `truncate` is
/// set the file is cleared; otherwise it is opened for append.
fn open_log_file(path: &PathBuf, truncate: bool) -> std::io::Result<File> {
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true);
    if truncate {
        opts.write(true).truncate(true);
    } else {
        opts.append(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)
}

/// Initialize the logging system with the given configuration
pub fn init_logging(config: &LogConfig) -> anyhow::Result<()> {
    if !config.enabled {
        // Initialize with no-op logger
        let _ = WriteLogger::init(LevelFilter::Off, Config::default(), std::io::sink());
        return Ok(());
    }

    // Clear log file if requested
    if config.clear_on_startup {
        let _ = open_log_file(&config.log_file, true)?;
    }

    // Open log file (owner-only, since debug logs may contain sensitive data).
    let log_file = open_log_file(&config.log_file, false)?;

    // Configure log format. Third-party crates are capped below the app level:
    // at TRACE, tungstenite dumps the WebSocket handshake — including the
    // `x-session-token` header — into this plaintext file, so those targets are
    // filtered out entirely rather than allowed to leak the credential.
    let log_config = ConfigBuilder::new()
        .set_time_format_rfc3339()
        .set_time_offset_to_local()
        .unwrap_or_else(|builder| builder)
        .add_filter_ignore_str("tungstenite")
        .add_filter_ignore_str("tokio_tungstenite")
        .build();

    // Initialize logger
    WriteLogger::init(config.level, log_config, log_file)?;

    log::info!(
        "Logging initialized: file={}, level={:?}",
        config.log_file.display(),
        config.level
    );
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

/// Macro for logging settings changes
#[macro_export]
macro_rules! log_settings {
    ($config:expr, $($arg:tt)*) => {
        if $config.enabled && $config.features.settings {
            log::debug!(target: "settings", $($arg)*);
        }
    };
}
