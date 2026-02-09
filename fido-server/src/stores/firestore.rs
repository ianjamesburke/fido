//! Firestore store scaffold.
//!
//! This module is intentionally a placeholder. The store trait surface in
//! `stores/mod.rs` is the contract that Firestore implementations should satisfy.

use anyhow::{bail, Result};

/// Validate that Firestore backend prerequisites are present.
///
/// Full Firestore store implementations are not wired yet.
pub fn validate_firestore_env() -> Result<()> {
    let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")
        .or_else(|_| std::env::var("FIREBASE_PROJECT_ID"))
        .ok();

    if project_id.is_none() {
        bail!("Firestore backend selected, but GOOGLE_CLOUD_PROJECT/FIREBASE_PROJECT_ID is not set")
    }

    Ok(())
}
