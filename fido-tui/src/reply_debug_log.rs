use std::fs::OpenOptions;
use std::io::Write;

pub fn log_reply_event(message: &str) {
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open("fido_reply_debug.log");

    if let Ok(ref mut f) = log {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(f, "[{}] {}", timestamp, message);
        let _ = f.flush();
    }
}

#[macro_export]
macro_rules! log_reply {
    ($($arg:tt)*) => {{
        $crate::reply_debug_log::log_reply_event(&format!($($arg)*));
    }};
}
