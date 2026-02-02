use std::sync::Arc;

use crate::db::Database;
use crate::security::AuditLogger;
use crate::session::SessionManager;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub session_manager: SessionManager,
    pub audit_logger: AuditLogger,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        let session_manager = SessionManager::new(db.clone());
        let audit_logger = AuditLogger::new(Arc::new(db.pool.clone()));
        Self {
            db,
            session_manager,
            audit_logger,
        }
    }

    /// Get authenticated user ID from session token
    pub fn get_authenticated_user_id_from_token(&self, token: &str) -> Option<uuid::Uuid> {
        self.session_manager.validate_session(token).ok()
    }
}
