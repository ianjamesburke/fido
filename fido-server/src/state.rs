use crate::db::repositories::Repositories;
use crate::db::Database;
use crate::security::AuditLogger;
use crate::session::SessionManager;

#[derive(Clone)]
pub struct AppState {
    #[allow(dead_code)]
    pub db: Database,
    pub repos: Repositories,
    pub session_manager: SessionManager,
    pub audit_logger: AuditLogger,
}

impl AppState {
    #[allow(dead_code)]
    pub fn new(db: Database) -> Self {
        let repos = Repositories::new(db.pool.clone());
        Self::new_with_repos(db, repos)
    }

    pub fn new_with_repos(db: Database, repos: Repositories) -> Self {
        let session_manager = SessionManager::new(repos.sessions.clone());
        let audit_logger = AuditLogger::new(repos.audit.clone());
        Self {
            db,
            repos,
            session_manager,
            audit_logger,
        }
    }

    /// Get authenticated user ID from session token
    pub fn get_authenticated_user_id_from_token(&self, token: &str) -> Option<uuid::Uuid> {
        self.session_manager.validate_session(token).ok()
    }
}
