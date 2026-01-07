use anyhow::{Context, Result};
use rusqlite::Connection;

fn main() -> Result<()> {
    println!("Following hashtags for alice...");

    let db_path = "../fido.db";
    let conn = Connection::open(db_path).context("Failed to open database")?;

    // Follow rust hashtag
    conn.execute(
        "INSERT OR IGNORE INTO user_hashtag_follows (user_id, hashtag_id, followed_at)
         SELECT '550e8400-e29b-41d4-a716-446655440001', h.id, 1732428000
         FROM hashtags h WHERE h.name = 'rust'",
        [],
    )?;

    // Follow sqlite hashtag
    conn.execute(
        "INSERT OR IGNORE INTO user_hashtag_follows (user_id, hashtag_id, followed_at)
         SELECT '550e8400-e29b-41d4-a716-446655440001', h.id, 1732428000
         FROM hashtags h WHERE h.name = 'sqlite'",
        [],
    )?;

    // Follow terminal hashtag
    conn.execute(
        "INSERT OR IGNORE INTO user_hashtag_follows (user_id, hashtag_id, followed_at)
         SELECT '550e8400-e29b-41d4-a716-446655440001', h.id, 1732428000
         FROM hashtags h WHERE h.name = 'terminal'",
        [],
    )?;

    // Follow ui hashtag
    conn.execute(
        "INSERT OR IGNORE INTO user_hashtag_follows (user_id, hashtag_id, followed_at)
         SELECT '550e8400-e29b-41d4-a716-446655440001', h.id, 1732428000
         FROM hashtags h WHERE h.name = 'ui'",
        [],
    )?;

    println!("✓ Followed hashtags for alice");

    // Verify
    let mut stmt = conn.prepare(
        "SELECT h.name FROM user_hashtag_follows uhf
         JOIN hashtags h ON uhf.hashtag_id = h.id
         WHERE uhf.user_id = '550e8400-e29b-41d4-a716-446655440001'",
    )?;

    let hashtags: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    println!("\nFollowed hashtags:");
    for hashtag in hashtags {
        println!("  - #{}", hashtag);
    }

    Ok(())
}
