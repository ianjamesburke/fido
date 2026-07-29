use std::fs;
use std::io;
use std::path::Path;

const REPLY_DEBUG_LOG: &str = "fido_reply_debug.log";

pub(super) fn remove_reply_debug_log(launch_dir: &Path) -> io::Result<()> {
    match fs::remove_file(launch_dir.join(REPLY_DEBUG_LOG)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn remove_reply_debug_log_deletes_legacy_file_only() {
        let launch_dir = tempdir().unwrap();
        let legacy_log = launch_dir.path().join(REPLY_DEBUG_LOG);
        let neighboring_log = launch_dir.path().join("keep.log");
        fs::write(&legacy_log, "legacy reply trace").unwrap();
        fs::write(&neighboring_log, "keep").unwrap();

        remove_reply_debug_log(launch_dir.path()).unwrap();

        assert_eq!(
            (legacy_log.exists(), neighboring_log.exists()),
            (false, true)
        );
    }

    #[test]
    fn remove_reply_debug_log_accepts_missing_legacy_file() {
        let launch_dir = tempdir().unwrap();

        let result = remove_reply_debug_log(launch_dir.path());

        assert!(result.is_ok(), "cleanup failed: {result:?}");
    }
}
