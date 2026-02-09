#!/usr/bin/env bash
set -euo pipefail

PROJECT_ID="${FIREBASE_PROJECT_ID:-${1:-}}"
REGION="${REGION:-us-central1}"
SERVICE_NAME="${SERVICE_NAME:-fido-web}"
ALLOWED_ORIGINS="${ALLOWED_ORIGINS:-https://${PROJECT_ID}.web.app,https://${PROJECT_ID}.firebaseapp.com}"

if [ -z "$PROJECT_ID" ]; then
  echo "Usage: FIREBASE_PROJECT_ID=<project-id> $0"
  echo "   or: $0 <project-id>"
  exit 1
fi

if ! command -v gcloud >/dev/null 2>&1; then
  echo "gcloud CLI is required: https://cloud.google.com/sdk/docs/install"
  exit 1
fi

if ! command -v firebase >/dev/null 2>&1; then
  echo "Firebase CLI is required: https://firebase.google.com/docs/cli"
  exit 1
fi

if [ -z "${GITHUB_CLIENT_ID:-}" ]; then
  echo "GITHUB_CLIENT_ID is not set; using placeholder value 'demo-client-id'."
  echo "GitHub OAuth login will not work until you set a real client ID."
  GITHUB_CLIENT_ID="demo-client-id"
fi

echo "==> Enabling required Google APIs (idempotent)"
gcloud services enable \
  run.googleapis.com \
  cloudbuild.googleapis.com \
  artifactregistry.googleapis.com \
  firebasehosting.googleapis.com \
  --project "$PROJECT_ID"

echo "==> Deploying Cloud Run service '${SERVICE_NAME}' in ${REGION}"
gcloud run deploy "$SERVICE_NAME" \
  --source . \
  --project "$PROJECT_ID" \
  --region "$REGION" \
  --allow-unauthenticated \
  --port 8080 \
  --min-instances 0 \
  --max-instances 3 \
  --set-env-vars "HOST=0.0.0.0,FIDO_SERVER_PORT=3000,NGINX_PORT=8080,TTYD_PORT=7681,DATABASE_PATH=/tmp/fido.db,RUST_LOG=info,GITHUB_CLIENT_ID=${GITHUB_CLIENT_ID},ALLOWED_ORIGINS=${ALLOWED_ORIGINS},ENVIRONMENT=production"

echo "==> Deploying Firebase Hosting rewrite"
firebase deploy --only hosting --project "$PROJECT_ID"

echo "Deploy complete."
