use fido::debug_log;
use std::fs;
use std::path::Path;

#[test]
fn test_logging_integration() {
    // This test simulates the application startup sequence

    // Step 1: Clear debug log (as done in main.rs)
    debug_log::clear_debug_log();

    // Verify log file exists and is empty
    assert!(
        Path::new(debug_log::DEBUG_LOG_FILE).exists(),
        "Log file should exist"
    );
    let metadata = fs::metadata(debug_log::DEBUG_LOG_FILE).expect("Should read metadata");
    assert_eq!(metadata.len(), 0, "Log file should be empty after clear");

    // Step 2: Simulate modal state logging (as done in tabs.rs)
    debug_log::log_modal_state(false, false, false, "None");

    // Step 3: Simulate key event logging (as done in main.rs)
    debug_log::log_key_event("Char('r')", "main_view");

    // Step 4: Simulate debug logging (as done in tabs.rs)
    debug_log::log_debug("render_posts_tab_with_data: START");
    debug_log::log_debug("Rendering thread modal (full post modal)");

    // Step 5: Read and verify log contents
    let contents =
        fs::read_to_string(debug_log::DEBUG_LOG_FILE).expect("Should be able to read log file");

    // Verify all log entries are present
    assert!(
        contents.contains("MODAL_STATE"),
        "Should contain modal state log"
    );
    assert!(
        contents.contains("KEY_EVENT"),
        "Should contain key event log"
    );
    assert!(
        contents.contains("render_posts_tab_with_data: START"),
        "Should contain render start log"
    );
    assert!(
        contents.contains("Rendering thread modal"),
        "Should contain thread modal log"
    );

    // Verify timestamps are present (format: [YYYY-MM-DD HH:MM:SS.mmm])
    assert!(contents.contains("[20"), "Should contain timestamp");

    // Count number of log lines (should be 4)
    let line_count = contents.lines().count();
    assert_eq!(line_count, 4, "Should have 4 log lines");

    // Clean up
    let _ = fs::remove_file(debug_log::DEBUG_LOG_FILE);
}
