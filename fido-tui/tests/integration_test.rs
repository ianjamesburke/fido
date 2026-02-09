use fido::logging::{init_logging, LogConfig, LogFeatures};
use log::LevelFilter;
use std::fs;
use std::path::Path;

#[test]
fn test_logging_integration() {
    let log_file = "fido_integration_test.log";
    let config = LogConfig {
        enabled: true,
        log_file: log_file.into(),
        clear_on_startup: true,
        features: LogFeatures {
            modal_state: true,
            key_events: true,
            rendering: true,
            settings: true,
        },
        level: LevelFilter::Debug,
    };

    init_logging(&config).expect("logging should initialize");

    log::debug!(target: "modal_state", "modal changed");
    log::debug!(target: "key_events", "pressed key");
    log::debug!(target: "rendering", "rendering frame");
    log::debug!(target: "settings", "settings updated");

    assert!(Path::new(log_file).exists(), "Log file should exist");

    let contents = fs::read_to_string(log_file).expect("Should be able to read log file");

    // Verify all log entries are present
    assert!(
        contents.contains("modal changed"),
        "Should contain modal state log"
    );
    assert!(
        contents.contains("pressed key"),
        "Should contain key event log"
    );
    assert!(
        contents.contains("rendering frame"),
        "Should contain render start log"
    );
    assert!(
        contents.contains("settings updated"),
        "Should contain thread modal log"
    );

    // Clean up
    let _ = fs::remove_file(log_file);
}
