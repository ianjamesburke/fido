use anyhow::{Context, Result};
use clap::Parser;
use rusqlite::Connection;

/// Database Schema Inspector
/// 
/// This tool inspects a SQLite database and reports on the schema,
/// specifically checking for hashtag-related tables.
#[derive(Parser, Debug)]
#[command(name = "inspect-db")]
#[command(about = "Inspect Fido database schema", long_about = None)]
struct Args {
    /// Path to the SQLite database file
    #[arg(short, long, default_value = "./fido.db")]
    database: String,
}

#[derive(Debug)]
struct TableInfo {
    name: String,
    exists: bool,
    columns: Vec<ColumnInfo>,
}

#[derive(Debug)]
struct ColumnInfo {
    name: String,
    type_name: String,
    not_null: bool,
    pk: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("Fido Database Schema Inspector");
    println!("================================");
    println!();
    println!("Database: {}", args.database);
    println!();
    
    // Check if database file exists
    if !std::path::Path::new(&args.database).exists() {
        println!("❌ Database file not found: {}", args.database);
        return Ok(());
    }
    
    // Open database connection
    let conn = Connection::open(&args.database)
        .context("Failed to open database connection")?;
    
    println!("✓ Database file exists and is accessible");
    println!();
    
    // Check for required tables
    let required_tables = vec![
        "users",
        "posts",
        "hashtags",
        "post_hashtags",
        "user_hashtag_follows",
        "user_hashtag_activity",
        "votes",
    ];
    
    println!("Checking for required tables:");
    println!("-----------------------------");
    
    let mut all_tables_exist = true;
    let mut table_infos = Vec::new();
    
    for table_name in &required_tables {
        let exists = check_table_exists(&conn, table_name)?;
        let columns = if exists {
            get_table_columns(&conn, table_name)?
        } else {
            Vec::new()
        };
        
        table_infos.push(TableInfo {
            name: table_name.to_string(),
            exists,
            columns,
        });
        
        if exists {
            println!("  ✓ {} (exists)", table_name);
        } else {
            println!("  ❌ {} (MISSING)", table_name);
            all_tables_exist = false;
        }
    }
    
    println!();
    
    // Display detailed schema for hashtag tables
    println!("Hashtag Table Details:");
    println!("----------------------");
    
    for table_info in &table_infos {
        if table_info.name.contains("hashtag") && table_info.exists {
            println!();
            println!("Table: {}", table_info.name);
            println!("Columns:");
            for col in &table_info.columns {
                let pk_marker = if col.pk { " (PRIMARY KEY)" } else { "" };
                let null_marker = if col.not_null { " NOT NULL" } else { "" };
                println!("  - {} : {}{}{}", col.name, col.type_name, null_marker, pk_marker);
            }
        }
    }
    
    // Check for indexes
    println!();
    println!("Checking for hashtag indexes:");
    println!("------------------------------");
    
    let expected_indexes = vec![
        "idx_hashtags_name",
        "idx_post_hashtags_post",
        "idx_post_hashtags_hashtag",
        "idx_user_hashtag_follows_user",
        "idx_user_hashtag_activity_user",
    ];
    
    for index_name in &expected_indexes {
        let exists = check_index_exists(&conn, index_name)?;
        if exists {
            println!("  ✓ {}", index_name);
        } else {
            println!("  ❌ {} (MISSING)", index_name);
        }
    }
    
    // Count records in hashtag tables
    println!();
    println!("Record Counts:");
    println!("--------------");
    
    for table_name in &["posts", "hashtags", "post_hashtags", "user_hashtag_follows"] {
        if check_table_exists(&conn, table_name)? {
            let count = count_records(&conn, table_name)?;
            println!("  {} : {} records", table_name, count);
        }
    }
    
    // Summary
    println!();
    println!("Summary:");
    println!("--------");
    
    if all_tables_exist {
        println!("✓ All required tables exist");
        println!("✓ Database schema is ready for migration");
    } else {
        println!("❌ Some required tables are missing");
        println!("⚠️  Database schema needs to be initialized");
        println!();
        println!("To fix this, you need to:");
        println!("1. Ensure Database::initialize() is called on server startup");
        println!("2. Or manually run the schema SQL from fido-server/src/db/schema.rs");
    }
    
    Ok(())
}

fn check_table_exists(conn: &Connection, table_name: &str) -> Result<bool> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
        [table_name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn check_index_exists(conn: &Connection, index_name: &str) -> Result<bool> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?",
        [index_name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn get_table_columns(conn: &Connection, table_name: &str) -> Result<Vec<ColumnInfo>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table_name))?;
    
    let columns = stmt.query_map([], |row| {
        Ok(ColumnInfo {
            name: row.get(1)?,
            type_name: row.get(2)?,
            not_null: row.get::<_, i32>(3)? != 0,
            pk: row.get::<_, i32>(5)? != 0,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;
    
    Ok(columns)
}

fn count_records(conn: &Connection, table_name: &str) -> Result<i32> {
    let count: i32 = conn.query_row(
        &format!("SELECT COUNT(*) FROM {}", table_name),
        [],
        |row| row.get(0),
    )?;
    Ok(count)
}
