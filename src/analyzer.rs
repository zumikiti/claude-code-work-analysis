use anyhow::Result;
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::{
    ClaudeLogEntry, WorkSession, WorkAnalysis, ProjectStats, ActivityType, 
    MessageContentVariant, EntryType, ConversationSummary
};
use crate::scanner::ProjectScanner;
use crate::message_analyzer::MessageAnalyzer;

pub struct WorkAnalyzer {
    /// Minimum time between messages to consider them part of the same session
    session_gap_threshold: Duration,
    /// Minimum number of messages to consider a session meaningful
    min_session_messages: usize,
    /// Message analyzer for content analysis
    message_analyzer: MessageAnalyzer,
}

impl WorkAnalyzer {
    pub fn new() -> Self {
        Self {
            session_gap_threshold: Duration::hours(2), // 2 hours gap = new session
            min_session_messages: 3,
            message_analyzer: MessageAnalyzer::new(),
        }
    }

    pub fn with_session_gap(mut self, gap: Duration) -> Self {
        self.session_gap_threshold = gap;
        self
    }

    pub fn with_min_messages(mut self, min_messages: usize) -> Self {
        self.min_session_messages = min_messages;
        self
    }

    /// Analyze a collection of Claude log entries and produce work analysis
    pub fn analyze_entries(&self, entries: &[ClaudeLogEntry]) -> Result<WorkAnalysis> {
        if entries.is_empty() {
            // Use epoch time for empty entries instead of current time
            let epoch = DateTime::from_timestamp(0, 0).unwrap_or(Utc::now());
            return Ok(WorkAnalysis {
                sessions: Vec::new(),
                project_stats: HashMap::new(),
                time_range: (epoch, epoch),
                total_sessions: 0,
                total_messages: 0,
                total_work_time: Duration::zero(),
                conversation_summary: None,
            });
        }

        // Group entries by session
        let sessions = self.group_entries_into_sessions(entries);
        
        // Filter sessions by minimum message count
        let meaningful_sessions: Vec<WorkSession> = sessions
            .into_iter()
            .filter(|session| session.entries.len() >= self.min_session_messages)
            .collect();

        // Calculate project statistics
        let project_stats = self.calculate_project_stats(&meaningful_sessions);

        // Calculate time range
        let time_range = self.calculate_time_range(entries);

        // Calculate totals
        let total_sessions = meaningful_sessions.len();
        let total_messages = meaningful_sessions
            .iter()
            .map(|s| s.entries.len())
            .sum();
        let total_work_time = meaningful_sessions
            .iter()
            .map(|s| s.end_time - s.start_time)
            .fold(Duration::zero(), |acc, d| acc + d);

        // Generate conversation summary
        let conversation_summary = self.generate_conversation_summary(&meaningful_sessions);

        Ok(WorkAnalysis {
            sessions: meaningful_sessions,
            project_stats,
            time_range,
            total_sessions,
            total_messages,
            total_work_time,
            conversation_summary: Some(conversation_summary),
        })
    }

    /// Group entries into work sessions based on timing and project
    fn group_entries_into_sessions(&self, entries: &[ClaudeLogEntry]) -> Vec<WorkSession> {
        let mut sessions = Vec::new();
        let mut current_session_entries = Vec::new();
        let mut last_timestamp: Option<DateTime<Utc>> = None;
        let mut last_session_id: Option<Uuid> = None;
        let mut last_project_path: Option<String> = None;

        for entry in entries {
            let should_start_new_session = match (last_timestamp, &last_session_id, &last_project_path) {
                (Some(last_ts), Some(last_sid), Some(last_path)) => {
                    // Start new session if:
                    // 1. Time gap is too large
                    // 2. Session ID changed
                    // 3. Project path changed significantly
                    entry.timestamp - last_ts > self.session_gap_threshold
                        || entry.session_id != *last_sid
                        || !self.is_same_project(last_path, &entry.cwd)
                }
                _ => false, // First entry
            };

            if should_start_new_session && !current_session_entries.is_empty() {
                // Finalize current session
                if let Some(session) = self.create_session_from_entries(current_session_entries) {
                    sessions.push(session);
                }
                current_session_entries = Vec::new();
            }

            current_session_entries.push(entry.clone());
            last_timestamp = Some(entry.timestamp);
            last_session_id = Some(entry.session_id);
            last_project_path = Some(entry.cwd.clone());
        }

        // Don't forget the last session
        if !current_session_entries.is_empty() {
            if let Some(session) = self.create_session_from_entries(current_session_entries) {
                sessions.push(session);
            }
        }

        sessions
    }

