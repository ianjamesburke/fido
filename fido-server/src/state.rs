use crate::db::repositories::Repositories;
use crate::db::Database;
use crate::events::SharedEventBus;
use crate::realtime::RealtimeGateway;
use crate::security::AuditLogger;
use crate::services::github::GithubService;
use crate::session::SessionManager;
use anyhow::Result;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    #[allow(dead_code)]
    pub db: Database,
    pub repos: Repositories,
    pub session_manager: SessionManager,
    pub audit_logger: AuditLogger,
    pub github_service: GithubService,
    pub event_bus: SharedEventBus,
    pub realtime: Arc<RealtimeGateway>,
}

impl AppState {
    #[allow(dead_code)]
    pub fn new(db: Database) -> Result<Self> {
        let repos = Repositories::new(db.pool.clone());
        Self::new_with_repos(db, repos)
    }

    pub fn new_with_repos(db: Database, repos: Repositories) -> Result<Self> {
        let session_manager = SessionManager::new(repos.sessions.clone());
        let audit_logger = AuditLogger::new(repos.audit.clone());
        let github_service = GithubService::from_env(repos.clone())?;
        let realtime = Arc::new(RealtimeGateway::new(repos.clone()));
        let event_bus: SharedEventBus = realtime.clone();
        Ok(Self {
            db,
            repos,
            session_manager,
            audit_logger,
            github_service,
            event_bus,
            realtime,
        })
    }

    /// Get authenticated user ID from session token
    pub fn get_authenticated_user_id_from_token(&self, token: &str) -> Option<uuid::Uuid> {
        self.session_manager.validate_session(token).ok()
    }
}
