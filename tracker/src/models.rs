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

    #[test]
    fn test_segment_positive_duration() {
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

    #[test]
    fn test_segment_negative_duration() {
        let segment = Segment {
            id: 1,
            process: "test".to_string(),
            title: "test title".to_string(),
            start: 1500,
            end: 1000,
            active_secs: -500.0,
            total_secs: -500.0,
            entry_id: None,
        };
        assert_eq!(segment.duration(), -500);
    }

    #[test]
    fn test_segment_serialization() {
        let segment = Segment {
            id: 42,
            process: "Test Process".to_string(),
            title: "Test Title".to_string(),
            start: 1000,
            end: 2000,
            active_secs: 1000.0,
            total_secs: 1000.0,
            entry_id: Some(1),
        };

        let json = serde_json::to_string(&segment).expect("Failed to serialize Segment");
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"process\":\"Test Process\""));
        assert!(json.contains("\"title\":\"Test Title\""));
        assert!(json.contains("\"start\":1000"));
        assert!(json.contains("\"end\":2000"));
        assert!(json.contains("\"active_secs\":1000.0"));
        assert!(json.contains("\"total_secs\":1000.0"));
        assert!(json.contains("\"entry_id\":1"));

        let deserialized: Segment =
            serde_json::from_str(&json).expect("Failed to deserialize Segment");
        assert_eq!(segment, deserialized);
    }
}
