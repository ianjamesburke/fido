use chrono::{DateTime, Utc};
use rusqlite::types::Type;
use rusqlite::Error;
use uuid::Uuid;

use fido_types::VoteDirection;

fn conversion_error<E: std::error::Error + Send + Sync + 'static>(col: usize, err: E) -> Error {
    Error::FromSqlConversionFailure(col, Type::Text, Box::new(err))
}

pub fn parse_uuid(value: &str, col: usize) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(value).map_err(|e| conversion_error(col, e))
}

pub fn parse_optional_uuid(value: Option<&str>, col: usize) -> rusqlite::Result<Option<Uuid>> {
    match value {
        Some(value) => parse_uuid(value, col).map(Some),
        None => Ok(None),
    }
}

pub fn parse_datetime(value: &str, col: usize) -> rusqlite::Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|e| conversion_error(col, e))?;
    Ok(parsed.with_timezone(&Utc))
}

pub fn parse_vote_direction(value: &str, col: usize) -> rusqlite::Result<VoteDirection> {
    VoteDirection::parse(value).ok_or_else(|| {
        conversion_error(
            col,
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid vote direction"),
        )
    })
}
