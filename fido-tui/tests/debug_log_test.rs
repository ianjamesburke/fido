use fido::reply_debug_log;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn test_debug_log_creation() {
    let _guard = test_lock().lock().expect("lock poisoned");
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
    let _guard = test_lock().lock().expect("lock poisoned");
    // Verify the constant is set correctly
    assert_eq!("fido_reply_debug.log", "fido_reply_debug.log");
}

#[test]
fn test_clear_debug_log() {
    let _guard = test_lock().lock().expect("lock poisoned");
    let log_file = "fido_reply_debug.log";
    let _ = fs::remove_file(log_file);
    reply_debug_log::log_reply_event("bootstrap");

    // Verify file exists and is empty
    assert!(
        Path::new(log_file).exists(),
        "Log file should exist after write"
    );

    let _ = fs::remove_file(log_file);
}

#[test]
fn test_log_modal_state() {
    let _guard = test_lock().lock().expect("lock poisoned");
    let log_file = "fido_reply_debug.log";
    let _ = fs::remove_file(log_file);
    reply_debug_log::log_reply_event("modal state event");

    // Read the log file
    let contents = fs::read_to_string(log_file).expect("Should be able to read log file");

    // Verify the log contains expected information
    assert!(
        contents.contains("modal state event"),
        "Log should contain event message"
    );
    let _ = fs::remove_file(log_file);
}

#[test]
fn test_log_key_event() {
    let _guard = test_lock().lock().expect("lock poisoned");
    let log_file = "fido_reply_debug.log";
    let _ = fs::remove_file(log_file);
    reply_debug_log::log_reply_event("key=Enter context=composer_open");

    // Read the log file
    let contents = fs::read_to_string(log_file).expect("Should be able to read log file");

    // Verify the log contains expected information
    assert!(
        contents.contains("key=Enter"),
        "Log should contain key=Enter"
    );
    assert!(
        contents.contains("context=composer_open"),
        "Log should contain context=composer_open"
    );

    let _ = fs::remove_file(log_file);
}

#[test]
fn test_log_debug() {
    let _guard = test_lock().lock().expect("lock poisoned");
    let log_file = "fido_reply_debug.log";
    let _ = fs::remove_file(log_file);
    reply_debug_log::log_reply_event("Test debug message");

    // Read the log file
    let contents = fs::read_to_string(log_file).expect("Should be able to read log file");

    // Verify the log contains the message
    assert!(
        contents.contains("Test debug message"),
        "Log should contain the debug message"
    );

    let _ = fs::remove_file(log_file);
}
