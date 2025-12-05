# Requirements Document

## Introduction

This specification defines the requirements for transforming Fido from a local development prototype into a production-ready, publicly accessible social platform. The system must enable any developer to install the TUI client via Cargo and immediately connect to a live server with persistent authentication, making Fido widely useful and accessible for the hackathon demonstration and beyond.

## Glossary

- **TUI Client**: The terminal user interface application (fido-tui) that users install and run locally
- **API Server**: The backend Axum server (fido-server) deployed on Fly.io that handles all data and business logic
- **GitHub OAuth**: GitHub's OAuth 2.0 authentication system for verifying user identity
- **Session Token**: A cryptographically secure token that identifies an authenticated user session
- **Cargo Crate**: A Rust package published to crates.io that users can install with `cargo install`
- **Fly.io**: The cloud platform hosting the production API server
- **Session Store**: Local file storage at `~/.fido/session` (user home directory) for persisting authentication tokens with 0600 permissions

## Requirements

### Requirement 1

**User Story:** As a developer, I want to install Fido with a single Cargo command, so that I can start using the platform immediately without manual setup.

#### Acceptance Criteria

1. WHEN a user runs `cargo install fido-tui` THEN the system SHALL download and install the TUI client binary
2. WHEN the TUI client starts THEN the system SHALL connect to the production API server at `https://fido-social.fly.dev` by default
3. WHEN the installation completes THEN the system SHALL provide clear instructions for first-time authentication
4. THE TUI client SHALL include the production server URL (`https://fido-social.fly.dev`) as a compile-time constant
5. THE TUI client SHALL support overriding the server URL via `FIDO_SERVER_URL` environment variable or `--server` CLI argument

### Requirement 2

**User Story:** As a new user, I want to authenticate with my GitHub account, so that I can have a persistent identity on the platform without creating yet another account.

#### Acceptance Criteria

1. WHEN a user launches the TUI without a valid session THEN the system SHALL display a GitHub authentication prompt
2. WHEN a user initiates GitHub authentication THEN the system SHALL open the user's default browser to GitHub's OAuth page
3. WHEN GitHub redirects after successful authentication THEN the API server SHALL create a new user account if one does not exist
4. WHEN authentication completes THEN the system SHALL store the session token securely in the local session store
5. WHEN a user's GitHub account is linked THEN the system SHALL use the GitHub username as the Fido username

### Requirement 3

**User Story:** As an authenticated user, I want my session to persist across TUI restarts, so that I don't have to re-authenticate every time I use Fido.

#### Acceptance Criteria

1. WHEN the TUI client starts THEN the system SHALL check for a valid session token in the session store
2. WHEN a valid session token exists THEN the system SHALL authenticate automatically without user interaction
3. WHEN a session token is invalid or expired THEN the system SHALL prompt for re-authentication
4. THE session store SHALL be located at `~/.fido/session` with appropriate file permissions (0600)
5. WHEN a user logs out THEN the system SHALL delete the local session token

### Requirement 4

**User Story:** As the API server, I want to validate GitHub OAuth tokens and manage user sessions, so that only authenticated users can access protected resources.

#### Acceptance Criteria

1. WHEN the server receives an OAuth callback from GitHub THEN the system SHALL exchange the authorization code for an access token
2. WHEN the server obtains a GitHub access token THEN the system SHALL fetch the user's GitHub profile information
3. WHEN a new GitHub user authenticates THEN the system SHALL create a user record in the database with GitHub ID and username
4. WHEN a user session is created THEN the system SHALL generate a cryptographically secure session token (UUID v4)
5. WHEN the server receives an API request with a session token THEN the system SHALL validate the token and identify the user

### Requirement 5

**User Story:** As the API server, I want to be deployed on Fly.io with persistent storage, so that user data survives server restarts and the platform is publicly accessible.

#### Acceptance Criteria

