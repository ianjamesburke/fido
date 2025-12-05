# Fido Deployment Guide

This guide covers deploying the Fido API server to Fly.io with GitHub OAuth authentication and persistent storage.

## Overview

The Fido architecture consists of:
- **TUI Client** (`fido`) - Published to crates.io, installed by users
- **API Server** (`fido-server`) - Deployed to Fly.io at `https://fido-social.fly.dev`
- **Database** - SQLite on persistent volume

Users install the TUI client via `cargo install fido` and connect to the production server automatically.

## Prerequisites

### 1. Install Fly.io CLI

```bash
# macOS
brew install flyctl

# Linux/WSL
curl -L https://fly.io/install.sh | sh

# Windows
powershell -Command "iwr https://fly.io/install.ps1 -useb | iex"
```

### 2. Login to Fly.io

```bash
flyctl auth login
```

### 3. Create GitHub OAuth Application

1. Go to https://github.com/settings/developers
2. Click "New OAuth App"
3. Fill in the details:
   - **Application name:** Fido Social
   - **Homepage URL:** `https://github.com/yourusername/fido`
   - **Authorization callback URL:** `https://fido-social.fly.dev/auth/github/callback`
   - (Replace `fido-social` with your app name if different)
4. Click "Register application"
5. Save the **Client ID** and generate a **Client Secret**
6. Keep these credentials secure - you'll need them for deployment

## Deployment Steps

### 1. Initialize Fly Application

```bash
cd fido
flyctl launch --no-deploy
```

**Answer the prompts:**
- **App name:** `fido-social` (or your choice)
- **Region:** Choose closest to your users (e.g., `sjc` for San Jose)
- **Add databases?** No
- **Deploy now?** No

This creates a `fly.toml` configuration file.

### 2. Create Persistent Volume

The SQLite database needs persistent storage:

```bash
flyctl volumes create fido_data --size 1 --region sjc
```

**Note:** Replace `sjc` with your chosen region from step 1.

### 3. Configure GitHub OAuth Secrets

Set your GitHub OAuth credentials as Fly secrets:

```bash
flyctl secrets set GITHUB_CLIENT_ID=your_client_id_here
flyctl secrets set GITHUB_CLIENT_SECRET=your_client_secret_here
```

**Important:** Never commit these secrets to version control!

### 4. Verify fly.toml Configuration

Ensure your `fly.toml` includes:

```toml
app = 'fido-social'
primary_region = 'sjc'

[build]
  dockerfile = "Dockerfile"

[env]
  DATABASE_PATH = '/data/fido.db'
  RUST_LOG = 'info'
  HOST = '0.0.0.0'
  PORT = '3000'

[[mounts]]
  source = 'fido_data'
  destination = '/data'

[http_service]
  internal_port = 3000
  force_https = true
  auto_stop_machines = true
  auto_start_machines = true
  min_machines_running = 0

[[services]]
  protocol = 'tcp'
  internal_port = 3000
  
  [[services.ports]]
    port = 80
    handlers = ['http']
  
  [[services.ports]]
    port = 443
    handlers = ['tls', 'http']
```

### 5. Deploy to Fly.io

```bash
flyctl deploy
```

This will:
1. Build the Docker image
2. Push it to Fly.io
3. Start the server with your configuration
4. Mount the persistent volume
5. Run database migrations

**Monitor the deployment:**
```bash
flyctl logs -f
```

### 6. Verify Deployment

**Check server status:**
```bash
flyctl status
```

**Test health endpoint:**
```bash
curl https://fido-social.fly.dev/health
```

**Expected response:**
```json
{"status":"ok"}
```

### 7. Test OAuth Flow

1. Install the TUI client: `cargo install fido`
2. Run: `fido`
3. Select "Login with GitHub"
4. Complete the OAuth flow
5. Verify you can create posts and interact with the platform

Your app is now live at: `https://fido-social.fly.dev`

## CI/CD Setup with GitHub Actions

Automate deployments using GitHub Actions.

### 1. Add Fly.io API Token to GitHub Secrets

1. Generate a Fly.io API token:
   ```bash
   flyctl auth token
   ```

2. Go to your GitHub repository settings
3. Navigate to **Settings → Secrets and variables → Actions**
4. Click **New repository secret**
5. Name: `FLY_API_TOKEN`
6. Value: Paste your Fly.io token
7. Click **Add secret**

### 2. Create GitHub Actions Workflow

Create `.github/workflows/deploy.yml`:

```yaml
name: Deploy to Fly.io

on:
  push:
    branches: [main]
  workflow_dispatch:

jobs:
  deploy:
    name: Deploy API Server
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - uses: superfly/flyctl-actions/setup-flyctl@master
      
      - name: Deploy to Fly.io
        run: flyctl deploy --remote-only
        working-directory: ./fido
        env:
          FLY_API_TOKEN: ${{ secrets.FLY_API_TOKEN }}
```

