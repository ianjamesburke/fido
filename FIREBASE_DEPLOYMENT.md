# Firebase Deployment (Hosting + Cloud Run)

This repo now supports Firebase deployment by running the existing Dockerized stack
(nginx + ttyd + fido-server) on **Cloud Run**, then routing traffic through
**Firebase Hosting**.

## Why this shape

Firebase Hosting cannot run Rust binaries directly. Cloud Run runs the container,
and Hosting provides a Firebase-managed URL/domain in front of it.

## Prerequisites

- `gcloud` CLI installed and authenticated
- `firebase` CLI installed and authenticated
- A Firebase project (with billing enabled for Cloud Run)
- `GITHUB_CLIENT_ID` set for OAuth in production

## One-command deploy

```bash
export FIREBASE_PROJECT_ID=your-project-id
export GITHUB_CLIENT_ID=your-github-oauth-client-id
./scripts/deploy-firebase.sh
```

This will:
1. Enable required APIs (`run`, `cloudbuild`, `artifactregistry`, `firebasehosting`, `firestore`)
2. Build and deploy the container to Cloud Run service `fido-web` in `us-central1`
3. Deploy Firestore rules/indexes from `firestore.rules` + `firestore.indexes.json`
4. Deploy Firebase Hosting rewrite rules from `firebase.json`
5. Run post-deploy health checks (`/health`, `/users/test`)

## Local smoke test before deploy

```bash
./start.sh
# open http://localhost:8080
```

## SQLite to Firestore migration

Run a dry-run first:

```bash
cargo run -p fido-migrate --bin sqlite-to-firestore -- \
  --sqlite-path ./fido.db \
  --project-id demo-fido \
  --emulator-host 127.0.0.1:8088 \
  --dry-run
```

Run actual migration with count validation:

```bash
cargo run -p fido-migrate --bin sqlite-to-firestore -- \
  --sqlite-path ./fido.db \
  --project-id demo-fido \
  --emulator-host 127.0.0.1:8088 \
  --validate
```

The tool migrates users, posts/replies, hashtags/follows/activity, votes, follows,
DMs, user configs, rate limits, sessions, and audit rows.

## Important operational notes

- Cloud Run injects `PORT` for the external listener. The startup script now keeps
  nginx on `PORT` and fido-server on internal `3000`.
- CORS origins are configured automatically for `https://<project>.web.app` and
  `https://<project>.firebaseapp.com`. Override with `ALLOWED_ORIGINS` if needed.
- `DB_BACKEND=firestore` is set in the deploy script so runtime data is stored in Firestore.
- `GOOGLE_CLOUD_PROJECT` / `FIREBASE_PROJECT_ID` are injected into Cloud Run at deploy time.
- For production auth, prefer Cloud Run ADC metadata tokens over static service-account keys.

## Rollback strategy

1. Keep `DB_BACKEND=sqlite` in staging/prod environment until Firestore validation passes.
2. Take a SQLite backup before cutover (`cp fido.db fido-pre-firestore.db`).
3. If post-cutover checks fail, revert `DB_BACKEND` to `sqlite` and redeploy.
4. Investigate migration diffs with `sqlite-to-firestore --dry-run --validate` against emulator.

## crates.io client target URL

For installed `fido` clients, set a persistent server URL once:

```bash
mkdir -p ~/.fido
echo "https://your-project.web.app" > ~/.fido/server_url
```

Or per-run:

```bash
FIDO_SERVER_URL=https://your-project.web.app fido
```

## Updating service name or region

```bash
SERVICE_NAME=fido-web REGION=us-central1 ./scripts/deploy-firebase.sh your-project-id
```

## Optional deploy flags

```bash
# Skip Firestore rules/indexes deploy
DEPLOY_FIRESTORE=0 ./scripts/deploy-firebase.sh

# Skip Hosting deploy
DEPLOY_HOSTING=0 ./scripts/deploy-firebase.sh

# Skip post-deploy HTTP checks
RUN_HEALTH_CHECKS=0 ./scripts/deploy-firebase.sh
```
