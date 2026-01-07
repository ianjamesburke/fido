// Quick diagnostic to check database state
use fido_server::db::Database;

fn main() -> anyhow::Result<()> {
    let db = Database::new("../fido.db")?;
    let conn = db.pool.get()?;

    println!("=== Database Diagnostic ===\n");

    // Count post_hashtags
    let ph_count: i32 =
        conn.query_row("SELECT COUNT(*) FROM post_hashtags", [], |row| row.get(0))?;
    println!("Total post_hashtags entries: {}", ph_count);

    // Count unique hashtags
    let h_count: i32 = conn.query_row("SELECT COUNT(*) FROM hashtags", [], |row| row.get(0))?;
    println!("Total unique hashtags: {}", h_count);

    // Show hashtags
    println!("\n=== Hashtags in database ===");
    let mut stmt = conn.prepare("SELECT name FROM hashtags ORDER BY name")?;
    let hashtags = stmt.query_map([], |row| row.get::<_, String>(0))?;
    for (i, hashtag) in hashtags.enumerate() {
        println!("  {}. {}", i + 1, hashtag?);
    }

    // Check for 'fido' hashtag specifically
    println!("\n=== Checking for 'fido' hashtag ===");
    let fido_exists: i32 = conn.query_row(
        "SELECT COUNT(*) FROM hashtags WHERE name = 'fido'",
        [],
        |row| row.get(0),
    )?;
    println!(
        "'fido' hashtag exists: {}",
        if fido_exists > 0 { "YES" } else { "NO" }
    );

    if fido_exists > 0 {
        // Count posts with fido hashtag
        let fido_post_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM post_hashtags ph 
             JOIN hashtags h ON ph.hashtag_id = h.id 
             WHERE h.name = 'fido'",
            [],
            |row| row.get(0),
        )?;
        println!("Posts tagged with 'fido': {}", fido_post_count);

        // Show sample posts with fido hashtag
        println!("\n=== Sample posts with #fido ===");
        let mut stmt = conn.prepare(
            "SELECT p.content FROM posts p
             JOIN post_hashtags ph ON p.id = ph.post_id
             JOIN hashtags h ON ph.hashtag_id = h.id
             WHERE h.name = 'fido'
             LIMIT 5",
        )?;
        let posts = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for (i, post) in posts.enumerate() {
            let content = post?;
            let preview = if content.len() > 60 {
                format!("{}...", &content[..60])
            } else {
                content
            };
            println!("  {}. {}", i + 1, preview);
        }
    }

    // Check posts that have #fido in content but no hashtag association
    println!("\n=== Posts with #fido in content but NO hashtag association ===");
    let orphaned: i32 = conn.query_row(
        "SELECT COUNT(*) FROM posts p
         WHERE p.content LIKE '%#fido%'
         AND p.id NOT IN (
             SELECT ph.post_id FROM post_hashtags ph
             JOIN hashtags h ON ph.hashtag_id = h.id
             WHERE h.name = 'fido'
         )",
        [],
        |row| row.get(0),
    )?;
    println!("Orphaned posts: {}", orphaned);

    if orphaned > 0 {
        println!("\nThese posts have #fido in content but weren't migrated:");
        let mut stmt = conn.prepare(
            "SELECT content FROM posts p
             WHERE p.content LIKE '%#fido%'
             AND p.id NOT IN (
                 SELECT ph.post_id FROM post_hashtags ph
                 JOIN hashtags h ON ph.hashtag_id = h.id
                 WHERE h.name = 'fido'
             )
             LIMIT 5",
        )?;
        let posts = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for (i, post) in posts.enumerate() {
            let content = post?;
            let preview = if content.len() > 60 {
                format!("{}...", &content[..60])
            } else {
                content
            };
            println!("  {}. {}", i + 1, preview);
        }
    }

    Ok(())
}
