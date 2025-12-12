use fido::debug_log;
use std::fs;
use std::path::Path;

#[test]
fn test_debug_log_creation() {
    // This test verifies that the debug log file can be created
    let log_file = "test_fido_modal_debug.log";

    // Clean up any existing test log
    let _ = fs::remove_file(log_file);

    // Create a new log file
    let result = fs::File::create(log_file);
    assert!(result.is_ok(), "Should be able to create log file");

    // Verify file exists
    assert!(Path::new(log_file).exists(), "Log file should exist");

    // Clean up
    let _ = fs::remove_file(log_file);
}

#[test]
fn test_log_file_constant() {
    // Verify the constant is set correctly
    assert_eq!(debug_log::DEBUG_LOG_FILE, "fido_modal_debug.log");
}

#[test]
fn test_clear_debug_log() {
    // Test that clear_debug_log creates/clears the log file
    debug_log::clear_debug_log();

    // Verify file exists and is empty
    assert!(
        Path::new(debug_log::DEBUG_LOG_FILE).exists(),
        "Log file should exist after clear"
    );

    let metadata =
        fs::metadata(debug_log::DEBUG_LOG_FILE).expect("Should be able to read metadata");
    assert_eq!(metadata.len(), 0, "Log file should be empty after clear");

    // Note: Don't clean up to avoid interfering with parallel tests
}

#[test]
fn test_log_modal_state() {
    // Clear the log first
    debug_log::clear_debug_log();

    // Log some modal state
    debug_log::log_modal_state(true, true, false, "Reply");

    // Read the log file
    let contents =
        fs::read_to_string(debug_log::DEBUG_LOG_FILE).expect("Should be able to read log file");

    // Verify the log contains expected information
    assert!(
        contents.contains("MODAL_STATE"),
        "Log should contain MODAL_STATE"
    );
    assert!(
        contents.contains("viewing_post_detail=true"),
        "Log should contain viewing_post_detail=true"
    );
    assert!(
        contents.contains("show_full_post_modal=true"),
        "Log should contain show_full_post_modal=true"
    );
    assert!(
        contents.contains("composer_open=false"),
        "Log should contain composer_open=false"
    );
    assert!(
        contents.contains("composer_mode=Reply"),
        "Log should contain composer_mode=Reply"
    );

    // Note: Don't clean up to avoid interfering with parallel tests
}

#[test]
fn test_log_key_event() {
    // Clear the log first
    debug_log::clear_debug_log();

    // Log a key event
    debug_log::log_key_event("Enter", "composer_open");

    // Read the log file
    let contents =
        fs::read_to_string(debug_log::DEBUG_LOG_FILE).expect("Should be able to read log file");

    // Verify the log contains expected information
    assert!(
        contents.contains("KEY_EVENT"),
        "Log should contain KEY_EVENT"
    );
    assert!(
        contents.contains("key=Enter"),
        "Log should contain key=Enter"
    );
    assert!(
        contents.contains("context=composer_open"),
        "Log should contain context=composer_open"
    );

    // Note: Don't clean up to avoid interfering with parallel tests
}

#[test]
fn test_log_debug() {
    // Clear the log first
    debug_log::clear_debug_log();

    // Log a custom message
    debug_log::log_debug("Test debug message");

    // Read the log file
    let contents =
        fs::read_to_string(debug_log::DEBUG_LOG_FILE).expect("Should be able to read log file");

    // Verify the log contains the message
    assert!(
        contents.contains("Test debug message"),
        "Log should contain the debug message"
    );

    // Note: Don't clean up to avoid interfering with parallel tests
}
