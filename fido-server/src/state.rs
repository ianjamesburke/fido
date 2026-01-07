use crate::db::Database;
use crate::session::SessionManager;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub session_manager: SessionManager,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        let session_manager = SessionManager::new(db.clone());
        Self {
            db,
            session_manager,
        }
    }

    /// Get authenticated user ID from session token
    pub fn get_authenticated_user_id_from_token(&self, token: &str) -> Option<uuid::Uuid> {
        self.session_manager.validate_session(token).ok()
    }
}
