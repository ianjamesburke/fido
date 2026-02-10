use fido_server::db::Database;
use fido_server::stores::Stores;

#[test]
fn firestore_stores_roundtrip_with_emulator() {
    let emulator_host = std::env::var("FIRESTORE_EMULATOR_HOST").ok();
    let project_id = std::env::var("FIREBASE_PROJECT_ID")
        .ok()
        .or_else(|| std::env::var("GOOGLE_CLOUD_PROJECT").ok());

    if emulator_host.is_none() || project_id.is_none() {
        eprintln!("Skipping Firestore emulator integration test; set FIRESTORE_EMULATOR_HOST and FIREBASE_PROJECT_ID");
        return;
    }

    std::env::set_var("DB_BACKEND", "firestore");

    let db = Database::in_memory().expect("failed to create in-memory db for pool");
    let stores = Stores::from_env(db.pool.clone()).expect("failed to initialize firestore stores");

    let user = stores
        .users
        .create_or_update_from_github(987_654_321, "firestore_ci_user", Some("firestore_ci_user"))
        .expect("failed to create user via firestore store");

    let loaded = stores
        .users
        .get_by_id(&user.id)
        .expect("failed to fetch user by id")
        .expect("user should exist");

    assert_eq!(loaded.username, "firestore_ci_user");

    let config = stores
        .config
        .get(&user.id)
        .expect("failed to load config default");
    assert_eq!(config.user_id, user.id);

    let session_token = format!("test-token-{}", user.id);
    let now = chrono::Utc::now();
    stores
        .sessions
        .create_session(
            &session_token,
            user.id,
            now,
            now + chrono::Duration::hours(1),
            now,
        )
        .expect("failed to create session");

    let session = stores
        .sessions
        .get_session(&session_token)
        .expect("failed to get session")
        .expect("session missing");
    assert_eq!(session.user_id, user.id);

    stores
        .sessions
        .delete_session(&session_token)
        .expect("failed to delete session");
}
