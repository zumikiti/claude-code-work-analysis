use chrono::{DateTime, Utc, Datelike, TimeZone, FixedOffset};

use crate::models::ClaudeLogEntry;
use crate::scanner::ProjectScanner;

pub struct TimeRangeFilter {
    /// Start of the time range (inclusive)
    from_date: Option<DateTime<Utc>>,
    /// End of the time range (inclusive) 
    to_date: Option<DateTime<Utc>>,
    /// Project name filter (partial match)
    project_filter: Option<String>,
}

impl TimeRangeFilter {
    pub fn new(
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
        project_filter: Option<String>,
    ) -> Self {
        Self {
            from_date,
            to_date,
            project_filter,
        }
    }

    /// Create a filter for the last N days (in JST)
    pub fn last_days(days: i64) -> Self {
        let jst = FixedOffset::east_opt(9 * 3600).unwrap();
        let now_jst = Utc::now().with_timezone(&jst);
        let from_date_jst = now_jst - chrono::Duration::days(days);
        
        Self {
            from_date: Some(from_date_jst.with_timezone(&Utc)),
            to_date: Some(now_jst.with_timezone(&Utc)),
            project_filter: None,
        }
    }

    /// Create a filter for the current week (in JST)
    pub fn current_week() -> Self {
        let jst = FixedOffset::east_opt(9 * 3600).unwrap();
        let now_jst = Utc::now().with_timezone(&jst);
        let days_since_monday = now_jst.weekday().num_days_from_monday() as i64;
        let monday_jst = now_jst - chrono::Duration::days(days_since_monday);
        
        Self {
            from_date: Some(monday_jst.with_timezone(&Utc)),
            to_date: Some(now_jst.with_timezone(&Utc)),
            project_filter: None,
        }
    }

    /// Create a filter for a specific project
    pub fn for_project(project_name: impl Into<String>) -> Self {
        Self {
            from_date: None,
            to_date: None,
            project_filter: Some(project_name.into()),
        }
    }

    /// Filter entries based on the configured criteria
    pub fn filter_entries(&self, entries: Vec<ClaudeLogEntry>) -> Vec<ClaudeLogEntry> {
        entries
            .into_iter()
            .filter(|entry| self.matches_entry(entry))
            .collect()
    }

    /// Check if an entry matches the filter criteria
    pub fn matches_entry(&self, entry: &ClaudeLogEntry) -> bool {
        // Check time range
        if let Some(from_date) = self.from_date {
            if entry.timestamp < from_date {
                return false;
            }
        }

        if let Some(to_date) = self.to_date {
            if entry.timestamp > to_date {
                return false;
            }
        }

        // Check project filter
        if let Some(ref project_filter) = self.project_filter {
            if !self.matches_project(&entry.cwd, project_filter) {
                return false;
            }
        }

        true
    }

    /// Check if a project path matches the project filter
    fn matches_project(&self, project_path: &str, filter: &str) -> bool {
        // Simple case-insensitive substring match
        project_path.to_lowercase().contains(&filter.to_lowercase())
    }

    /// Filter project directories based on the project filter
    pub fn filter_project_directories(&self, project_dirs: Vec<std::path::PathBuf>) -> Vec<std::path::PathBuf> {
        if let Some(ref project_filter) = self.project_filter {
            project_dirs
                .into_iter()
                .filter(|dir| {
                    if let Some(project_name) = ProjectScanner::extract_project_name(dir) {
                        self.matches_project(&project_name, project_filter)
                    } else {
                        false
                    }
                })
                .collect()
        } else {
            project_dirs
        }
    }

    /// Get the effective date range for this filter
    pub fn get_date_range(&self) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
        (self.from_date, self.to_date)
    }

    /// Get the project filter
    pub fn get_project_filter(&self) -> Option<&str> {
        self.project_filter.as_deref()
    }

    /// Create a filter that combines this filter with another
    pub fn and(self, other: TimeRangeFilter) -> TimeRangeFilter {
        let from_date = match (self.from_date, other.from_date) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        let to_date = match (self.to_date, other.to_date) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        let project_filter = match (self.project_filter, other.project_filter) {
            (Some(a), Some(b)) => {
                // Combine project filters - require both to match
                Some(format!("{} {}", a, b))
            }
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        TimeRangeFilter {
            from_date,
            to_date,
            project_filter,
        }
    }

    /// Check if this filter has any active criteria
    pub fn is_empty(&self) -> bool {
        self.from_date.is_none() && self.to_date.is_none() && self.project_filter.is_none()
    }
}