### 3. Test the Workflow

Push to main branch:
```bash
git add .github/workflows/deploy.yml
git commit -m "Add CI/CD workflow"
git push origin main
```

Monitor the deployment in the **Actions** tab of your GitHub repository.

## Local Testing with Docker

Test the Docker build locally before deploying:

```bash
cd fido

# Build image
docker build -t fido-server .

# Run container with volume
docker run -p 3000:3000 \
  -v $(pwd)/data:/data \
  -e DATABASE_PATH=/data/fido.db \
  -e GITHUB_CLIENT_ID=your_client_id \
  -e GITHUB_CLIENT_SECRET=your_client_secret \
  fido-server

# Test health endpoint
curl http://localhost:3000/health
```

## Production Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Developer's Machine                       │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Fido TUI Client (installed via cargo install fido)  │  │
│  │  - Hardcoded server URL: https://fido-social.fly.dev │  │
│  │  - Session token stored in ~/.fido/session           │  │
│  │  - Opens browser for GitHub OAuth                    │  │
│  └────────────────┬─────────────────────────────────────┘  │
│                   │                                          │
└───────────────────┼──────────────────────────────────────────┘
                    │ HTTPS
                    │
┌───────────────────▼──────────────────────────────────────────┐
│                    Fly.io Cloud                               │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Fido API Server (fido-server)                       │  │
│  │  - GitHub OAuth endpoints                            │  │
│  │  - Session management                                │  │
│  │  - REST API for posts, DMs, profiles                │  │
│  │  - Environment: GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET │
│  └────────────────┬─────────────────────────────────────┘  │
│                   │                                          │
│  ┌────────────────▼─────────────────────────────────────┐  │
│  │  SQLite Database (/data/fido.db)                     │  │
│  │  - Persistent volume                                 │  │
│  │  - Users, posts, DMs, sessions                       │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                               │
└───────────────────────────────────────────────────────────────┘
                    │
                    │ OAuth redirect
                    ▼
┌───────────────────────────────────────────────────────────────┐
│                    GitHub OAuth                               │
│  - User authorizes Fido app                                   │
│  - Returns authorization code                                 │
└───────────────────────────────────────────────────────────────┘
```

## Monitoring and Troubleshooting

### View Logs

**Stream live logs:**
```bash
flyctl logs -f
```

**Filter by level:**
```bash
flyctl logs -f | grep ERROR
flyctl logs -f | grep WARN
```

**View recent logs:**
```bash
flyctl logs --limit 100
```

### Check Application Status

```bash
# Overall status
flyctl status

# View metrics dashboard
flyctl dashboard

# Check machine health
flyctl checks list
```

### SSH into Server

```bash
flyctl ssh console
```

**Useful commands once inside:**
```bash
# Check database
ls -la /data/
sqlite3 /data/fido.db ".tables"

# Check environment
env | grep GITHUB
env | grep DATABASE

# Check running processes
ps aux

# Check disk usage
df -h
```

### Common Issues

#### Database not persisting

**Problem:** Data is lost after restart

**Solutions:**
1. Verify volume is mounted: `flyctl volumes list`
2. Check DATABASE_PATH: `flyctl ssh console -C "env | grep DATABASE"`
3. Verify volume is in the same region as your app
4. Check volume size: `flyctl volumes list`

#### OAuth callback fails

**Problem:** GitHub OAuth redirects but authentication fails

**Solutions:**
1. Verify callback URL in GitHub OAuth app matches your Fly.io URL
2. Check secrets are set: `flyctl secrets list`
3. Review logs for OAuth errors: `flyctl logs | grep oauth`
4. Ensure HTTPS is working: `curl https://your-app.fly.dev/health`

#### Server not starting

**Problem:** Deployment succeeds but server won't start

**Solutions:**
1. Check build logs: `flyctl logs`
2. Verify Dockerfile builds locally: `docker build -t test .`
3. Check for missing environment variables
4. Review database migration errors in logs

#### High memory usage

**Problem:** Server crashes with OOM errors

**Solutions:**
1. Scale up memory: `flyctl scale memory 512`
2. Check for memory leaks in logs
3. Review database connection pool settings
4. Consider upgrading to a larger VM

#### Connection timeouts

**Problem:** TUI client can't connect to server

**Solutions:**
1. Check server is running: `flyctl status`
2. Test health endpoint: `curl https://your-app.fly.dev/health`
3. Verify DNS is resolving: `nslookup your-app.fly.dev`
4. Check for firewall issues
5. Review Fly.io status page: https://status.fly.io/

### Performance Monitoring

**Check response times:**
```bash
curl -w "@curl-format.txt" -o /dev/null -s https://your-app.fly.dev/health
```

