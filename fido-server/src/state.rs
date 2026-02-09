use crate::db::Database;
use crate::security::AuditLogger;
use crate::session::SessionManager;
use crate::stores::Stores;

#[derive(Clone)]
pub struct AppState {
    #[allow(dead_code)]
    pub db: Database,
    pub stores: Stores,
    pub session_manager: SessionManager,
    pub audit_logger: AuditLogger,
}

impl AppState {
    #[allow(dead_code)]
    pub fn new(db: Database) -> Self {
        let stores = Stores::sqlite(db.pool.clone());
        Self::new_with_stores(db, stores)
    }

    pub fn new_with_stores(db: Database, stores: Stores) -> Self {
        let session_manager = SessionManager::new(stores.sessions.clone());
        let audit_logger = AuditLogger::new(stores.audit.clone());
        Self {
            db,
            stores,
            session_manager,
            audit_logger,
        }
    }

    /// Get authenticated user ID from session token
    pub fn get_authenticated_user_id_from_token(&self, token: &str) -> Option<uuid::Uuid> {
        self.session_manager.validate_session(token).ok()
    }
}
