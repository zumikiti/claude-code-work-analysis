use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeLogEntry {
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<Uuid>,
    #[serde(rename = "isSidechain")]
    pub is_sidechain: bool,
    #[serde(rename = "userType")]
    pub user_type: String,
    pub cwd: String,
    #[serde(rename = "sessionId")]
    pub session_id: Uuid,
    pub version: String,
    #[serde(rename = "type")]
    pub entry_type: EntryType,
    pub message: MessageContent,
    pub uuid: Uuid,
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    #[serde(rename = "toolUseResult")]
    pub tool_use_result: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    pub role: String,
    pub content: MessageContentVariant,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub message_type: Option<String>,
    pub model: Option<String>,
    #[serde(rename = "stop_reason")]
    pub stop_reason: Option<String>,
    #[serde(rename = "stop_sequence")]
    pub stop_sequence: Option<String>,
    pub usage: Option<UsageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContentVariant {
    String(String),
    Array(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
    pub thinking: Option<String>,
    pub signature: Option<String>,
    pub id: Option<String>,
    pub name: Option<String>,
    pub input: Option<serde_json::Value>,
    #[serde(rename = "tool_use_id")]
    pub tool_use_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub cache_creation_input_tokens: Option<u32>,
    pub cache_read_input_tokens: Option<u32>,
    pub service_tier: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkSession {
    pub session_id: Uuid,
    pub project_path: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub entries: Vec<ClaudeLogEntry>,
    pub total_messages: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
}

#[derive(Debug, Clone)]
pub struct WorkAnalysis {
    pub sessions: Vec<WorkSession>,
    pub project_stats: HashMap<String, ProjectStats>,
    pub time_range: (DateTime<Utc>, DateTime<Utc>),
    pub total_sessions: usize,
    pub total_messages: usize,
    pub total_work_time: chrono::Duration,
}

#[derive(Debug, Clone)]
pub struct ProjectStats {
    pub project_name: String,
    pub total_sessions: usize,
    pub total_messages: usize,
    pub work_time: chrono::Duration,
    pub activity_types: HashMap<String, usize>,
    pub most_active_day: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum ActivityType {
    Coding,
    Debugging,
    Planning,
    Research,
    Documentation,
    Learning,
    Other,
}

impl ActivityType {
    pub fn from_message_content(content: &str) -> Self {
        let content_lower = content.to_lowercase();
        
        if content_lower.contains("implement") || content_lower.contains("write") 
            || content_lower.contains("create") || content_lower.contains("add") {
            ActivityType::Coding
        } else if content_lower.contains("debug") || content_lower.contains("fix") 
            || content_lower.contains("error") || content_lower.contains("bug") {
            ActivityType::Debugging
        } else if content_lower.contains("plan") || content_lower.contains("design") 
            || content_lower.contains("architect") {
            ActivityType::Planning
        } else if content_lower.contains("research") || content_lower.contains("investigate") 
            || content_lower.contains("analyze") {
            ActivityType::Research
        } else if content_lower.contains("document") || content_lower.contains("readme") 
            || content_lower.contains("comment") {
            ActivityType::Documentation
        } else if content_lower.contains("learn") || content_lower.contains("understand") 
            || content_lower.contains("explain") {
            ActivityType::Learning
        } else {
            ActivityType::Other
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityType::Coding => "Coding",
            ActivityType::Debugging => "Debugging", 
            ActivityType::Planning => "Planning",
            ActivityType::Research => "Research",
            ActivityType::Documentation => "Documentation",
            ActivityType::Learning => "Learning",
            ActivityType::Other => "Other",
        }
    }
}

impl Default for MessageContentVariant {
    fn default() -> Self {
        MessageContentVariant::String(String::new())
    }
}