**Create `curl-format.txt`:**
```
time_namelookup:  %{time_namelookup}\n
time_connect:  %{time_connect}\n
time_starttransfer:  %{time_starttransfer}\n
time_total:  %{time_total}\n
```

## Environment Variables

### Required Secrets

Set via Fly secrets (never in fly.toml):
```bash
flyctl secrets set GITHUB_CLIENT_ID=xxx
flyctl secrets set GITHUB_CLIENT_SECRET=yyy
```

### Optional Configuration

Set in `fly.toml` `[env]` section:
- `DATABASE_PATH`: Path to SQLite database (default: `/data/fido.db`)
- `RUST_LOG`: Log level (default: `info`, options: `trace`, `debug`, `info`, `warn`, `error`)
- `HOST`: Bind address (default: `0.0.0.0`)
- `PORT`: API server port (default: `3000`)

### View Current Configuration

```bash
# List secrets (values hidden)
flyctl secrets list

# View environment variables
flyctl ssh console -C "env"
```

## Scaling

### Vertical Scaling (More Resources)

```bash
# Upgrade to 2x CPU and 512MB RAM
flyctl scale vm shared-cpu-2x --memory 512

# View available VM sizes
flyctl platform vm-sizes
```

### Horizontal Scaling (Multiple Regions)

```bash
# Add regions
flyctl regions add lax syd fra

# List current regions
flyctl regions list

# Remove a region
flyctl regions remove fra
```

### Auto-Scaling

```bash
# Set auto-scaling limits
flyctl autoscale set min=1 max=3

# View current auto-scale settings
flyctl autoscale show
```

## Database Backups

### Manual Backup

```bash
# SSH into server and backup database
flyctl ssh console -C "sqlite3 /data/fido.db .dump" > backup.sql

# Or copy the entire database file
flyctl ssh sftp get /data/fido.db ./fido-backup.db
```

### Automated Backups

Create a GitHub Action for scheduled backups:

```yaml
name: Backup Database

on:
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM UTC
  workflow_dispatch:

jobs:
  backup:
    runs-on: ubuntu-latest
    steps:
      - uses: superfly/flyctl-actions/setup-flyctl@master
      
      - name: Backup database
        run: |
          flyctl ssh console -C "sqlite3 /data/fido.db .dump" > backup-$(date +%Y%m%d).sql
        env:
          FLY_API_TOKEN: ${{ secrets.FLY_API_TOKEN }}
      
      - name: Upload backup
        uses: actions/upload-artifact@v3
        with:
          name: database-backup
          path: backup-*.sql
          retention-days: 30
```

## Cost Optimization

### Fly.io Free Tier

Includes:
- 3 shared-cpu-1x VMs (256MB RAM)
- 3GB persistent volumes
- 160GB outbound data transfer per month

### Cost Estimates (Beyond Free Tier)

- **shared-cpu-1x (256MB):** ~$2/month
- **shared-cpu-2x (512MB):** ~$4/month
- **Persistent volume (1GB):** ~$0.15/month
- **Outbound data:** ~$0.02/GB

### Optimization Tips

1. **Enable auto-stop:** Machines stop when idle (already configured)
2. **Use minimal VM size:** Start with shared-cpu-1x
3. **Optimize database:** Regular VACUUM and cleanup
4. **Monitor usage:** `flyctl dashboard` to track costs
5. **Set spending limits:** Configure in Fly.io dashboard

## Security Best Practices

### Secrets Management

- Store OAuth credentials as Fly secrets
- Never commit secrets to version control
- Rotate secrets periodically
- Use different credentials for staging/production

### HTTPS Configuration

- Force HTTPS (configured in fly.toml)
- Automatic TLS certificates from Let's Encrypt
- HSTS headers enabled

### Database Security

- Database file permissions (handled by volume)
- No direct database access from internet
- Regular backups
- Session token expiry (30 days)

### Application Security

- Session tokens are cryptographically secure (UUID v4)
- OAuth state parameter validation
- Input validation on all endpoints
- Rate limiting (consider adding for production)

## Production Readiness Checklist

Before going to production:

- [ ] GitHub OAuth app configured with production callback URL
- [ ] Fly secrets set (GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET)
- [ ] Persistent volume created and mounted
- [ ] Health endpoint responding: `/health`
- [ ] OAuth flow tested end-to-end
- [ ] Database migrations run successfully
- [ ] Logs configured and monitored
- [ ] Backups scheduled
- [ ] Error tracking configured (optional: Sentry)
- [ ] Performance monitoring set up
- [ ] Documentation updated with production URL
- [ ] TUI client published to crates.io
- [ ] CI/CD pipeline configured

## Additional Resources

- **Fly.io Documentation:** https://fly.io/docs/
- **Fly.io Status:** https://status.fly.io/
- **Fly.io Community:** https://community.fly.io/
- **GitHub OAuth Docs:** https://docs.github.com/en/developers/apps/building-oauth-apps
