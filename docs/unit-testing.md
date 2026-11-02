# Unit Testing

This document outlines the unit testing strategy for the Time Tracker project.

## Testing Philosophy

We aim for high test coverage by separating side-effect-heavy operations from pure logic.
- **Pure Logic**: Functions that only operate on data (like `Segment` methods) should be tested with standard unit tests.
- **Database Interactions**: Since the database is a key dependency, we use SQLite's in-memory database feature (`Connection::open_in_memory()`) to ensure tests are fast, isolated, and do not leave side effects on the local filesystem.

## Database Testing Example

When testing database interactions in `db.rs`, we use an in-memory connection. This allows us to run the initialization logic and verify table creation or data insertion without needing a real `.db` file.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

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
}
```

## Logic Testing Example

Pure functions, like those in `models.rs`, are tested by providing inputs and asserting expected outputs. These do not require any external dependencies or mocks.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_duration() {
        let segment = Segment {
            id: 1,
            process: "test".to_string(),
            title: "test title".to_string(),
            start: 1000,
            end: 1500,
            active_secs: 500.0,
            total_secs: 500.0,
            entry_id: None,
        };
        assert_eq!(segment.duration(), 500);
    }
}
```

## Running Tests

To run all tests, use the build script:
```bash
./build.sh
```
