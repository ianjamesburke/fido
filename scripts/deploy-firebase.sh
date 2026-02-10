#!/usr/bin/env bash
set -euo pipefail

# Load .env automatically when present
if [ -f ".env" ]; then
  # shellcheck disable=SC1091
  set -a
  source ".env"
  set +a
fi

PROJECT_ID="${FIREBASE_PROJECT_ID:-${1:-}}"
if [ -z "${PROJECT_ID}" ] && command -v gcloud >/dev/null 2>&1; then
  PROJECT_ID="$(gcloud config get-value core/project 2>/dev/null || true)"
  PROJECT_ID="${PROJECT_ID//$'\n'/}"
fi

REGION="${REGION:-us-central1}"
SERVICE_NAME="${SERVICE_NAME:-fido-web}"
ALLOWED_ORIGINS="${ALLOWED_ORIGINS:-https://${PROJECT_ID}.web.app,https://${PROJECT_ID}.firebaseapp.com}"
DEPLOY_FIRESTORE="${DEPLOY_FIRESTORE:-1}"
DEPLOY_HOSTING="${DEPLOY_HOSTING:-1}"
RUN_HEALTH_CHECKS="${RUN_HEALTH_CHECKS:-1}"
SEED_EMULATOR_DATA="${FIRESTORE_SEED_TEST_DATA:-false}"

if [ -z "$PROJECT_ID" ]; then
  echo "Usage: FIREBASE_PROJECT_ID=<project-id> $0"
  echo "   or: $0 <project-id>"
  echo "   or: set an active gcloud project first"
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

if ! gcloud auth list --filter=status:ACTIVE --format="value(account)" | grep -q .; then
  echo "No active gcloud account. Run: gcloud auth login"
  exit 1
fi

if ! firebase projects:list >/dev/null 2>&1; then
  echo "Firebase CLI is not authenticated. Run: firebase login"
  exit 1
fi

if [ -z "${GITHUB_CLIENT_ID:-}" ]; then
  echo "GITHUB_CLIENT_ID is not set; using placeholder value 'demo-client-id'."
  echo "GitHub OAuth login will not work until you set a real client ID."
  GITHUB_CLIENT_ID="demo-client-id"
fi

echo "==> Setting gcloud active project"
gcloud config set project "$PROJECT_ID" >/dev/null

echo "==> Enabling required Google APIs (idempotent)"
gcloud services enable \
  run.googleapis.com \
  cloudbuild.googleapis.com \
  artifactregistry.googleapis.com \
  firebasehosting.googleapis.com \
  firestore.googleapis.com \
  --project "$PROJECT_ID"

echo "==> Deploying Cloud Run service '${SERVICE_NAME}' in ${REGION}"

ENV_VARS_FILE="$(mktemp /tmp/fido-cloudrun-env.XXXXXX.yaml)"
trap 'rm -f "$ENV_VARS_FILE"' EXIT
cat >"$ENV_VARS_FILE" <<EOF
HOST: "0.0.0.0"
FIDO_SERVER_PORT: "3000"
NGINX_PORT: "8080"
TTYD_PORT: "7681"
DB_BACKEND: "firestore"
GOOGLE_CLOUD_PROJECT: "${PROJECT_ID}"
FIREBASE_PROJECT_ID: "${PROJECT_ID}"
RUST_LOG: "info"
GITHUB_CLIENT_ID: "${GITHUB_CLIENT_ID}"
ALLOWED_ORIGINS: "${ALLOWED_ORIGINS}"
ENVIRONMENT: "production"
FIRESTORE_SEED_TEST_DATA: "${SEED_EMULATOR_DATA}"
EOF

gcloud run deploy "$SERVICE_NAME" \
  --source . \
  --project "$PROJECT_ID" \
  --region "$REGION" \
  --allow-unauthenticated \
  --port 8080 \
  --min-instances 0 \
  --max-instances 3 \
  --env-vars-file "$ENV_VARS_FILE"

if [ "$DEPLOY_FIRESTORE" = "1" ]; then
  echo "==> Deploying Firestore rules and indexes"
  firebase deploy --only firestore:rules,firestore:indexes --project "$PROJECT_ID"
else
  echo "==> Skipping Firestore rules/index deploy (DEPLOY_FIRESTORE=${DEPLOY_FIRESTORE})"
fi

if [ "$DEPLOY_HOSTING" = "1" ]; then
  echo "==> Deploying Firebase Hosting rewrite"
  firebase deploy --only hosting --project "$PROJECT_ID"
else
  echo "==> Skipping Firebase Hosting deploy (DEPLOY_HOSTING=${DEPLOY_HOSTING})"
fi

WEB_URL="https://${PROJECT_ID}.web.app"
FIREBASEAPP_URL="https://${PROJECT_ID}.firebaseapp.com"

if [ "$RUN_HEALTH_CHECKS" = "1" ]; then
  echo "==> Running post-deploy health checks"
  if ! curl -fsS "${WEB_URL}/health" >/dev/null; then
    echo "Health check failed at ${WEB_URL}/health"
    exit 1
  fi
  if ! curl -fsS "${WEB_URL}/users/test" >/dev/null; then
    echo "Test users endpoint check failed at ${WEB_URL}/users/test"
    exit 1
  fi
  echo "Health checks passed."
fi

echo ""
echo "Deploy complete."
echo "  Hosting URL: ${WEB_URL}"
echo "  Alt URL:     ${FIREBASEAPP_URL}"
echo "  Health:      ${WEB_URL}/health"
echo "  Test users:  ${WEB_URL}/users/test"
