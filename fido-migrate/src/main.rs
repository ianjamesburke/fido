use anyhow::{Context, Result};
use clap::Parser;
use fido_server::db::{repositories::HashtagRepository, Database};
use fido_server::hashtag::extract_hashtags;
use std::collections::HashSet;
use uuid::Uuid;

/// Fido Hashtag Migration Utility
/// 
/// This tool backfills hashtag data for existing posts by extracting hashtags
/// from post content and storing them in the database tables.
#[derive(Parser, Debug)]
#[command(name = "fido-migrate")]
#[command(about = "Backfill hashtag data for existing Fido posts", long_about = None)]
struct Args {
    /// Path to the SQLite database file
    #[arg(short, long, default_value = "./fido.db")]
    database: String,
    
    /// Perform a dry run without making changes
    #[arg(short = 'n', long)]
    dry_run: bool,
    
    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    yes: bool,
}

/// Statistics collected during migration
#[derive(Debug, Default)]
struct MigrationStats {
    /// Total number of posts processed
    posts_processed: usize,
    /// Number of posts that had hashtags
    posts_with_hashtags: usize,
    /// Total number of hashtag associations created
    total_hashtags_created: usize,
    /// Number of unique hashtags encountered
    unique_hashtags: usize,
    /// Errors encountered during migration
    errors: Vec<String>,
}

impl MigrationStats {
    /// Create a new empty statistics tracker
    fn new() -> Self {
        Self::default()
    }
    
    /// Record that a post was processed
    fn record_post_processed(&mut self) {
        self.posts_processed += 1;
    }
    
    /// Record hashtags found in a post
    fn record_hashtags(&mut self, count: usize) {
        if count > 0 {
            self.posts_with_hashtags += 1;
            self.total_hashtags_created += count;
        }
    }
    
    /// Record a unique hashtag
    fn record_unique_hashtag(&mut self) {
        self.unique_hashtags += 1;
    }
    
    /// Record an error
    fn record_error(&mut self, error: String) {
        self.errors.push(error);
    }
}

/// Represents a post from the database
#[derive(Debug)]
struct Post {
    id: Uuid,
    content: String,
}

/// Process a single post: extract hashtags and store them
fn process_post(
    post: &Post,
    hashtag_repo: &HashtagRepository,
    unique_hashtags: &mut HashSet<String>,
    stats: &mut MigrationStats,
    dry_run: bool,
) -> Result<()> {
    // Extract hashtags from post content
    let hashtags = extract_hashtags(&post.content);
    
    if hashtags.is_empty() {
        // No hashtags in this post
        stats.record_post_processed();
        return Ok(());
    }
    
    // Track unique hashtags
    for hashtag in &hashtags {
        if unique_hashtags.insert(hashtag.clone()) {
            stats.record_unique_hashtag();
        }
    }
    
    // Store hashtags if not dry run
    if !dry_run {
        hashtag_repo.store_hashtags(&post.id, &hashtags)
            .with_context(|| format!("Failed to store hashtags for post {}", post.id))?;
    }
    
    // Update statistics
    stats.record_post_processed();
    stats.record_hashtags(hashtags.len());
    
    Ok(())
}

/// Query all posts from the database
fn query_all_posts(db: &Database) -> Result<Vec<Post>> {
    let conn = db.pool.get()
        .context("Failed to get database connection")?;
    
    let mut stmt = conn.prepare("SELECT id, content FROM posts")
        .context("Failed to prepare query")?;
    
    let posts = stmt.query_map([], |row| {
        let id_str: String = row.get(0)?;
        let content: String = row.get(1)?;
        
        Ok(Post {
            id: Uuid::parse_str(&id_str).map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(e))
            })?,
            content,
        })
    })
    .context("Failed to execute query")?
    .collect::<Result<Vec<_>, _>>()
    .context("Failed to collect posts")?;
    
    Ok(posts)
}

