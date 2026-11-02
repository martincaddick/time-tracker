use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Segment {
    pub id: i64,
    pub process: String,
    pub title: String,
    pub start: i64,
    pub end: i64,
    pub active_secs: f64,
    pub total_secs: f64,
    pub entry_id: Option<i64>,
}

impl Segment {
    /// Returns the duration of the segment in seconds.
    pub fn duration(&self) -> i64 {
        self.end - self.start
    }
}

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
            end: 1100,
            active_secs: 100.0,
            total_secs: 100.0,
            entry_id: None,
        };
        assert_eq!(segment.duration(), 100);
    }
}
