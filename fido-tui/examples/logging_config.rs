/// Example showing different logging configurations
///
/// Run with: cargo run --example logging_config
use fido::logging::{LogConfig, LogFeatures};
use log::LevelFilter;
use std::path::PathBuf;

fn main() {
    println!("=== Fido Logging Configuration Examples ===\n");

    // Example 1: Disabled logging
    println!("1. Disabled logging:");
    let disabled = LogConfig::disabled();
    println!("   enabled: {}", disabled.enabled);
    println!("   log_file: {:?}\n", disabled.log_file);

    // Example 2: Verbose logging (all features)
    println!("2. Verbose logging (all features):");
    let verbose = LogConfig::verbose();
    println!("   enabled: {}", verbose.enabled);
    println!("   level: {:?}", verbose.level);
    println!("   features.modal_state: {}", verbose.features.modal_state);
    println!("   features.key_events: {}", verbose.features.key_events);
    println!("   features.rendering: {}\n", verbose.features.rendering);

    // Example 3: Default configuration
    println!("3. Default configuration:");
    let default = LogConfig::default();
    println!("   enabled: {}", default.enabled);
    println!("   level: {:?}", default.level);
    println!("   clear_on_startup: {}", default.clear_on_startup);
    println!("   log_file: {:?}\n", default.log_file);

    // Example 4: Custom configuration
    println!("4. Custom configuration (only key events):");
    let custom = LogConfig {
        enabled: true,
        log_file: PathBuf::from("custom_debug.log"),
        clear_on_startup: false, // Append to existing log
        features: LogFeatures {
            modal_state: false,
            key_events: true, // Only key events
            rendering: false,
            settings: false,
        },
        level: LevelFilter::Debug,
    };
    println!("   enabled: {}", custom.enabled);
    println!("   log_file: {:?}", custom.log_file);
    println!("   clear_on_startup: {}", custom.clear_on_startup);
    println!("   features.key_events: {}", custom.features.key_events);
    println!("   features.rendering: {}\n", custom.features.rendering);

    println!("To use any of these configurations in your app:");
    println!("  let log_config = logging::LogConfig::default(); // or disabled(), verbose()");
    println!("  logging::init_logging(&log_config)?;");
    println!("\nSee fido/LOGGING.md for complete documentation");
}
