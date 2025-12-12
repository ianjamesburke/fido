/// Migration script to extract and store hashtags from existing posts
///
/// This script scans all posts in the database, extracts hashtags from their content,
/// and populates the hashtags and post_hashtags tables.
///
/// Run with: cargo run --bin migrate-hashtags [--db-path <path>]
use anyhow::{Context, Result};
use clap::Parser;
use rusqlite::{Connection, Transaction};
use uuid::Uuid;

/// Command-line arguments for hashtag migration
#[derive(Parser, Debug)]
#[command(name = "migrate-hashtags")]
#[command(about = "Extract and migrate hashtags from existing posts")]
struct Args {
    /// Path to the SQLite database file
    #[arg(short, long, default_value = "fido.db")]
    db_path: String,

    /// Show progress every N posts
    #[arg(short, long, default_value = "100")]
    progress_interval: usize,

    /// Dry run - don't actually write to database
    #[arg(long)]
    dry_run: bool,
}

/// Statistics for migration results
#[derive(Default, Debug)]
struct MigrationStats {
    total_posts: usize,
    posts_with_hashtags: usize,
    hashtag_links_added: usize,
    hashtags_created: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("🔍 Starting hashtag migration...");
    if args.dry_run {
        println!("⚠️  DRY RUN MODE - No changes will be written");
    }

    // Connect to the database
    let mut conn = Connection::open(&args.db_path)
        .context(format!("Failed to open database: {}", args.db_path))?;
    println!("✅ Connected to database: {}", args.db_path);

    // Check if migration is needed
    if !needs_migration(&conn)? {
        println!("ℹ️  Migration appears to have already been run.");
        println!("   All posts with hashtags are already linked.");
        println!("   Use --force to run anyway (not implemented).");
        return Ok(());
    }

    // Run migration in a transaction
    let stats = if args.dry_run {
        analyze_migration(&conn, args.progress_interval)?
    } else {
        let tx = conn.transaction().context("Failed to start transaction")?;
        let stats = run_migration(&tx, args.progress_interval)?;
        tx.commit().context("Failed to commit transaction")?;
        stats
    };

    // Print results
    print_results(&stats, &conn)?;

    Ok(())
}

/// Check if migration is needed by looking for posts with hashtags but no links
fn needs_migration(conn: &Connection) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM posts WHERE content LIKE '%#%' 
         AND id NOT IN (SELECT DISTINCT post_id FROM post_hashtags)",
        [],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}

/// Analyze what the migration would do (dry run)
fn analyze_migration(conn: &Connection, progress_interval: usize) -> Result<MigrationStats> {
    let posts = fetch_posts(conn)?;
    let mut stats = MigrationStats {
        total_posts: posts.len(),
        ..Default::default()
    };

    println!("📊 Analyzing {} posts...", posts.len());

    for (i, (post_id, content)) in posts.iter().enumerate() {
        if (i + 1) % progress_interval == 0 {
            println!("   Progress: {}/{} posts analyzed", i + 1, posts.len());
        }

        let hashtags = fido_server::hashtag::extract_hashtags(content);

        if !hashtags.is_empty() {
            stats.posts_with_hashtags += 1;
            stats.hashtag_links_added += hashtags.len();

            println!(
                "  📝 Post {}: would add {} hashtags: {:?}",
                &post_id[..8],
                hashtags.len(),
                hashtags
            );
        }
    }

    Ok(stats)
}

