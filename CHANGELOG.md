# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

### Fixed

### Removed

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
- Fixed production server URL to include `/api` path prefix for deployed Fly.io instance
- TUI client now correctly connects to `https://fido-social.fly.dev/api` instead of root path
