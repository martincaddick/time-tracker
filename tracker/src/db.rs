use rusqlite::Connection;
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

fn connect() -> rusqlite::Result<Connection> {
    let conn = Connection::open(data_dir().join("tracker.db"))?;
    conn.busy_timeout(Duration::from_secs(5))?;
    Ok(conn)
}

/// Run once at startup, before any other thread touches the database:
/// switches to WAL (so the page can read while the collector writes) and
/// creates the tables. Doing this in one place avoids a start-up race.
pub fn init() -> rusqlite::Result<()> {
    let conn = connect()?;
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
    connect()
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