    /// Create a WorkSession from a collection of entries
    fn create_session_from_entries(&self, entries: Vec<ClaudeLogEntry>) -> Option<WorkSession> {
        if entries.is_empty() {
            return None;
        }

        let session_id = entries[0].session_id;
        let project_path = entries[0].cwd.clone();
        let start_time = entries[0].timestamp;
        let end_time = entries.last()?.timestamp;

        let user_messages = entries
            .iter()
            .filter(|e| matches!(e.entry_type, EntryType::User))
            .count();
        
        let assistant_messages = entries
            .iter()
            .filter(|e| matches!(e.entry_type, EntryType::Assistant))
            .count();

        // Generate session summary
        let session_summary = self.message_analyzer.analyze_session(&entries);
        
        Some(WorkSession {
            session_id,
            project_path,
            start_time,
            end_time,
            total_messages: entries.len(),
            user_messages,
            assistant_messages,
            entries,
            summary: Some(session_summary),
        })
    }

    /// Check if two project paths represent the same project
    fn is_same_project(&self, path1: &str, path2: &str) -> bool {
        // Simple heuristic: if they share the same final directory name, they're the same project
        let extract_project_name = |path: &str| -> String {
            std::path::Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(path)
                .to_string()
        };

        extract_project_name(path1) == extract_project_name(path2)
    }

    /// Calculate statistics for each project
    fn calculate_project_stats(&self, sessions: &[WorkSession]) -> HashMap<String, ProjectStats> {
        let mut project_stats = HashMap::new();

        for session in sessions {
            let project_name = ProjectScanner::extract_project_name(
                std::path::Path::new(&session.project_path)
            ).unwrap_or_else(|| session.project_path.clone());

            let stats = project_stats
                .entry(project_name.clone())
                .or_insert_with(|| ProjectStats {
                    project_name: project_name.clone(),
                    total_sessions: 0,
                    total_messages: 0,
                    work_time: Duration::zero(),
                    activity_types: HashMap::new(),
                    most_active_day: None,
                    topic_analysis: None,
                });

            stats.total_sessions += 1;
            stats.total_messages += session.total_messages;
            stats.work_time = stats.work_time + (session.end_time - session.start_time);

            // Analyze activity types in this session
            for entry in &session.entries {
                if let EntryType::User = entry.entry_type {
                    let content = self.extract_message_content(&entry.message.content);
                    let activity_type = ActivityType::from_message_content(&content);
                    *stats.activity_types.entry(activity_type.as_str().to_string()).or_insert(0) += 1;
                }
            }

            // Update most active day
            let session_date = session.start_time.date_naive();
            match stats.most_active_day {
                None => stats.most_active_day = Some(session.start_time),
                Some(current_most_active) => {
                    if session_date != current_most_active.date_naive() {
                        // For simplicity, just use the latest session's date
                        // In a more sophisticated implementation, we'd track actual message counts per day
                        if session.start_time > current_most_active {
                            stats.most_active_day = Some(session.start_time);
                        }
                    }
                }
            }
        }

        // Generate topic analysis for each project
        for (project_name, stats) in project_stats.iter_mut() {
            let project_entries: Vec<ClaudeLogEntry> = sessions
                .iter()
                .filter(|session| {
                    ProjectScanner::extract_project_name(
                        std::path::Path::new(&session.project_path)
                    ).unwrap_or_else(|| session.project_path.clone()) == *project_name
                })
                .flat_map(|session| session.entries.clone())
                .collect();
            
            if !project_entries.is_empty() {
                let topic_analysis = self.message_analyzer.analyze_project_topics(&project_entries);
                stats.topic_analysis = Some(topic_analysis);
            }
        }

        project_stats
    }

    /// Extract readable content from message content variant
    fn extract_message_content(&self, content: &MessageContentVariant) -> String {
        match content {
            MessageContentVariant::String(s) => s.clone(),
            MessageContentVariant::Array(blocks) => {
                blocks
                    .iter()
                    .filter_map(|block| block.text.as_ref())
                    .cloned()
                    .collect::<Vec<String>>()
                    .join(" ")
            }
        }
    }

    /// Calculate the overall time range of the entries
    fn calculate_time_range(&self, entries: &[ClaudeLogEntry]) -> (DateTime<Utc>, DateTime<Utc>) {
        if entries.is_empty() {
            let now = Utc::now();
            return (now, now);
        }

        let min_time = entries
            .iter()
            .map(|e| e.timestamp)
            .min()
            .unwrap_or_else(Utc::now);

        let max_time = entries
            .iter()
            .map(|e| e.timestamp)
            .max()
            .unwrap_or_else(Utc::now);

        (min_time, max_time)
    }

