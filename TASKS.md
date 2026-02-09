# TASKS.md

Tracking checklist for the Firestore migration and Firebase deployment.
Mark items complete by changing `- [ ]` to `- [x]` as work lands.

## 1. Firestore Backend Implementation

- [ ] Implement Firestore-backed store types in `fido-server/src/stores/firestore.rs` for all traits in `fido-server/src/stores/mod.rs`:
- [ ] `UserStore`
- [ ] `PostStore`
- [ ] `HashtagStore`
- [ ] `VoteStore`
- [ ] `FriendStore`
- [ ] `ConfigStore`
- [ ] `RateLimitStore`
- [ ] `DirectMessageStore`
- [ ] `SessionStore`
- [ ] `AuditStore`
- [ ] Add Firestore document schema mapping layer (IDs, timestamps, optional fields, defaults).
- [ ] Add robust error translation from Firestore API errors into server `ApiError`/`SecureError`.
- [ ] Support local emulator via `FIRESTORE_EMULATOR_HOST`.

## 2. Runtime Wiring and Configuration

- [ ] Finalize `DB_BACKEND=firestore` path in `Stores::from_env` so server starts successfully.
- [ ] Define and document required env vars for Firestore production:
- [ ] `DB_BACKEND=firestore`
- [ ] `GOOGLE_CLOUD_PROJECT` or `FIREBASE_PROJECT_ID`
- [ ] Credentials strategy for Firebase hosting environment (service account / ADC).
- [ ] Ensure GitHub OAuth flow continues to work with Firestore-backed user/session data.
- [ ] Remove/retire direct SQLite runtime dependencies for normal server operation.

## 3. Data Migration and Compatibility

- [ ] Create one-time migration/export strategy from existing SQLite data to Firestore.
- [ ] Provide migration script/tool and dry-run mode.
- [ ] Validate key entities after migration:
- [ ] users
- [ ] posts and replies
- [ ] hashtags and follows
- [ ] votes/karma
- [ ] friendships/follow graph
- [ ] DMs/conversations
- [ ] user config
- [ ] sessions/audit (as applicable)
- [ ] Document rollback strategy if Firestore cutover fails.

## 4. Test Coverage and Validation

- [ ] Add unit tests for Firestore store implementations.
- [ ] Add integration tests that run against Firestore emulator.
- [ ] Remove or archive SQLite-specific tests that are no longer relevant after cutover.
- [ ] Add CI path that runs Firestore emulator tests.
- [ ] Keep and pass local smoke check: `just firestore-emulator-check`.

## 5. Local Preview and Demo Behavior

- [ ] Verify `start.sh` local stack still works (`fido-server` + `ttyd` + `nginx`).
- [ ] Verify browser preview still loads terminal at `/ttyd/`.
- [ ] Confirm demo mode (`FIDO_DEMO_MODE=true`) remains mock/in-memory and does not require Firestore.
- [ ] Confirm non-demo mode can hit Firestore backend when configured.

## 6. Firebase Deployment

- [ ] Finalize Firebase project setup and service enablement.
- [ ] Confirm/update `firebase.json`, `firestore.rules`, and `firestore.indexes.json`.
- [ ] Define deployment target(s) for:
- [ ] API server runtime
- [ ] web terminal preview/static hosting
- [ ] Configure env vars/secrets in Firebase deployment target.
- [ ] Run deploy in staging and validate:
- [ ] health endpoint
- [ ] auth login flow
- [ ] post CRUD and feed
- [ ] DM flow
- [ ] follow/unfollow + profile lookups
- [ ] Firestore reads/writes confirmed in console/emulator logs
- [ ] Run production deploy and post-deploy sanity checks.

## 7. Crates.io TUI Release Alignment

- [ ] Ensure server URL resolution strategy supports Firebase endpoint cleanly:
- [ ] default endpoint for released client
- [ ] `FIDO_SERVER_URL` override remains supported
- [ ] update docs for endpoint configuration
- [ ] Update `fido-tui` release notes to call out Firebase/Firestore backend support.
- [ ] Publish updated `fido-tui` crate to crates.io with new default/config behavior.
- [ ] Verify freshly installed crate (`cargo install fido`) connects to intended Firebase-hosted API by default (or documented override path).

## 8. Documentation and Operations

- [ ] Update `README.md` for Firestore-first backend and env configuration.
- [ ] Update `QUICKSTART.md` with Firebase-local workflow and emulator usage.
- [ ] Update deployment docs (including Firebase runbook and rollback notes).
- [ ] Add troubleshooting section for Firestore emulator and auth issues.
- [ ] Remove/mark Fly.io-specific docs/config as legacy once Firebase path is primary.