1. THE API server SHALL be deployed to Fly.io with a public HTTPS endpoint at `https://fido-social.fly.dev`
2. THE API server SHALL use a persistent volume mounted at `/data` for the SQLite database at `/data/fido.db`
3. WHEN the server starts THEN the system SHALL run database migrations automatically
4. THE server SHALL read GitHub OAuth credentials from Fly secrets: `GITHUB_CLIENT_ID` and `GITHUB_CLIENT_SECRET`
5. THE server SHALL log all startup events and errors for debugging

### Requirement 6

**User Story:** As a developer, I want clear documentation on how to use Fido, so that I can quickly understand the installation and authentication process.

#### Acceptance Criteria

1. THE README SHALL include a "Quick Start" section with installation and first-run instructions
2. THE README SHALL document the GitHub OAuth authentication flow
3. THE README SHALL provide the production server URL for reference
4. THE repository SHALL include a DEPLOYMENT.md file with server deployment instructions
5. THE TUI client SHALL display helpful error messages when authentication fails

### Requirement 7

**User Story:** As a system administrator, I want the OAuth flow to handle errors gracefully, so that users receive clear feedback when authentication fails.

#### Acceptance Criteria

1. WHEN GitHub OAuth fails THEN the system SHALL display a user-friendly error message
2. WHEN the browser cannot be opened THEN the system SHALL provide a manual authentication URL
3. WHEN the API server is unreachable THEN the system SHALL display a connection error with retry instructions
4. WHEN a session token expires THEN the system SHALL prompt for re-authentication without data loss
5. THE system SHALL log all authentication errors for debugging purposes

### Requirement 8

**User Story:** As a developer, I want the TUI to support both production and local development modes, so that I can test changes without affecting the production server.

#### Acceptance Criteria

1. WHEN the environment variable `FIDO_SERVER_URL` is set THEN the system SHALL use that URL instead of the default
2. WHEN the CLI argument `--server <URL>` is provided THEN the system SHALL use that URL
3. THE TUI client SHALL display the current server URL in the settings tab
4. WHEN connecting to localhost THEN the system SHALL support test user authentication as a fallback
5. THE system SHALL validate the server URL format before attempting connection

### Requirement 9

**User Story:** As a maintainer, I want automated deployment pipelines, so that updates to the server and TUI are published efficiently without manual intervention.

#### Acceptance Criteria

1. WHEN code is pushed to the main branch THEN the system SHALL automatically deploy the API server to Fly.io via GitHub Actions
2. WHEN a git tag matching `v*.*.*` is pushed THEN the system SHALL automatically publish the TUI crate to crates.io
3. THE GitHub Actions workflow SHALL run tests before deploying or publishing
4. THE Cargo.toml version SHALL be automatically updated based on the git tag
5. THE deployment pipeline SHALL notify on success or failure

### Requirement 10

**User Story:** As a user, I want to install only the TUI client, so that I don't need to manage server infrastructure myself.

#### Acceptance Criteria

1. THE fido-types crate SHALL be published to crates.io as a library crate
2. THE TUI client SHALL be published to crates.io as the `fido` crate (binary name: `fido`)
3. THE fido-server SHALL remain unpublished and only run on Fly.io
4. THE TUI client SHALL have no runtime dependency on the server binary
5. THE installation process SHALL require only `cargo install fido` with Cargo automatically resolving the fido-types dependency

### Requirement 11

**User Story:** As a maintainer, I want a clean production repository, so that the hackathon submission is professional and the CI/CD pipeline is straightforward to set up.

#### Acceptance Criteria

1. THE production code SHALL be moved to a new GitHub repository named `fido`
2. THE new repository SHALL have a clean commit history starting from the production-ready state
3. THE new repository SHALL include only the `fido/` directory contents at the root level (fido-server, fido-tui, fido-types, etc.)
4. THE new repository SHALL include the `.kiro/` directory to demonstrate spec-driven development
5. THE new repository SHALL remove all temporary markdown files (FIXES_APPLIED.md, BUGFIXES.md, etc.) keeping only essential documentation (README.md, QUICKSTART.md, ARCHITECTURE.md, DEPLOYMENT.md)
6. THE database path configuration SHALL work correctly with the new directory structure (fido.db at root level)
7. THE GitHub Actions workflows SHALL be configured in the new repository for automated deployment
