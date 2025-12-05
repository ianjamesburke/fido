use std::fs::{File, OpenOptions};
use std::io::Write;
use chrono::Local;

/// Log file path constant
pub const DEBUG_LOG_FILE: &str = "fido_modal_debug.log";

/// Clear the debug log file (call on application startup)
pub fn clear_debug_log() {
    match File::create(DEBUG_LOG_FILE) {
        Ok(mut file) => {
            // Explicitly flush to ensure file is truncated
            let _ = file.flush();
        }
        Err(e) => {
            eprintln!("Failed to clear debug log: {}", e);
        }
    }
}

/// Append a message to the debug log file
fn append_to_log(message: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let log_line = format!("[{}] {}\n", timestamp, message);
    
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(DEBUG_LOG_FILE)
    {
        let _ = file.write_all(log_line.as_bytes());
    }
}

/// Log modal state information
pub fn log_modal_state(
    viewing_post_detail: bool,
    show_full_post_modal: bool,
    composer_open: bool,
    composer_mode: &str,
) {
    let message = format!(
        "MODAL_STATE: viewing_post_detail={}, show_full_post_modal={}, composer_open={}, composer_mode={}",
        viewing_post_detail, show_full_post_modal, composer_open, composer_mode
    );
    append_to_log(&message);
}

/// Log key event information
pub fn log_key_event(key_code: &str, modal_context: &str) {
    let message = format!(
        "KEY_EVENT: key={}, context={}",
        key_code, modal_context
    );
    append_to_log(&message);
}

/// Log a custom debug message
pub fn log_debug(message: &str) {
    append_to_log(message);
}
