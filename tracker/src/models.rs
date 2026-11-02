use serde::{Deserialize, Serialize};

/// Represents a single activity segment in the time tracker.
/// Each segment records a process, its title, and the time range it spanned.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Segment {
    /// Unique identifier for the segment.
    pub id: i64,
    /// Name of the process that was running.
    pub process: String,
    /// User-defined title for the activity.
    pub title: String,
    /// Unix timestamp representing the start of the segment.
    pub start: i64,
    /// Unix timestamp representing the end of the segment.
    pub end: i64,
    /// Total number of seconds the process was active.
    pub active_secs: f64,
    /// Total duration of the segment in seconds.
    pub total_secs: f64,
    /// Optional ID linking this segment to a specific entry.
    pub entry_id: Option<i64>,
}

impl Segment {
    /// Calculates the total duration of the segment in seconds.
    ///
    /// This is derived from the difference between the `end` and `start` timestamps.
    pub fn duration(&self) -> i64 {
        self.end - self.start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_zero_duration() {
        let segment = Segment {
            id: 1,
            process: "test".to_string(),
            title: "test title".to_string(),
            start: 1000,
            end: 1000,
            active_secs: 0.0,
            total_secs: 0.0,
            entry_id: None,
        };
        assert_eq!(segment.duration(), 0);
    }
}
