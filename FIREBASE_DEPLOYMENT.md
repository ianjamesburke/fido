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
1. Build and deploy the container to Cloud Run service `fido-web` in `us-central1`
2. Deploy Firebase Hosting rewrite rules from `firebase.json`

## Local smoke test before deploy

```bash
./start.sh
# open http://localhost:8080
```

## Important operational notes

- Cloud Run injects `PORT` for the external listener. The startup script now keeps
  nginx on `PORT` and fido-server on internal `3000`.
- CORS origins are configured automatically for `https://<project>.web.app` and
  `https://<project>.firebaseapp.com`. Override with `ALLOWED_ORIGINS` if needed.
- `DATABASE_PATH` is set to `/tmp/fido.db` in the deploy script, which is ephemeral.
  Data will not persist across instance restarts.
- For persistent production data, migrate off SQLite file storage (for example,
  use a managed database such as Cloud SQL/Postgres).

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
