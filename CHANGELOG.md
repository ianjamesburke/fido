# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

### Fixed

### Removed



## [0.1.16] - 2026-02-11

### Changed
- Extended session lifetime from 7 days to 30 days for improved user experience
- Extended session idle timeout from 24 hours to 30 days
- Updated session cookie max age to 30 days
- Improved OAuth flow with manual browser open option (press 'o')
- Enhanced auth screen instructions with clearer step-by-step guidance
- Improved session validation error handling to preserve tokens on transient failures

### Fixed
- Session validation now distinguishes between auth failures and transient errors
- Browser no longer auto-opens during GitHub OAuth to prevent interruptions



## [0.1.15] - 2026-02-10

### Added
- Version bump command in justfile for automated version management
- Filtered query support in Firestore client for complex queries

### Changed
- Optimized reply count computation in Firestore post store
- Improved Firestore query performance with batch operations

### Removed
- Unused mark_messages_read endpoint from API client and backend



## [0.1.14] - 2026-02-10

### Changed
- Improved event loop responsiveness with reduced polling interval (100ms → 33ms)
- Optimized tab data loading to prevent redundant API calls

### Fixed
- Removed artificial loading delay in post refresh for faster UI updates

### Removed
- Unnecessary 200ms delay in post loading operations



## [0.1.12] - 2026-02-10

### Changed
- Improved type safety and terminal validation in UI layer
- Consolidated app architecture with improved test isolation
- Extracted business logic into service layer with store abstraction
- Consolidated post operations into dedicated service layer
- Extracted post retrieval logic into service layer
- Extracted authentication and header utilities into dedicated HTTP module

### Fixed
- Cleaned up compiler warnings

## [0.1.11] - 2026-02-10

### Changed
- Migrated deployment documentation from Fly.io to Firebase/Cloud Run

## [0.1.10] - 2026-01-07

### Changed
- Simplified cargo deploy process

## [0.1.9] - 2026-01-07

### Changed
- Firebase refactor complete
- Improved Firebase integration

## [0.1.8] - 2026-01-07

- Literally just testing the outdated version banner so needed a version bump.
- Apologies for the braindead vibe coding smell. 
- sorry not sorry.
- this shits fun get off my ass.
- luv you.

## [0.1.7] - 2026-01-07

### Added
- Update availability checking with notification banner in TUI
- Self-update functionality via `--update` CLI flag

### Changed
- Expanded README with installation options and development guide
- Improved dry-run checks and dependency handling in publish script

## [0.1.6] - 2026-01-07

### Added
- Web terminal demo mode with ttyd integration
- Rate limit handling with improved error display in TUI

### Changed
- Extended session expiration and improved logging defaults
- Simplified demo mode banner and terminal title
- Install ttyd from GitHub releases with improved environment detection

## [0.1.5] - 2024-12-11

### Changed
- Optimized Docker build caching for faster deployments
- Improved deployment configuration

## [0.1.4] - 2024-12-10

### Added
- Improved API client robustness for better error handling
- Enhanced Docker security configurations

### Changed
- Updated README.md with latest project information

## [0.1.3] - 2024-12-07

### Added
- ASCII art logo to auth screen
- SEO metadata and GitHub link to landing page

### Changed
- Standardized auth screen colors to white
- Redesigned website with updated color scheme and improved typography
- Refined landing page copy and simplified feature presentation
- Simplified installation instructions and feature list in README

### Fixed
- Adjusted max posts input constraints in settings

## [0.1.2] - 2024-12-05

### Added
- Multi-layer rate limiting system for API protection
- Auto-clearing message system for success notifications
- Enhanced new conversation modal with search and selection
- Improved terminal UI responsiveness and styling
- Pull-to-refresh functionality for web interface
- Demo section to web interface
- README files and Cargo.toml metadata for crates.io publishing
- Changelog automation

### Changed
- Updated action bar text for posts tab with improved shortcuts
- Updated default colors
- Refactored color scheme to use CSS variables for improved maintainability
- Increased social modal footer height to display keyboard shortcuts

### Fixed
- Correct production server URL to include /api path prefix

### Removed
- Post editing functionality

## [0.1.1] - 2024-12-05

### Fixed
- Fixed production server URL to include `/api` path prefix for legacy hosted instance
- TUI client now correctly connects to the production `/api` path instead of root
