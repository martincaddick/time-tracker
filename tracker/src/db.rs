use crate::models::Segment;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::Duration;

pub fn data_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    let dir = PathBuf::from(base).join("tracker");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn connect(path: Option<&str>) -> rusqlite::Result<Connection> {
    let db_path = match path {
        Some(p) => p.to_string(),
        None => data_dir().join("tracker.db").to_string_lossy().into_owned(),
    };
    let conn = Connection::open(db_path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    Ok(conn)
}

/// Run once at startup, before any other thread touches the database:
/// switches to WAL (so the page can read while the collector writes) and
/// creates the tables. Doing this in one place avoids a start-up race.
pub fn init() -> rusqlite::Result<()> {
    let conn = connect(None)?;
    init_db(&conn)
}

/// Internal initialization logic that takes an existing connection.
pub fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    let _mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS segments (
            id          INTEGER PRIMARY KEY,
            process     TEXT    NOT NULL,
            title       TEXT    NOT NULL,
            start       INTEGER NOT NULL,   -- unix seconds, UTC
            end         INTEGER NOT NULL,
            active_secs REAL    NOT NULL,
            total_secs  REAL    NOT NULL,
            entry_id    INTEGER             -- set once logged into an entry
        );
        CREATE INDEX IF NOT EXISTS idx_segments_start ON segments(start);
        CREATE TABLE IF NOT EXISTS entries (
            id          INTEGER PRIMARY KEY,
            start       INTEGER NOT NULL,
            end         INTEGER NOT NULL,
            description TEXT    NOT NULL,
            project     TEXT    NOT NULL
        );",
    )
}

/// A plain connection for a worker thread (call `init` first).
pub fn open() -> rusqlite::Result<Connection> {
    connect(None)
}

pub fn log(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_dir().join("tracker.log"))
    {
        let _ = writeln!(f, "{msg}");
    }
}

pub fn insert_segment(conn: &Connection, segment: &Segment) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO segments (process, title, start, end, active_secs, total_secs, entry_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            segment.process,
            segment.title,
            segment.start,
            segment.end,
            segment.active_secs,
            segment.total_secs,
            segment.entry_id,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_all_segments(conn: &Connection) -> rusqlite::Result<Vec<Segment>> {
    let mut stmt = conn.prepare("SELECT id, process, title, start, end, active_secs, total_secs, entry_id FROM segments ORDER BY start ASC")?;
    let segment_iter = stmt.query_map([], |row| {
        Ok(Segment {
            id: row.get(0)?,
            process: row.get(1)?,
            title: row.get(2)?,
            start: row.get(3)?,
            end: row.get(4)?,
            active_secs: row.get(5)?,
            total_secs: row.get(6)?,
            entry_id: row.get(7)?,
        })
    })?;

    let mut segments = Vec::new();
    for segment in segment_iter {
        segments.push(segment?);
    }
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Tests that the database initialization correctly creates the required tables.
    /// Verifies both the 'segments' and 'entries' tables exist in the schema.
    #[test]
    fn test_init_creates_tables() {
        // Use an in-memory database for testing
        let conn = Connection::open_in_memory().expect("Failed to open in-memory db");

        // Pass the in-memory connection to our init logic
        init_db(&conn).expect("Failed to initialize database tables");

        // Verify that the 'segments' table exists
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<String>, _>>()
            .unwrap();

        assert!(tables.contains(&"segments".to_string()));
        assert!(tables.contains(&"entries".to_string()));
    }

    /// Tests that a segment can be successfully inserted into the database
    /// and then retrieved using `get_all_segments`.
    /// Verifies that the returned data matches the input.
    #[test]
    fn test_insert_and_get_segments() {
        // Use an in-memory database for testing
        let conn = Connection::open_in_memory().expect("Failed to open in-memory db");

        // Initialize tables
        init_db(&conn).expect("Failed to initialize database tables");

        let segment = Segment {
            id: 0, // SQLite will assign this
            process: "Test Process".to_string(),
            title: "Test Title".to_string(),
            start: 1000,
            end: 2000,
            active_secs: 1000.0,
            total_secs: 1000.0,
            entry_id: None,
        };

        // Test insertion
        let _id = insert_segment(&conn, &segment).expect("Failed to insert segment");

        // Test retrieval
        let segments = get_all_segments(&conn).expect("Failed to get segments");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].process, "Test Process");
        assert_eq!(segments[0].title, "Test Title");
        assert_eq!(segments[0].start, 1000);
        assert_eq!(segments[0].end, 2000);
    }
}