    /// Get sessions for a specific project
    pub fn get_project_sessions<'a>(&self, analysis: &'a WorkAnalysis, project_name: &str) -> Vec<&'a WorkSession> {
        analysis
            .sessions
            .iter()
            .filter(|session| {
                ProjectScanner::extract_project_name(
                    std::path::Path::new(&session.project_path)
                ).map(|name| name.contains(project_name))
                .unwrap_or(false)
            })
            .collect()
    }

    /// Get sessions within a specific time range
    pub fn get_sessions_in_range<'a>(
        &self,
        analysis: &'a WorkAnalysis,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&'a WorkSession> {
        analysis
            .sessions
            .iter()
            .filter(|session| session.start_time >= start && session.end_time <= end)
            .collect()
    }

    /// Generate conversation summary from all sessions
    fn generate_conversation_summary(&self, sessions: &[WorkSession]) -> ConversationSummary {
        let sessions_with_summaries: Vec<(Vec<ClaudeLogEntry>, crate::models::SessionSummary)> = sessions
            .iter()
            .filter_map(|session| {
                if let Some(ref summary) = session.summary {
                    Some((session.entries.clone(), summary.clone()))
                } else {
                    None
                }
            })
            .collect();

        if sessions_with_summaries.is_empty() {
            return ConversationSummary {
                total_topics: 0,
                most_discussed_topics: Vec::new(),
                technology_usage: HashMap::new(),
                common_problems: Vec::new(),
                learning_progression: Vec::new(),
                productivity_insights: Vec::new(),
                overall_themes: Vec::new(),
            };
        }

        self.message_analyzer.analyze_conversations(&sessions_with_summaries)
    }
}

impl Default for WorkAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MessageContent, MessageContentVariant};
    use uuid::Uuid;

    fn create_test_entry(
        timestamp: DateTime<Utc>,
        session_id: Uuid,
        cwd: &str,
        entry_type: EntryType,
        content: &str,
    ) -> ClaudeLogEntry {
        ClaudeLogEntry {
            parent_uuid: None,
            is_sidechain: false,
            user_type: "external".to_string(),
            cwd: cwd.to_string(),
            session_id,
            version: "1.0.0".to_string(),
            entry_type,
            message: MessageContent {
                role: match entry_type {
                    EntryType::User => "user".to_string(),
                    EntryType::Assistant => "assistant".to_string(),
                },
                content: MessageContentVariant::String(content.to_string()),
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
    fn test_session_grouping() {
        let analyzer = WorkAnalyzer::new();
        let session_id = Uuid::new_v4();
        let base_time = Utc::now();

        let entries = vec![
            create_test_entry(base_time, session_id, "/project1", EntryType::User, "test 1"),
            create_test_entry(base_time + Duration::minutes(5), session_id, "/project1", EntryType::Assistant, "response 1"),
            create_test_entry(base_time + Duration::minutes(10), session_id, "/project1", EntryType::User, "test 2"),
            create_test_entry(base_time + Duration::minutes(15), session_id, "/project1", EntryType::Assistant, "response 2"),
        ];

        let sessions = analyzer.group_entries_into_sessions(&entries);
        
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].entries.len(), 4);
        assert_eq!(sessions[0].user_messages, 2);
        assert_eq!(sessions[0].assistant_messages, 2);
    }

    #[test]
    fn test_session_splitting_by_time() {
        let analyzer = WorkAnalyzer::new().with_session_gap(Duration::hours(1));
        let session_id = Uuid::new_v4();
        let base_time = Utc::now();

        let entries = vec![
            create_test_entry(base_time, session_id, "/project1", EntryType::User, "test 1"),
            create_test_entry(base_time + Duration::minutes(5), session_id, "/project1", EntryType::Assistant, "response 1"),
            // Long gap - should create new session
            create_test_entry(base_time + Duration::hours(2), session_id, "/project1", EntryType::User, "test 2"),
            create_test_entry(base_time + Duration::hours(2) + Duration::minutes(5), session_id, "/project1", EntryType::Assistant, "response 2"),
        ];

        let sessions = analyzer.group_entries_into_sessions(&entries);
        
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].entries.len(), 2);
        assert_eq!(sessions[1].entries.len(), 2);
    }

    #[test]
    fn test_activity_type_classification() {
        assert!(matches!(
            ActivityType::from_message_content("implement a new feature"),
            ActivityType::Coding
        ));
        
        assert!(matches!(
            ActivityType::from_message_content("fix this bug"),
            ActivityType::Debugging
        ));
        
        assert!(matches!(
            ActivityType::from_message_content("plan the architecture"),
            ActivityType::Planning
        ));
        
        assert!(matches!(
            ActivityType::from_message_content("research this topic"),
            ActivityType::Research
        ));
    }

    #[test]
    fn test_empty_entries_analysis() {
        let analyzer = WorkAnalyzer::new();
        let analysis = analyzer.analyze_entries(&[]).unwrap();
        
        assert_eq!(analysis.total_sessions, 0);
        assert_eq!(analysis.total_messages, 0);
        assert!(analysis.project_stats.is_empty());
    }
}