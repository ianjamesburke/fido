# Fido Hashtag Migration Utility

A one-time migration tool to backfill hashtag data for existing posts in the Fido database.

## Overview

This utility extracts hashtags from existing post content and stores them in the database tables (`hashtags` and `post_hashtags`). This is necessary for posts created before the hashtag storage system was implemented.

## Usage

### Basic Usage

```bash
# Run migration with default database (./fido.db)
cargo run --package fido-migrate

# Specify custom database path
cargo run --package fido-migrate -- --database /path/to/fido.db

# Dry run to preview changes without modifying database
cargo run --package fido-migrate -- --dry-run

# Skip confirmation prompt
cargo run --package fido-migrate -- --yes
```

### Command-Line Options

- `-d, --database <PATH>` - Path to the SQLite database file (default: `./fido.db`)
- `-n, --dry-run` - Perform a dry run without making changes
- `-y, --yes` - Skip confirmation prompt
- `-h, --help` - Print help information

### Examples

```bash
# Preview what the migration will do
cargo run --package fido-migrate -- --database ../fido.db --dry-run

# Run migration with confirmation
cargo run --package fido-migrate -- --database ../fido.db

# Run migration without confirmation prompt
cargo run --package fido-migrate -- --database ../fido.db --yes
```

## Exit Codes

- `0` - Migration completed successfully
- `1` - Fatal error (database connection failed, invalid schema, etc.)
- `2` - Partial failure (some posts failed to process, but migration completed)

## Features

- **Idempotent**: Safe to run multiple times - won't create duplicate hashtags or associations
- **Error Handling**: Continues processing remaining posts if individual posts fail
- **Progress Reporting**: Shows progress every 100 posts for large databases
- **Statistics**: Displays summary of posts processed, hashtags created, and any errors
- **Dry Run Mode**: Preview changes without modifying the database

## How It Works

1. Connects to the database and validates schema
2. Queries all posts from the database
3. For each post:
   - Extracts hashtags from content using regex pattern `#(\w{2,})`
   - Stores hashtags in `hashtags` table (if not already present)
   - Creates associations in `post_hashtags` table
4. Displays migration summary with statistics

## Requirements

- Rust 1.70 or later
- SQLite database with Fido schema
- Required tables: `posts`, `hashtags`, `post_hashtags`

## Testing

Run the test suite:

```bash
cargo test --package fido-migrate
```

The test suite includes:
- Property-based tests for extraction consistency
- Integration tests for association correctness
- Idempotency tests
- Completeness tests
- Robustness tests for edge cases

## Troubleshooting

### Database file not found

Ensure the database path is correct. The default is `./fido.db` relative to where you run the command.

### Schema validation failed

The database must have the required tables (`posts`, `hashtags`, `post_hashtags`). Run the Fido server initialization first.

### Foreign key constraint failed

Ensure all posts have valid `author_id` references to existing users in the `users` table.

## Development

The migration utility is part of the Fido workspace:

```
fido/
├── fido-migrate/     # This migration utility
├── fido-server/      # Main server
├── fido-tui/         # Terminal UI
└── fido-types/       # Shared types
```

Build the migration utility:

```bash
cd fido
cargo build --package fido-migrate
```

The compiled binary will be at `target/debug/fido-migrate` (or `target/release/fido-migrate` for release builds).
