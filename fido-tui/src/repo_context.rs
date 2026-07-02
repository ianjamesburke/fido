//! Detects the GitHub repo the TUI was launched inside.
//!
//! The launch directory decides the community: if the working directory is
//! inside a git repo whose `origin` remote points at GitHub, the TUI opens
//! straight onto that repo's community. Anything else (no repo, no origin,
//! non-GitHub remote) is Home mode — a valid state, not an error.

use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRef {
    pub owner: String,
    pub name: String,
}

impl RepoRef {
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

/// Resolve the GitHub repo for `cwd`, walking up to the nearest git repo.
pub fn detect_repo_context(cwd: &Path) -> Option<RepoRef> {
    let toplevel = git_stdout(cwd, &["rev-parse", "--show-toplevel"])?;
    let origin_url = git_stdout(Path::new(&toplevel), &["remote", "get-url", "origin"])?;

    match parse_github_remote(&origin_url) {
        Some(repo) => {
            log::info!(
                "Launch context: {} (from origin remote in {})",
                repo.full_name(),
                toplevel
            );
            Some(repo)
        }
        None => {
            log::info!(
                "Launch context: origin remote '{}' is not a GitHub repo; using Home mode",
                origin_url
            );
            None
        }
    }
}

fn git_stdout(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| log::info!("git {:?} failed to spawn: {}; using Home mode", args, e))
        .ok()?;

    if !output.status.success() {
        log::info!(
            "git {:?} exited {} in {}; using Home mode",
            args,
            output.status,
            dir.display()
        );
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return None;
    }
    Some(stdout)
}

/// Parse `owner/name` out of a GitHub remote URL.
///
/// Handles the forms git actually produces:
/// - `https://github.com/owner/name(.git)`
/// - `git@github.com:owner/name(.git)`
/// - `ssh://git@github.com/owner/name(.git)`
/// - `git://github.com/owner/name(.git)`
pub fn parse_github_remote(url: &str) -> Option<RepoRef> {
    let url = url.trim();

    let path = if let Some(rest) = url.strip_prefix("git@github.com:") {
        rest
    } else {
        let after_scheme = url.split_once("://").map(|(_, rest)| rest)?;
        let after_scheme = after_scheme.strip_prefix("git@").unwrap_or(after_scheme);
        after_scheme.strip_prefix("github.com/")?
    };

    let path = path.strip_suffix(".git").unwrap_or(path);
    let path = path.strip_suffix('/').unwrap_or(path);

    let (owner, name) = path.split_once('/')?;
    if owner.is_empty() || name.is_empty() || name.contains('/') {
        return None;
    }

    Some(RepoRef {
        owner: owner.to_string(),
        name: name.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(owner: &str, name: &str) -> Option<RepoRef> {
        Some(RepoRef {
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }

    #[test]
    fn parses_github_remote_url_forms() {
        let cases = [
            (
                "https://github.com/ianjamesburke/fido.git",
                repo("ianjamesburke", "fido"),
            ),
            (
                "https://github.com/ianjamesburke/fido",
                repo("ianjamesburke", "fido"),
            ),
            (
                "https://github.com/rust-lang/rust/",
                repo("rust-lang", "rust"),
            ),
            (
                "git@github.com:octocat/Hello-World.git",
                repo("octocat", "Hello-World"),
            ),
            (
                "git@github.com:octocat/Hello-World",
                repo("octocat", "Hello-World"),
            ),
            ("ssh://git@github.com/owner/name.git", repo("owner", "name")),
            ("git://github.com/owner/name.git", repo("owner", "name")),
            (
                "  https://github.com/owner/name.git\n",
                repo("owner", "name"),
            ),
            // Not GitHub / not parseable
            ("https://gitlab.com/owner/name.git", None),
            ("git@gitlab.com:owner/name.git", None),
            ("https://github.com/owner", None),
            ("https://github.com/", None),
            ("not-a-url", None),
            ("", None),
        ];

        for (url, expected) in cases {
            assert_eq!(parse_github_remote(url), expected, "url: {:?}", url);
        }
    }

    #[test]
    fn detects_none_outside_a_repo() {
        let dir = std::env::temp_dir().join(format!("fido-no-repo-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(detect_repo_context(&dir), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detects_repo_from_nested_directory() {
        let dir = std::env::temp_dir().join(format!("fido-repo-{}", uuid::Uuid::new_v4()));
        let nested = dir.join("a/b");
        std::fs::create_dir_all(&nested).unwrap();

        let run = |args: &[&str]| {
            let status = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .status()
                .unwrap();
            assert!(status.success(), "git {:?}", args);
        };
        run(&["init", "-q"]);
        run(&[
            "remote",
            "add",
            "origin",
            "git@github.com:testowner/testrepo.git",
        ]);

        assert_eq!(detect_repo_context(&nested), repo("testowner", "testrepo"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
