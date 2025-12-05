# Fido Logging System

## Overview

Fido uses a cohesive, file-based logging system built on Rust's `log` and `simplelog` crates. Since Fido is a terminal application, all debug output must be written to files to avoid interfering with the TUI.

## Quick Start

### Disable All Logging

In `fido-tui/src/main.rs`:

```rust
let log_config = logging::LogConfig::disabled();
logging::init_logging(&log_config)?;
```

### Minimal Logging (Errors/Warnings Only)

```rust
let log_config = logging::LogConfig::minimal();
logging::init_logging(&log_config)?;
```

### Verbose Logging (All Features)

```rust
let log_config = logging::LogConfig::verbose();
logging::init_logging(&log_config)?;
```

### Default Configuration

```rust
let log_config = logging::LogConfig::default();
logging::init_logging(&log_config)?;
```

## Configuration Options

### LogConfig Structure

```rust
pub struct LogConfig {
    /// Master switch - set to false to disable all logging
    pub enabled: bool,
    
    /// Path to the log file (default: "fido_debug.log")
    pub log_file: PathBuf,
    
    /// Clear log file on startup (default: true)
    pub clear_on_startup: bool,
    
    /// Feature flags for specific logging categories
    pub features: LogFeatures,
    
    /// Overall log level (default: Debug)
    pub level: LevelFilter,
}
```

### Log Features

Control which categories of logs are written:

```rust
pub struct LogFeatures {
    pub modal_state: bool,    // Modal state changes
    pub key_events: bool,     // Keyboard input events
    pub rendering: bool,      // UI rendering operations
    pub api_calls: bool,      // API/network requests
    pub settings: bool,       // Settings changes
    pub general: bool,        // General debug messages
}
```

### Custom Configuration Example

```rust
let log_config = logging::LogConfig {
    enabled: true,
    log_file: PathBuf::from("my_custom_log.log"),
    clear_on_startup: false,  // Append to existing log
    features: logging::LogFeatures {
        modal_state: true,
        key_events: true,
        rendering: false,     // Disable rendering logs
        api_calls: true,
        settings: false,      // Disable settings logs
        general: true,
    },
    level: LevelFilter::Info, // Only Info and above
};
logging::init_logging(&log_config)?;
```

## Using the Logging Macros

### Modal State Logging

```rust
log_modal_state!(app.log_config, 
    "viewing_post_detail={}, composer_open={}", 
    app.viewing_post_detail, 
    app.composer_state.is_open()
);
```

### Key Event Logging

```rust
log_key_event!(app.log_config, 
    "key={:?}, context={}", 
    key.code, 
    "main_view"
);
```

### Rendering Logging

```rust
log_rendering!(app.log_config, 
    "Rendering composer modal (mode: {})", 
    composer_mode
);
```

### API Call Logging

```rust
log_api_call!(app.log_config, 
    "GET /posts - status: {}", 
    response.status()
);
```

### Settings Logging

```rust
log_settings!(app.log_config, 
    "Color scheme changed to: {:?}", 
    new_scheme
);
```

### General Debug Logging

```rust
log_debug!(app.log_config, 
    "Processing {} items", 
    items.len()
);
```

## Standard Rust Logging

You can also use standard Rust logging macros for critical messages that should always be logged:

```rust
log::error!("Critical error: {}", error);
log::warn!("Warning: {}", warning);
log::info!("Application started");
```

These will respect the `enabled` flag and `level` setting but bypass feature flags.

## Log Levels

- `Trace` - Very detailed, typically only for debugging specific issues
- `Debug` - Detailed information for debugging (default)
- `Info` - General informational messages
- `Warn` - Warning messages
- `Error` - Error messages
- `Off` - No logging

## Log File Location

By default, logs are written to `fido_debug.log` in the current working directory (typically the `fido` project root).

## Best Practices

1. **Use feature-specific macros** - Use `log_key_event!`, `log_rendering!`, etc. instead of generic `log_debug!` for better control
2. **Disable in production** - Set `enabled: false` for production builds
3. **Clear on startup** - Keep `clear_on_startup: true` to avoid massive log files
4. **Use appropriate levels** - Reserve `Error` and `Warn` for actual problems
5. **Include context** - Add relevant context to log messages (IDs, states, etc.)
6. **Check before expensive operations** - The macros check feature flags before evaluating arguments

## Migration from debug_log

The old `debug_log` module is deprecated. Here's how to migrate:

### Old Code
```rust
debug_log::log_modal_state(viewing, show_modal, composer_open, &mode);
debug_log::log_key_event(&format!("{:?}", key.code), context);
debug_log::log_debug("Some message");
```

### New Code
```rust
log_modal_state!(app.log_config, "viewing={}, show_modal={}, composer_open={}, mode={}", 
    viewing, show_modal, composer_open, mode);
log_key_event!(app.log_config, "key={:?}, context={}", key.code, context);
log_debug!(app.log_config, "Some message");
```

## Troubleshooting

### No logs appearing

1. Check that `log_config.enabled` is `true`
2. Verify the specific feature flag is enabled (e.g., `features.key_events`)
3. Ensure `log_config.level` is set appropriately (e.g., `Debug` or `Trace`)
4. Check file permissions for the log file path

### Too many logs

1. Use `LogConfig::minimal()` for errors/warnings only
2. Disable specific features you don't need
3. Increase the log level to `Info` or `Warn`

### Log file too large

1. Enable `clear_on_startup: true` (default)
2. Manually delete the log file periodically
3. Consider implementing log rotation (future enhancement)