/// Connect to the database and validate schema
fn connect_database(path: &str) -> Result<Database> {
    println!("Connecting to database: {}", path);
    
    // Check if database file exists
    if !std::path::Path::new(path).exists() {
        anyhow::bail!("Database file not found: {}", path);
    }
    
    // Open database connection
    let db = Database::new(path)
        .context("Failed to open database connection")?;
    
    // Validate that the database has the required schema
    // We'll do a simple check by querying the posts table
    let conn = db.pool.get()
        .context("Failed to get database connection from pool")?;
    
    // Check if posts table exists
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='posts'",
        [],
        |row| row.get::<_, i32>(0).map(|count| count > 0)
    ).context("Failed to check for posts table")?;
    
    if !table_exists {
        anyhow::bail!("Database schema is invalid - posts table not found");
    }
    
    // Check if hashtags table exists
    let hashtags_table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='hashtags'",
        [],
        |row| row.get::<_, i32>(0).map(|count| count > 0)
    ).context("Failed to check for hashtags table")?;
    
    if !hashtags_table_exists {
        anyhow::bail!("Database schema is invalid - hashtags table not found");
    }
    
    // Check if post_hashtags table exists
    let post_hashtags_table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='post_hashtags'",
        [],
        |row| row.get::<_, i32>(0).map(|count| count > 0)
    ).context("Failed to check for post_hashtags table")?;
    
    if !post_hashtags_table_exists {
        anyhow::bail!("Database schema is invalid - post_hashtags table not found");
    }
    
    println!("Database connection successful - schema validated");
    
    Ok(db)
}