impl Default for TimeRangeFilter {
    fn default() -> Self {
        Self {
            from_date: None,
            to_date: None,
            project_filter: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, FixedOffset};
    use uuid::Uuid;
    use crate::models::{MessageContent, MessageContentVariant, EntryType};

    fn create_test_entry(timestamp: DateTime<Utc>, cwd: &str) -> ClaudeLogEntry {
        ClaudeLogEntry {
            parent_uuid: None,
            is_sidechain: false,
            user_type: "external".to_string(),
            cwd: cwd.to_string(),
            session_id: Uuid::new_v4(),
            version: "1.0.0".to_string(),
            entry_type: EntryType::User,
            message: MessageContent {
                role: "user".to_string(),
                content: MessageContentVariant::String("test".to_string()),
                id: None,
                message_type: None,
                model: None,
                stop_reason: None,
                stop_sequence: None,
                usage: None,
            },
            uuid: Uuid::new_v4(),
            timestamp,
            request_id: None,
            tool_use_result: None,
        }
    }

    #[test]
    fn test_time_range_filter() {
        // JST timezone for testing
        let jst = FixedOffset::east_opt(9 * 3600).unwrap();
        
        // Create JST dates and convert to UTC for storage
        let from_date_jst = jst.with_ymd_and_hms(2025, 6, 25, 0, 0, 0).unwrap();
        let to_date_jst = jst.with_ymd_and_hms(2025, 6, 30, 23, 59, 59).unwrap();
        let from_date = from_date_jst.with_timezone(&Utc);
        let to_date = to_date_jst.with_timezone(&Utc);
        
        let filter = TimeRangeFilter::new(Some(from_date), Some(to_date), None);

        // Should match (JST time within range)
        let entry1_jst = jst.with_ymd_and_hms(2025, 6, 26, 12, 0, 0).unwrap();
        let entry1 = create_test_entry(
            entry1_jst.with_timezone(&Utc),
            "/test/project"
        );
        assert!(filter.matches_entry(&entry1));

        // Should not match (too early in JST)
        let entry2_jst = jst.with_ymd_and_hms(2025, 6, 24, 12, 0, 0).unwrap();
        let entry2 = create_test_entry(
            entry2_jst.with_timezone(&Utc),
            "/test/project"
        );
        assert!(!filter.matches_entry(&entry2));

        // Should not match (too late in JST)
        let entry3_jst = jst.with_ymd_and_hms(2025, 7, 1, 12, 0, 0).unwrap();
        let entry3 = create_test_entry(
            entry3_jst.with_timezone(&Utc),
            "/test/project"
        );
        assert!(!filter.matches_entry(&entry3));
    }

    #[test]
    fn test_project_filter() {
        let filter = TimeRangeFilter::for_project("test-project");

        // Should match
        let entry1 = create_test_entry(
            Utc::now(),
            "/Users/user/projects/test-project"
        );
        assert!(filter.matches_entry(&entry1));

        // Should not match
        let entry2 = create_test_entry(
            Utc::now(),
            "/Users/user/projects/other-project"
        );
        assert!(!filter.matches_entry(&entry2));
    }

    #[test]
    fn test_last_days_filter() {
        let filter = TimeRangeFilter::last_days(7);
        
        // Should match (recent)
        let entry1 = create_test_entry(
            Utc::now() - chrono::Duration::days(3),
            "/test/project"
        );
        assert!(filter.matches_entry(&entry1));

        // Should not match (too old)
        let entry2 = create_test_entry(
            Utc::now() - chrono::Duration::days(10),
            "/test/project"
        );
        assert!(!filter.matches_entry(&entry2));
    }

    #[test]
    fn test_empty_filter() {
        let filter = TimeRangeFilter::default();
        assert!(filter.is_empty());

        let entry = create_test_entry(Utc::now(), "/test/project");
        assert!(filter.matches_entry(&entry)); // Empty filter matches everything
    }
}
