set dotenv-load

# Start Firebase emulators (Firestore + Emulator UI)
emulator project_id="demo-fido":
    firebase emulators:start --project {{project_id}}

# Alias for emulator
emu project_id="demo-fido":
    just emulator {{project_id}}

# Start the Fido server against the local Firestore emulator
# Note: auto-seeding runs on startup when the emulator is empty.
server project_id="demo-fido" emulator_host="127.0.0.1:8088":
    DB_BACKEND=firestore \
    FIREBASE_PROJECT_ID={{project_id}} \
    GOOGLE_CLOUD_PROJECT={{project_id}} \
    FIRESTORE_EMULATOR_HOST={{emulator_host}} \
    FIRESTORE_SEED_TEST_DATA=true \
    cargo run --bin fido-server

# Start the Fido server with fresh database (deletes and recreates)
server-reset:
    cargo run --bin fido-server -- --reset-db

# Start the Fido TUI client
tui:
    cargo run --bin fido

# Start TUI connected to local server
tui-local:
    cargo run --bin fido -- --server http://localhost:3000

# Start full local web stack (fido-server + ttyd + nginx)
# Uses Firestore emulator by default and enables test-data seeding.
web project_id="demo-fido" emulator_host="127.0.0.1:8088":
    DB_BACKEND=firestore \
    FIREBASE_PROJECT_ID={{project_id}} \
    GOOGLE_CLOUD_PROJECT={{project_id}} \
    FIRESTORE_EMULATOR_HOST={{emulator_host}} \
    FIRESTORE_SEED_TEST_DATA=true \
    ./start.sh

# Run full test suite
test:
    cargo test --workspace

# Deploy web stack to Firebase Hosting + Cloud Run
firebase-deploy project_id:
    FIREBASE_PROJECT_ID={{project_id}} ./scripts/deploy-firebase.sh

# Deploy web stack using env/.env and active gcloud project defaults
deploy:
    ./scripts/deploy-firebase.sh

# Publish crates to crates.io in dependency order (no auto-bump).
# Usage:
#   just deploy-cargo
#   just deploy-cargo dry-run=true
deploy-cargo dry-run="false":
    chmod +x ./scripts/deploy-cargo.sh
    if [ "{{dry-run}}" = "true" ]; then ./scripts/deploy-cargo.sh --dry-run; else ./scripts/deploy-cargo.sh; fi

# Run Firestore emulator smoke test
firestore-emulator-check project_id="demo-fido":
    FIREBASE_PROJECT_ID={{project_id}} ./scripts/run-firestore-emulator-check.sh