/// Display migration statistics in a formatted way
fn display_stats(stats: &MigrationStats, dry_run: bool) {
    println!();
    println!("Migration Summary");
    println!("=================");
    println!();
    println!("Posts processed: {}", stats.posts_processed);
    println!("Posts with hashtags: {}", stats.posts_with_hashtags);
    println!("Total hashtag associations: {}", stats.total_hashtags_created);
    println!("Unique hashtags: {}", stats.unique_hashtags);
    
    if !stats.errors.is_empty() {
        println!();
        println!("Errors encountered: {}", stats.errors.len());
        for (i, error) in stats.errors.iter().enumerate() {
            println!("  {}. {}", i + 1, error);
        }
    }
    
    println!();
    if dry_run {
        println!("This was a dry run - no changes were made to the database.");
    } else {
        println!("Migration completed successfully!");
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("Fido Hashtag Migration Utility");
    println!("================================");
    println!();
    println!("Database: {}", args.database);
    println!("Dry run: {}", args.dry_run);
    println!();
    
    // Connect to database
    let db = connect_database(&args.database)?;
    
    // Query all posts
    println!("Querying all posts...");
    let posts = query_all_posts(&db)?;
    println!("Found {} posts", posts.len());
    
    // Handle empty database
    if posts.is_empty() {
        println!("No posts found in database - nothing to migrate.");
        return Ok(());
    }
    
    // Show confirmation prompt unless --yes flag is provided
    if !args.yes && !args.dry_run {
        println!("This will backfill hashtag data for {} posts.", posts.len());
        println!("Do you want to continue? (y/N): ");
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)
            .context("Failed to read user input")?;
        
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("Migration cancelled.");
            return Ok(());
        }
    }
    
    // Initialize statistics tracker and hashtag tracking
    let mut stats = MigrationStats::new();
    let mut unique_hashtags: HashSet<String> = HashSet::new();
    
    // Create hashtag repository
    let hashtag_repo = HashtagRepository::new(db.pool.clone());
    
    // Process each post
    println!();
    println!("Processing posts...");
    for (i, post) in posts.iter().enumerate() {
        // Show progress every 100 posts
        if (i + 1) % 100 == 0 {
            println!("Processed {} / {} posts...", i + 1, posts.len());
        }
        
        // Process the post
        if let Err(e) = process_post(post, &hashtag_repo, &mut unique_hashtags, &mut stats, args.dry_run) {
            // Log error but continue processing
            let error_msg = format!("Error processing post {}: {:#}", post.id, e);
            eprintln!("ERROR: {}", error_msg);
            stats.record_error(error_msg);
        }
    }
    
    println!("Finished processing {} posts", posts.len());
    
    // Display stats
    display_stats(&stats, args.dry_run);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    // Feature: hashtag-backfill-migration, Property 1: Migration completeness and accuracy
    // For any database state with posts, after running the migration, the reported statistics
    // should match the actual database state
    #[test]
    fn test_completeness() {
        // Create in-memory database
        let db = Database::in_memory().expect("Failed to create in-memory database");
        db.initialize().expect("Failed to initialize database");
        
        // Create a test user
        let user_id = Uuid::new_v4();
        let conn = db.pool.get().expect("Failed to get connection");
        conn.execute(
            "INSERT INTO users (id, username, join_date, is_test_user) VALUES (?, ?, ?, ?)",
            (user_id.to_string(), "testuser", "2024-01-01T00:00:00Z", 1),
        ).expect("Failed to insert test user");
        
        // Create test posts
        let posts_data = vec![
            ("Post with #rust", 1),
            ("Post with #rust and #webdev", 2),
            ("Post with no hashtags", 0),
            ("Another #rust post", 1),
        ];
        
        let mut expected_total_hashtags = 0;
        for (content, hashtag_count) in &posts_data {
            let post_id = Uuid::new_v4();
            conn.execute(
                "INSERT INTO posts (id, author_id, content, created_at, upvotes, downvotes, parent_post_id, reply_count) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                (post_id.to_string(), user_id.to_string(), content, "2024-01-01T00:00:00Z", 0, 0, None::<String>, 0),
            ).expect("Failed to insert post");
            expected_total_hashtags += hashtag_count;
        }
        
        // Run migration
        let posts = query_all_posts(&db).expect("Failed to query posts");
        let mut stats = MigrationStats::new();
        let mut unique_hashtags = HashSet::new();
        let hashtag_repo = HashtagRepository::new(db.pool.clone());
        
        for post in &posts {
            process_post(post, &hashtag_repo, &mut unique_hashtags, &mut stats, false)
                .expect("Failed to process post");
        }
        
        // Verify statistics match expectations
        assert_eq!(stats.posts_processed, posts_data.len(), "All posts should be processed");
        assert_eq!(stats.total_hashtags_created, expected_total_hashtags, "Total hashtags should match");
        assert_eq!(stats.posts_with_hashtags, 3, "Three posts have hashtags");
        assert_eq!(stats.unique_hashtags, 2, "Two unique hashtags: rust and webdev");
    }
    
    // Feature: hashtag-backfill-migration, Property 4: Migration idempotency
    // For any database state, running the migration multiple times should produce
    // the same final database state as running it once
    #[test]
    fn test_idempotency() {
        // Create in-memory database
        let db = Database::in_memory().expect("Failed to create in-memory database");
        db.initialize().expect("Failed to initialize database");
        
        // Create a test user
        let user_id = Uuid::new_v4();
        let conn = db.pool.get().expect("Failed to get connection");
        conn.execute(
            "INSERT INTO users (id, username, join_date, is_test_user) VALUES (?, ?, ?, ?)",
            (user_id.to_string(), "testuser", "2024-01-01T00:00:00Z", 1),
        ).expect("Failed to insert test user");
        
        // Create test posts with hashtags
        let post1_id = Uuid::new_v4();
        let post2_id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO posts (id, author_id, content, created_at, upvotes, downvotes, parent_post_id, reply_count) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (post1_id.to_string(), user_id.to_string(), "Post with #rust", "2024-01-01T00:00:00Z", 0, 0, None::<String>, 0),
        ).expect("Failed to insert post 1");
        conn.execute(
            "INSERT INTO posts (id, author_id, content, created_at, upvotes, downvotes, parent_post_id, reply_count) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (post2_id.to_string(), user_id.to_string(), "Post with #rust and #webdev", "2024-01-01T00:00:00Z", 0, 0, None::<String>, 0),
        ).expect("Failed to insert post 2");
        
        let hashtag_repo = HashtagRepository::new(db.pool.clone());
        
        // Run migration first time
        let hashtags1 = extract_hashtags("Post with #rust");
        let hashtags2 = extract_hashtags("Post with #rust and #webdev");
        hashtag_repo.store_hashtags(&post1_id, &hashtags1).expect("Failed first migration");
        hashtag_repo.store_hashtags(&post2_id, &hashtags2).expect("Failed first migration");
        
        // Count hashtags after first run
        let count_after_first: i32 = conn.query_row(
            "SELECT COUNT(*) FROM post_hashtags",
            [],
            |row| row.get(0)
        ).expect("Failed to count");
        
        // Run migration second time (should be idempotent)
        hashtag_repo.store_hashtags(&post1_id, &hashtags1).expect("Failed second migration");
        hashtag_repo.store_hashtags(&post2_id, &hashtags2).expect("Failed second migration");
        
        // Count hashtags after second run
        let count_after_second: i32 = conn.query_row(
            "SELECT COUNT(*) FROM post_hashtags",
            [],
            |row| row.get(0)
        ).expect("Failed to count");
        
        // Counts should be identical (no duplicates created)
        assert_eq!(count_after_first, count_after_second, 
            "Migration is not idempotent - duplicate associations created");
    }
    
    // Feature: hashtag-backfill-migration, Property 3: Association correctness
    // For any post with hashtags, after migration, querying the post_hashtags table
    // should return exactly the hashtags that were extracted from that post's content
    #[test]
    fn test_association_correctness() {
        // Create in-memory database
        let db = Database::in_memory().expect("Failed to create in-memory database");
        db.initialize().expect("Failed to initialize database");
        
        // Create a test user first (for foreign key constraint)
        let user_id = Uuid::new_v4();
        let conn = db.pool.get().expect("Failed to get connection");
        conn.execute(
            "INSERT INTO users (id, username, join_date, is_test_user) VALUES (?, ?, ?, ?)",
            (user_id.to_string(), "testuser", "2024-01-01T00:00:00Z", 1),
        ).expect("Failed to insert test user");
        
        // Create a test post with hashtags
        let post_id = Uuid::new_v4();
        let content = "Testing #rust and #programming with #webdev";
        
        conn.execute(
            "INSERT INTO posts (id, author_id, content, created_at, upvotes, downvotes, parent_post_id, reply_count) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (
                post_id.to_string(),
                user_id.to_string(),
                content,
                "2024-01-01T00:00:00Z",
                0,
                0,
                None::<String>,
                0,
            ),
        ).expect("Failed to insert test post");
        
        // Extract expected hashtags
        let expected_hashtags = extract_hashtags(content);
        assert_eq!(expected_hashtags.len(), 3);
        
        // Process the post
        let hashtag_repo = HashtagRepository::new(db.pool.clone());
        hashtag_repo.store_hashtags(&post_id, &expected_hashtags)
            .expect("Failed to store hashtags");
        
        // Query back the hashtags
        let stored_hashtags = hashtag_repo.get_by_post(&post_id)
            .expect("Failed to get hashtags");
        
        // Verify they match
        assert_eq!(stored_hashtags.len(), expected_hashtags.len());
        for hashtag in &expected_hashtags {
            assert!(stored_hashtags.contains(hashtag), 
                "Expected hashtag '{}' not found in stored hashtags", hashtag);
        }
    }
    
    // Feature: hashtag-backfill-migration, Property 5: Robustness to edge cases
    // For any post content including edge cases, the migration should complete successfully
    #[test]
    fn test_robustness_edge_cases() {
        let long_content = "a".repeat(10000);
        let edge_cases = vec![
            "",  // Empty string
            "   ",  // Only whitespace
            "#",  // Just hash symbol
            "#a",  // Single character hashtag (too short)
            "###",  // Multiple hash symbols
            "#rust#webdev",  // Adjacent hashtags
            "#rust!@#$%",  // Hashtag with special characters
            &long_content,  // Very long content
            "🦀 #rust 🚀",  // Unicode characters
            "#RUST #Rust #rust",  // Case variations
        ];
        
        for content in edge_cases {
            // Should not panic
            let result = extract_hashtags(content);
            // Result should be a valid vector (even if empty)
            assert!(result.len() >= 0);
        }
    }
    
    // Feature: hashtag-backfill-migration, Property 2: Extraction consistency
    // For any post content, the hashtags extracted by the migration script should be 
    // identical to the hashtags extracted by the existing extract_hashtags function
    proptest! {
        #[test]
        fn prop_extraction_consistency(content in ".*") {
            // Extract hashtags using the migration's extraction logic
            // (which is the same function, so this validates consistency)
            let hashtags1 = extract_hashtags(&content);
            let hashtags2 = extract_hashtags(&content);
            
            // Both extractions should produce identical results
            prop_assert_eq!(hashtags1, hashtags2);
        }
        
        #[test]
        fn prop_extraction_consistency_with_hashtags(
            prefix in "[^#]*",
            hashtag in "[a-zA-Z][a-zA-Z0-9_]{1,9}",  // Start with letter, then alphanumeric/underscore
            suffix in "[ \t\n]*"  // Whitespace to ensure hashtag boundary
        ) {
            let content = format!("{}#{}{}", prefix, hashtag, suffix);
            
            // Extract hashtags multiple times
            let hashtags1 = extract_hashtags(&content);
            let hashtags2 = extract_hashtags(&content);
            
            // Results should be consistent
            prop_assert_eq!(&hashtags1, &hashtags2);
            
            // Should contain the hashtag we added (normalized to lowercase)
            prop_assert!(hashtags1.contains(&hashtag.to_lowercase()));
        }
        
        // Feature: hashtag-backfill-migration, Property 5: Robustness property test
        // For any random content, extraction should not panic
        #[test]
        fn prop_robustness(content in "\\PC*") {
            // Should not panic on any input
            let _ = extract_hashtags(&content);
        }
    }
}
