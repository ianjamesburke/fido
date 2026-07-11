# Deployment topology

Fido runs as **two separate Railway services** built from the same Docker image.
The service's role is selected at runtime by the `FIDO_DEPLOY_MODE` environment
variable, read by `start.sh`.

Splitting them is a security requirement, not an optimization: the demo resets
its database on every boot, so if the demo and the persistent API shared one
service, either real accounts (and their encrypted GitHub tokens) would be wiped
on every deploy, or anonymous web-terminal visitors would be driving a TUI
against the production database. Neither is acceptable.

## Service 1 — Demo web terminal (`FIDO_DEPLOY_MODE=demo`)

- Runs nginx + ttyd + the TUI + fido-server (see `nginx.conf`).
- Database is **ephemeral**: `start.sh` deletes it on every boot unless
  `FIDO_DEMO_EPHEMERAL=false` (do **not** set that here).
- Serves the landing page and the anonymous browser terminal.
- Holds **no real user data**. No persistent volume.
- This is the default mode, so no `FIDO_DEPLOY_MODE` override is required, though
  setting it explicitly to `demo` is recommended for clarity.

Variables: `GITHUB_CLIENT_ID`, `FIDO_TOKEN_KEY`, `RUST_LOG`.

## Service 2 — Persistent API (`FIDO_DEPLOY_MODE=api`)

- Runs fido-server behind an **API-only** nginx (`nginx-api.conf`): real
  client-IP resolution and edge rate limiting, but **no ttyd**, no anonymous
  terminal, and no static demo site.
- Mounts a **persistent Railway volume at `/data`**; the SQLite DB lives at
  `/data/fido.db` and is **never wiped** on deploy.
- This is the host the installed TUI talks to by default
  (`DEFAULT_PUBLIC_SERVER_URL` in `fido-tui/src/api/client.rs`).

Variables:

- `FIDO_DEPLOY_MODE=api`
- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET` (required in production; see the GitHub integration)
- `FIDO_TOKEN_KEY`
- `ENVIRONMENT=production`
- `RUST_LOG`
- `MAX_REQUEST_SIZE` (optional, bytes; defaults to 1 MiB) — the request body
  limit is enforced from this value.
- `FIDO_ADMIN_LOGINS` (optional) — comma-separated GitHub logins granted access
  to the admin endpoints (`/auth/cleanup-sessions`, `/admin/config/validate`).
  This is the supported production grant path; seed-data admins are dev-only.
- Persistent volume mounted at `/data`.

## Client default

The installed TUI defaults to the **API** service
(`https://fido-api-production.up.railway.app`), not the demo host. Override with
`FIDO_SERVER_URL` or `~/.fido/server_url`.

## MANUAL FOLLOW-UP (cannot be done from the repo)

The second Railway service must be created in the Railway dashboard:

1. Create a new Railway service from the same repo/image as the demo.
2. Set `FIDO_DEPLOY_MODE=api`, `ENVIRONMENT=production`, `GITHUB_CLIENT_ID`,
   `GITHUB_CLIENT_SECRET`, and `FIDO_TOKEN_KEY`.
3. Attach a persistent volume mounted at `/data`.
4. Point `https://fido-api-production.up.railway.app` (or the chosen API domain)
   at this service; update `DEFAULT_PUBLIC_SERVER_URL` if the domain differs.
5. Verify the demo service has **no** persistent volume and still resets its DB.
6. Verify fido-server's internal port (3000) is not publicly reachable on either
   service — only nginx listens on the external port.
