use std::collections::HashMap;

use crate::models::{
    ClaudeLogEntry, SessionSummary, ConversationSummary, TopicAnalysis,
    MessageContentVariant, EntryType
};

pub struct MessageAnalyzer {
    /// Technology keywords for detection
    tech_keywords: Vec<String>,
    /// Problem indicators
    problem_indicators: Vec<String>,
    /// Solution indicators  
    solution_indicators: Vec<String>,
    /// Learning indicators
    learning_indicators: Vec<String>,
}

impl MessageAnalyzer {
    pub fn new() -> Self {
        Self {
            tech_keywords: vec![
                "rust", "python", "javascript", "typescript", "react", "vue", "angular",
                "nodejs", "express", "fastapi", "django", "flask", "next.js", "nuxt",
                "docker", "kubernetes", "aws", "gcp", "azure", "postgresql", "mysql",
                "mongodb", "redis", "git", "github", "gitlab", "ci/cd", "terraform",
                "ansible", "jenkins", "webpack", "vite", "babel", "eslint", "prettier",
                "jest", "pytest", "cargo", "npm", "yarn", "pip", "api", "rest", "graphql",
                "sql", "nosql", "html", "css", "sass", "scss", "tailwind", "bootstrap"
            ].iter().map(|s| s.to_string()).collect(),
            
            problem_indicators: vec![
                "error", "bug", "issue", "problem", "fail", "broken", "not work",
                "doesn't work", "crash", "exception", "undefined", "null", "panic",
                "stuck", "confused", "help", "troubleshoot", "debug", "fix"
            ].iter().map(|s| s.to_string()).collect(),
            
            solution_indicators: vec![
                "solution", "fix", "resolve", "implement", "create", "build", "add",
                "update", "modify", "change", "refactor", "optimize", "improve",
                "configure", "setup", "install", "deploy"
            ].iter().map(|s| s.to_string()).collect(),
            
            learning_indicators: vec![
                "learn", "understand", "explain", "how to", "what is", "why",
                "tutorial", "guide", "documentation", "example", "best practice",
                "pattern", "concept", "theory", "principle"
            ].iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Analyze a single session and generate summary
    pub fn analyze_session(&self, entries: &[ClaudeLogEntry]) -> SessionSummary {
        let mut key_discussions = Vec::new();
        let mut problems_addressed = Vec::new();
        let mut solutions_proposed = Vec::new();
        let mut learning_moments = Vec::new();
        
        let mut tech_mentions: HashMap<String, usize> = HashMap::new();
        let mut topic_keywords: HashMap<String, usize> = HashMap::new();
        
        for entry in entries {
            let content = self.extract_text_content(&entry.message.content);
            let content_lower = content.to_lowercase();
            
            // Detect technologies
            for tech in &self.tech_keywords {
                if content_lower.contains(tech) {
                    *tech_mentions.entry(tech.clone()).or_insert(0) += 1;
                }
            }
            
            // Analyze based on entry type
            match entry.entry_type {
                EntryType::User => {
                    // Extract user questions and requests
                    if self.contains_any(&content_lower, &self.problem_indicators) {
                        problems_addressed.push(self.extract_key_phrase(&content, 100));
                    }
                    
                    if self.contains_any(&content_lower, &self.learning_indicators) {
                        learning_moments.push(self.extract_key_phrase(&content, 100));
                    }
                    
                    // Extract topics from user messages
                    let topics = self.extract_topics(&content);
                    for topic in topics {
                        *topic_keywords.entry(topic).or_insert(0) += 1;
                    }
                }
                EntryType::Assistant => {
                    // Extract solutions from assistant responses
                    if self.contains_any(&content_lower, &self.solution_indicators) {
                        solutions_proposed.push(self.extract_key_phrase(&content, 150));
                    }
                    
                    // Extract key discussions
                    if content.len() > 200 {
                        key_discussions.push(self.extract_key_phrase(&content, 200));
                    }
                }
            }
        }
        
        // Sort and filter results
        let mut technologies_mentioned: Vec<String> = tech_mentions
            .into_iter()
            .filter(|(_, count)| *count >= 1) // At least 1 mention
            .map(|(tech, _)| tech)
            .collect();
        technologies_mentioned.sort();
        
        let mut main_topics: Vec<String> = topic_keywords
            .into_iter()
            .filter(|(_, count)| *count >= 1) // At least 1 mention
            .map(|(topic, _)| topic)
            .collect();
        main_topics.sort();
        
        // Generate overall summary
        let overall_summary = self.generate_session_summary(
            &main_topics,
            &technologies_mentioned,
            &problems_addressed,
            &solutions_proposed
        );
        
        SessionSummary {
            main_topics,
            key_discussions: key_discussions.into_iter().take(5).collect(),
            technologies_mentioned,
            problems_addressed: problems_addressed.into_iter().take(5).collect(),
            solutions_proposed: solutions_proposed.into_iter().take(5).collect(),
            learning_moments: learning_moments.into_iter().take(3).collect(),
            overall_summary,
        }
    }
    
    /// Analyze multiple sessions and generate conversation summary
    pub fn analyze_conversations(&self, sessions_with_summaries: &[(Vec<ClaudeLogEntry>, SessionSummary)]) -> ConversationSummary {
        let mut all_topics: HashMap<String, usize> = HashMap::new();
        let mut tech_usage: HashMap<String, usize> = HashMap::new();
        let mut common_problems = Vec::new();
        let mut learning_progression = Vec::new();
        
        for (_, summary) in sessions_with_summaries {
            // Aggregate topics
            for topic in &summary.main_topics {
                *all_topics.entry(topic.clone()).or_insert(0) += 1;
            }
            
            // Aggregate technologies
            for tech in &summary.technologies_mentioned {
                *tech_usage.entry(tech.clone()).or_insert(0) += 1;
            }
            
            // Collect problems and learning
            common_problems.extend(summary.problems_addressed.clone());
            learning_progression.extend(summary.learning_moments.clone());
        }
        
        // Sort topics by frequency
        let mut most_discussed_topics: Vec<(String, usize)> = all_topics.into_iter().collect();
        most_discussed_topics.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Generate productivity insights
        let productivity_insights = self.generate_productivity_insights(sessions_with_summaries);
        
        // Extract overall themes
        let overall_themes = self.extract_overall_themes(&most_discussed_topics, &tech_usage);
        
        ConversationSummary {
            total_topics: most_discussed_topics.len(),
            most_discussed_topics: most_discussed_topics.into_iter().take(10).collect(),
            technology_usage: tech_usage,
            common_problems: self.deduplicate_and_limit(common_problems, 10),
            learning_progression: self.deduplicate_and_limit(learning_progression, 10),
            productivity_insights,
            overall_themes,
        }
    }
    
    /// Generate topic analysis for a project
    pub fn analyze_project_topics(&self, all_entries: &[ClaudeLogEntry]) -> TopicAnalysis {
        let mut problem_categories: HashMap<String, usize> = HashMap::new();
        let mut complexity_indicators = Vec::new();
        
        let mut topic_frequency: HashMap<String, usize> = HashMap::new();
        let mut tech_frequency: HashMap<String, usize> = HashMap::new();
        
        for entry in all_entries {
            let content = self.extract_text_content(&entry.message.content);
            let content_lower = content.to_lowercase();
            
            // Count topic frequencies
            let topics = self.extract_topics(&content);
            for topic in topics {
                *topic_frequency.entry(topic).or_insert(0) += 1;
            }
            
            // Count technology frequencies
            for tech in &self.tech_keywords {
                if content_lower.contains(tech) {
                    *tech_frequency.entry(tech.clone()).or_insert(0) += 1;
                }
            }
            
            // Categorize problems
            if let EntryType::User = entry.entry_type {
                let problem_category = self.categorize_problem(&content_lower);
                if !problem_category.is_empty() {
                    *problem_categories.entry(problem_category).or_insert(0) += 1;
                }
            }
            
            // Detect complexity indicators
            if self.is_complex_discussion(&content) {
                complexity_indicators.push(self.extract_key_phrase(&content, 80));
            }
        }
        
        // Sort and categorize topics
        let mut sorted_topics: Vec<(String, usize)> = topic_frequency.into_iter().collect();
        sorted_topics.sort_by(|a, b| b.1.cmp(&a.1));
        
        let primary_topics: Vec<String> = sorted_topics.iter().take(5).map(|(topic, _)| topic.clone()).collect();
        let secondary_topics: Vec<String> = sorted_topics.iter().skip(5).take(10).map(|(topic, _)| topic.clone()).collect();
        
        // Extract technical stack
        let mut technical_stack: Vec<String> = tech_frequency
            .into_iter()
            .filter(|(_, count)| *count >= 3)
            .map(|(tech, _)| tech)
            .collect();
        technical_stack.sort();
        
        // Generate solution patterns
        let solution_patterns = self.extract_solution_patterns(all_entries);
        
        TopicAnalysis {
            primary_topics,
            secondary_topics,
            technical_stack,
            problem_categories,
            solution_patterns,
            complexity_indicators: complexity_indicators.into_iter().take(5).collect(),
        }
    }
    
    // Helper methods
    fn extract_text_content(&self, content: &MessageContentVariant) -> String {
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
    
    fn contains_any(&self, text: &str, keywords: &[String]) -> bool {
        keywords.iter().any(|keyword| text.contains(keyword))
    }
    
    fn extract_key_phrase(&self, text: &str, max_length: usize) -> String {
        let sentences: Vec<&str> = text.split('.').collect();
        for sentence in sentences {
            let sentence = sentence.trim();
            if sentence.chars().count() <= max_length && sentence.chars().count() > 10 {
                return sentence.to_string();
            }
        }
        
        // Fallback to truncated text using char boundaries
        if text.chars().count() <= max_length {
            text.to_string()
        } else {
            let truncated: String = text.chars().take(max_length).collect();
            format!("{}...", truncated)
        }
    }
    
    fn extract_topics(&self, content: &str) -> Vec<String> {
        let mut topics = Vec::new();
        
        // Simple keyword extraction - in a real implementation,
        // you might use NLP libraries or more sophisticated methods
        let content_lower = content.to_lowercase();
        let words: Vec<&str> = content_lower
            .split_whitespace()
            .filter(|word| word.len() > 3)
            .collect();
        
        // Look for potential topics (nouns, technical terms)
        for window in words.windows(2) {
            let phrase = window.join(" ");
            if self.is_potential_topic(&phrase) {
                topics.push(phrase);
            }
        }
        
        // Also include single important words
        for word in &words {
            if self.is_important_single_word(word) {
                topics.push(word.to_string());
            }
        }
        
        topics
    }
    
    fn is_potential_topic(&self, phrase: &str) -> bool {
        // Simple heuristics for topic detection
        phrase.contains("implement") ||
        phrase.contains("create") ||
        phrase.contains("build") ||
        phrase.contains("design") ||
        phrase.contains("configure") ||
        phrase.contains("setup")
    }
    
    fn is_important_single_word(&self, word: &str) -> bool {
        self.tech_keywords.contains(&word.to_string()) ||
        word.len() > 6 && !word.chars().all(|c| c.is_ascii_lowercase())
    }
    
    fn generate_session_summary(&self, topics: &[String], tech: &[String], problems: &[String], solutions: &[String]) -> String {
        let mut summary_parts = Vec::new();
        
        if !topics.is_empty() {
            summary_parts.push(format!("主要トピック: {}", topics.join(", ")));
        }
        
        if !tech.is_empty() {
            summary_parts.push(format!("使用技術: {}", tech.join(", ")));
        }
        
        if !problems.is_empty() {
            summary_parts.push(format!("解決した課題数: {}", problems.len()));
        }
        
        if !solutions.is_empty() {
            summary_parts.push(format!("提案された解決策数: {}", solutions.len()));
        }
        
        if summary_parts.is_empty() {
            "一般的な技術相談セッション".to_string()
        } else {
            summary_parts.join(" | ")
        }
    }
    
    fn categorize_problem(&self, content: &str) -> String {
        if content.contains("error") || content.contains("exception") || content.contains("crash") {
            "Runtime Error".to_string()
        } else if content.contains("compile") || content.contains("build") || content.contains("syntax") {
            "Build/Compile Issue".to_string()
        } else if content.contains("performance") || content.contains("slow") || content.contains("optimize") {
            "Performance Issue".to_string()
        } else if content.contains("config") || content.contains("setup") || content.contains("install") {
            "Configuration Issue".to_string()
        } else if content.contains("design") || content.contains("architecture") || content.contains("pattern") {
            "Design Question".to_string()
        } else if self.contains_any(content, &self.problem_indicators) {
            "General Problem".to_string()
        } else {
            String::new()
        }
    }
    
    fn is_complex_discussion(&self, content: &str) -> bool {
        content.len() > 500 &&
        (content.contains("architecture") ||
         content.contains("design pattern") ||
         content.contains("best practice") ||
         content.contains("scalability") ||
         content.contains("performance") ||
         content.contains("security"))
    }
    
    fn extract_solution_patterns(&self, entries: &[ClaudeLogEntry]) -> Vec<String> {
        let mut patterns = Vec::new();
        
        for entry in entries {
            if let EntryType::Assistant = entry.entry_type {
                let content = self.extract_text_content(&entry.message.content);
                let content_lower = content.to_lowercase();
                
                if content_lower.contains("pattern") || content_lower.contains("approach") {
                    patterns.push(self.extract_key_phrase(&content, 120));
                }
            }
        }
        
        self.deduplicate_and_limit(patterns, 5)
    }
    
    fn generate_productivity_insights(&self, sessions: &[(Vec<ClaudeLogEntry>, SessionSummary)]) -> Vec<String> {
        let mut insights = Vec::new();
        
        if sessions.len() > 5 {
            insights.push("定期的な開発活動が見られます".to_string());
        }
        
        let tech_diversity: std::collections::HashSet<String> = sessions
            .iter()
            .flat_map(|(_, summary)| summary.technologies_mentioned.clone())
            .collect();
        
        if tech_diversity.len() > 5 {
            insights.push("多様な技術スタックを使用しています".to_string());
        }
        
        let total_problems: usize = sessions
            .iter()
            .map(|(_, summary)| summary.problems_addressed.len())
            .sum();
        
        if total_problems > 10 {
            insights.push("問題解決スキルが積極的に活用されています".to_string());
        }
        
        insights
    }
    
    fn extract_overall_themes(&self, topics: &[(String, usize)], tech: &HashMap<String, usize>) -> Vec<String> {
        let mut themes = Vec::new();
        
        // Analyze dominant technologies
        if let Some((dominant_tech, _)) = tech.iter().max_by_key(|(_, count)| *count) {
            themes.push(format!("{}開発が中心", dominant_tech));
        }
        
        // Analyze topic patterns
        let total_topics = topics.len();
        if total_topics > 20 {
            themes.push("幅広いトピックをカバー".to_string());
        } else if total_topics > 5 {
            themes.push("集中的な学習・開発".to_string());
        }
        
        themes
    }
    
    fn deduplicate_and_limit(&self, mut items: Vec<String>, limit: usize) -> Vec<String> {
        items.sort();
        items.dedup();
        items.into_iter().take(limit).collect()
    }
}

impl Default for MessageAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MessageContent, MessageContentVariant};
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_entry(entry_type: EntryType, content: &str) -> ClaudeLogEntry {
        ClaudeLogEntry {
            parent_uuid: None,
            is_sidechain: false,
            user_type: "external".to_string(),
            cwd: "/test".to_string(),
            session_id: Uuid::new_v4(),
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
            timestamp: Utc::now(),
            request_id: None,
            tool_use_result: None,
        }
    }
    
    #[test]
    fn test_session_analysis() {
        let analyzer = MessageAnalyzer::new();
        let entries = vec![
            create_test_entry(EntryType::User, "I have an error with my Rust code"),
            create_test_entry(EntryType::Assistant, "Let me help you fix that error. The solution is to implement proper error handling"),
        ];
        
        let summary = analyzer.analyze_session(&entries);
        
        assert!(!summary.problems_addressed.is_empty());
        assert!(!summary.solutions_proposed.is_empty());
        assert!(summary.technologies_mentioned.contains(&"rust".to_string()));
    }
    
    #[test]
    fn test_technology_detection() {
        let analyzer = MessageAnalyzer::new();
        let entries = vec![
            create_test_entry(EntryType::User, "I'm working with React and TypeScript"),
        ];
        
        let summary = analyzer.analyze_session(&entries);
        
        assert!(summary.technologies_mentioned.contains(&"react".to_string()));
        assert!(summary.technologies_mentioned.contains(&"typescript".to_string()));
    }
}