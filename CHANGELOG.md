# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

### Fixed

### Removed

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