/// Run the actual migration
fn run_migration(tx: &Transaction, progress_interval: usize) -> Result<MigrationStats> {
    let posts = fetch_posts_from_tx(tx)?;
    let mut stats = MigrationStats {
        total_posts: posts.len(),
        ..Default::default()
    };

    println!("📊 Processing {} posts...", posts.len());

    for (i, (post_id, content)) in posts.iter().enumerate() {
        if (i + 1) % progress_interval == 0 {
            println!("   Progress: {}/{} posts processed", i + 1, posts.len());
        }

        let hashtags = fido_server::hashtag::extract_hashtags(content);

        if hashtags.is_empty() {
            continue;
        }

        stats.posts_with_hashtags += 1;
        println!(
            "  📝 Post {}: found {} hashtags: {:?}",
            &post_id[..8],
            hashtags.len(),
            hashtags
        );

        for hashtag in &hashtags {
            // Create hashtag if it doesn't exist
            let created = create_hashtag_if_not_exists(tx, hashtag)?;
            if created {
                stats.hashtags_created += 1;
            }

            // Get the hashtag ID
            let hashtag_id = get_hashtag_id(tx, hashtag)?;

            // Link post to hashtag (skip if already linked)
            let linked = link_post_to_hashtag(tx, post_id, &hashtag_id)?;
            if linked {
                stats.hashtag_links_added += 1;
            }
        }
    }

    Ok(stats)
}

/// Fetch all posts from the database
fn fetch_posts(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn
        .prepare("SELECT id, content FROM posts")
        .context("Failed to prepare posts query")?;

    let posts = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to fetch posts")?;

    Ok(posts)
}

/// Fetch all posts from a transaction
fn fetch_posts_from_tx(tx: &Transaction) -> Result<Vec<(String, String)>> {
    let mut stmt = tx
        .prepare("SELECT id, content FROM posts")
        .context("Failed to prepare posts query")?;

    let posts = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to fetch posts")?;

    Ok(posts)
}

/// Create a hashtag if it doesn't already exist
/// Returns true if created, false if already existed
fn create_hashtag_if_not_exists(tx: &Transaction, name: &str) -> Result<bool> {
    let hashtag_id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("Failed to get current time")?
        .as_secs() as i64;

    let rows_affected = tx
        .execute(
            "INSERT OR IGNORE INTO hashtags (id, name, created_at) VALUES (?, ?, ?)",
            (hashtag_id, name, now),
        )
        .context("Failed to insert hashtag")?;

    Ok(rows_affected > 0)
}

/// Get the ID of a hashtag by name
fn get_hashtag_id(tx: &Transaction, name: &str) -> Result<String> {
    tx.query_row("SELECT id FROM hashtags WHERE name = ?", [name], |row| {
        row.get(0)
    })
    .context(format!("Failed to get hashtag ID for: {}", name))
}

/// Link a post to a hashtag
/// Returns true if linked, false if already linked
fn link_post_to_hashtag(tx: &Transaction, post_id: &str, hashtag_id: &str) -> Result<bool> {
    let rows_affected = tx
        .execute(
            "INSERT OR IGNORE INTO post_hashtags (post_id, hashtag_id) VALUES (?, ?)",
            (post_id, hashtag_id),
        )
        .context("Failed to link post to hashtag")?;

    Ok(rows_affected > 0)
}

/// Print migration results and statistics
fn print_results(stats: &MigrationStats, conn: &Connection) -> Result<()> {
    println!("\n✨ Migration complete!");
    println!("   Posts processed: {}", stats.total_posts);
    println!("   Posts with hashtags: {}", stats.posts_with_hashtags);
    println!("   Hashtags created: {}", stats.hashtags_created);
    println!("   Hashtag links added: {}", stats.hashtag_links_added);

    // Show summary of top hashtags
    let mut stmt = conn
        .prepare(
            "SELECT h.name, COUNT(ph.post_id) as post_count 
         FROM hashtags h
         LEFT JOIN post_hashtags ph ON h.id = ph.hashtag_id
         GROUP BY h.name
         ORDER BY post_count DESC
         LIMIT 20",
        )
        .context("Failed to prepare hashtag stats query")?;

    println!("\n📈 Top hashtags:");
    let hashtag_stats = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
    })?;

    for (i, result) in hashtag_stats.enumerate() {
        let (name, count) = result?;
        println!("   {}. #{}: {} posts", i + 1, name, count);
    }

    Ok(())
}
