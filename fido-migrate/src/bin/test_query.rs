use anyhow::{Context, Result};
use rusqlite::Connection;

fn main() -> Result<()> {
    let db_path = "../fido.db";
    let conn = Connection::open(db_path)
        .context("Failed to open database")?;
    
    println!("Testing post count query for 'rust' hashtag...\n");
    
    // Test the exact query from get_post_count
    let count: i32 = conn.query_row(
        "SELECT COUNT(DISTINCT ph.post_id) 
         FROM post_hashtags ph
         JOIN hashtags h ON ph.hashtag_id = h.id
         WHERE h.name = ?",
        ["rust"],
        |row| row.get(0)
    )?;
    
    println!("Post count for 'rust': {}", count);
    println!();
    
    // Show the actual posts
    println!("Posts with 'rust' hashtag:");
    let mut stmt = conn.prepare(
        "SELECT p.id, p.content
         FROM posts p
         JOIN post_hashtags ph ON p.id = ph.post_id
         JOIN hashtags h ON ph.hashtag_id = h.id
         WHERE h.name = 'rust'
         LIMIT 5"
    )?;
    
    let posts = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    
    for (i, post_result) in posts.enumerate() {
        let (id, content) = post_result?;
        println!("  {}. {} - {}", i + 1, &id[..8], &content[..50.min(content.len())]);
    }
    
    println!();
    
    // Check all hashtags and their counts
    println!("All hashtags with post counts:");
    let mut stmt = conn.prepare(
        "SELECT h.name, COUNT(DISTINCT ph.post_id) as count
         FROM hashtags h
         LEFT JOIN post_hashtags ph ON h.id = ph.hashtag_id
         GROUP BY h.name
         ORDER BY count DESC"
    )?;
    
    let hashtags = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
    })?;
    
    for hashtag_result in hashtags {
        let (name, count) = hashtag_result?;
        println!("  #{}: {} posts", name, count);
    }
    
    Ok(())
}
