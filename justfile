set dotenv-load

# Start the Fido server
server:
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

# Run full test suite
test:
    cargo test --workspace

# Deploy web stack to Firebase Hosting + Cloud Run
firebase-deploy project_id:
    FIREBASE_PROJECT_ID={{project_id}} ./scripts/deploy-firebase.sh

# Run Firestore emulator smoke test
firestore-emulator-check project_id="demo-fido":
    FIREBASE_PROJECT_ID={{project_id}} ./scripts/run-firestore-emulator-check.sh
