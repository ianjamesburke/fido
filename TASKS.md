# TASKS.md

Tracking checklist for the Firestore migration and Firebase deployment.
Mark items complete by changing `- [ ]` to `- [x]` as work lands.

## 1. Firestore Backend Implementation

- [x] Implement Firestore-backed store types in `fido-server/src/stores/firestore.rs` for all traits in `fido-server/src/stores/mod.rs`:
- [x] `UserStore`
- [x] `PostStore`
- [x] `HashtagStore`
- [x] `VoteStore`
- [x] `FriendStore`
- [x] `ConfigStore`
- [x] `RateLimitStore`
- [x] `DirectMessageStore`
- [x] `SessionStore`
- [x] `AuditStore`
- [x] Add Firestore document schema mapping layer (IDs, timestamps, optional fields, defaults).
- [x] Add robust error translation from Firestore API errors into server `ApiError`/`SecureError`.
- [x] Support local emulator via `FIRESTORE_EMULATOR_HOST`.

## 2. Runtime Wiring and Configuration

- [x] Finalize `DB_BACKEND=firestore` path in `Stores::from_env` so server starts successfully.
- [x] Define and document required env vars for Firestore production:
- [x] `DB_BACKEND=firestore`
- [x] `GOOGLE_CLOUD_PROJECT` or `FIREBASE_PROJECT_ID`
- [x] Credentials strategy for Firebase hosting environment (service account / ADC).
- [ ] Ensure GitHub OAuth flow continues to work with Firestore-backed user/session data.
- [ ] Remove/retire direct SQLite runtime dependencies for normal server operation.

## 3. Data Migration and Compatibility

- [x] Create one-time migration/export strategy from existing SQLite data to Firestore.
- [x] Provide migration script/tool and dry-run mode.
- [x] Validate key entities after migration:
- [x] users
- [x] posts and replies
- [x] hashtags and follows
- [x] votes/karma
- [x] friendships/follow graph
- [x] DMs/conversations
- [x] user config
- [x] sessions/audit (as applicable)
- [x] Document rollback strategy if Firestore cutover fails.

## 4. Test Coverage and Validation

- [x] Add unit tests for Firestore store implementations.
- [x] Add integration tests that run against Firestore emulator.
- [x] Remove or archive SQLite-specific tests that are no longer relevant after cutover.
- [x] Add CI path that runs Firestore emulator tests.
- [ ] Keep and pass local smoke check: `just firestore-emulator-check`.

## 5. Local Preview and Demo Behavior

- [ ] Verify `start.sh` local stack still works (`fido-server` + `ttyd` + `nginx`).
- [ ] Verify browser preview still loads terminal at `/ttyd/`.
- [x] Confirm demo mode (`FIDO_DEMO_MODE=true`) remains mock/in-memory and does not require Firestore.
- [x] Confirm non-demo mode can hit Firestore backend when configured.

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

- [x] Update `README.md` for Firestore-first backend and env configuration.
- [x] Update `QUICKSTART.md` with Firebase-local workflow and emulator usage.
- [x] Update deployment docs (including Firebase runbook and rollback notes).
- [x] Add troubleshooting section for Firestore emulator and auth issues.
- [x] Remove/mark legacy hosting docs/config as legacy once Firebase path is primary.
