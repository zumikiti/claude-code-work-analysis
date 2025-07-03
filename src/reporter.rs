use anyhow::Result;
use chrono::{Timelike, TimeZone, FixedOffset};
use std::collections::HashMap;

use crate::models::WorkAnalysis;

pub struct ReportGenerator {
    /// Include detailed session information in reports
    include_session_details: bool,
    /// Maximum number of sessions to detail in reports
    max_detailed_sessions: usize,
}

impl ReportGenerator {
    pub fn new() -> Self {
        Self {
            include_session_details: true,
            max_detailed_sessions: 10,
        }
    }

    pub fn with_session_details(mut self, include: bool) -> Self {
        self.include_session_details = include;
        self
    }

    pub fn with_max_sessions(mut self, max: usize) -> Self {
        self.max_detailed_sessions = max;
        self
    }

    /// Generate a comprehensive markdown report
    pub fn generate_markdown_report(&self, analysis: &WorkAnalysis) -> Result<String> {
        let mut report = String::new();

        // Header
        report.push_str(&self.generate_header(analysis));
        report.push_str("\n\n");

        // Executive Summary
        report.push_str("## 📊 Executive Summary\n\n");
        report.push_str(&self.generate_executive_summary(analysis));
        report.push_str("\n\n");

        // Project Breakdown
        report.push_str("## 🚀 Project Breakdown\n\n");
        report.push_str(&self.generate_project_breakdown(analysis));
        report.push_str("\n\n");

        // Activity Analysis
        report.push_str("## 🔍 Activity Analysis\n\n");
        report.push_str(&self.generate_activity_analysis(analysis));
        report.push_str("\n\n");

        // Time Analysis
        report.push_str("## ⏰ Time Analysis\n\n");
        report.push_str(&self.generate_time_analysis(analysis));
        report.push_str("\n\n");

        // Conversation Summary
        report.push_str("## 💭 Conversation Summary\n\n");
        report.push_str(&self.generate_conversation_summary_section(analysis));
        report.push_str("\n\n");

        // Session Details (if enabled)
        if self.include_session_details {
            report.push_str("## 💬 Recent Sessions\n\n");
            report.push_str(&self.generate_session_details(analysis));
            report.push_str("\n\n");
        }

        // Recommendations
        report.push_str("## 💡 Insights & Recommendations\n\n");
        report.push_str(&self.generate_recommendations(analysis));

        Ok(report)
    }

    /// Generate a JSON report
    pub fn generate_json_report(&self, analysis: &WorkAnalysis) -> Result<String> {
        let json_data = serde_json::json!({
            "summary": {
                "total_sessions": analysis.total_sessions,
                "total_messages": analysis.total_messages,
                "total_work_time_hours": analysis.total_work_time.num_hours(),
                "time_range": {
                    "start": analysis.time_range.0.with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()).to_rfc3339(),
                    "end": analysis.time_range.1.with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()).to_rfc3339()
                }
            },
            "projects": analysis.project_stats.iter().map(|(name, stats)| {
                serde_json::json!({
                    "name": name,
                    "sessions": stats.total_sessions,
                    "messages": stats.total_messages,
                    "work_time_hours": stats.work_time.num_hours(),
                    "activity_types": stats.activity_types
                })
            }).collect::<Vec<_>>(),
            "sessions": analysis.sessions.iter().take(self.max_detailed_sessions).map(|session| {
                serde_json::json!({
                    "session_id": session.session_id,
                    "project_path": session.project_path,
                    "start_time": session.start_time.with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()).to_rfc3339(),
                    "end_time": session.end_time.with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()).to_rfc3339(),
                    "duration_minutes": (session.end_time - session.start_time).num_minutes(),
                    "total_messages": session.total_messages,
                    "user_messages": session.user_messages,
                    "assistant_messages": session.assistant_messages,
                    "summary": session.summary.as_ref().map(|s| serde_json::json!({
                        "overall_summary": s.overall_summary,
                        "main_topics": s.main_topics,
                        "technologies_mentioned": s.technologies_mentioned,
                        "problems_addressed": s.problems_addressed.len(),
                        "solutions_proposed": s.solutions_proposed.len()
                    }))
                })
            }).collect::<Vec<_>>(),
            "conversation_summary": analysis.conversation_summary.as_ref().map(|cs| serde_json::json!({
                "total_topics": cs.total_topics,
                "most_discussed_topics": cs.most_discussed_topics,
                "technology_usage": cs.technology_usage,
                "overall_themes": cs.overall_themes,
                "productivity_insights": cs.productivity_insights
            }))
        });

        Ok(serde_json::to_string_pretty(&json_data)?)
    }

    fn generate_header(&self, analysis: &WorkAnalysis) -> String {
        let (start, end) = analysis.time_range;
        // Convert to JST for display
        let jst = FixedOffset::east_opt(9 * 3600).unwrap();
        let start_jst = start.with_timezone(&jst);
        let end_jst = end.with_timezone(&jst);
        
        format!(
            "# 🤖 Claude Work Analysis Report\n\n**Analysis Period:** {} to {}",
            start_jst.format("%Y-%m-%d %H:%M JST"),
            end_jst.format("%Y-%m-%d %H:%M JST")
        )
    }

    fn generate_executive_summary(&self, analysis: &WorkAnalysis) -> String {
        let avg_session_length = if analysis.total_sessions > 0 {
            analysis.total_work_time.num_minutes() / analysis.total_sessions as i64
        } else {
            0
        };

        let avg_messages_per_session = if analysis.total_sessions > 0 {
            analysis.total_messages / analysis.total_sessions
        } else {
            0
        };

        format!(
            "- **Total Work Sessions:** {}\n\
             - **Total Messages:** {}\n\
             - **Total Work Time:** {:.1} hours\n\
             - **Average Session Length:** {} minutes\n\
             - **Average Messages per Session:** {}\n\
             - **Active Projects:** {}",
            analysis.total_sessions,
            analysis.total_messages,
            analysis.total_work_time.num_minutes() as f64 / 60.0,
            avg_session_length,
            avg_messages_per_session,
            analysis.project_stats.len()
        )
    }

    fn generate_project_breakdown(&self, analysis: &WorkAnalysis) -> String {
        let mut projects: Vec<_> = analysis.project_stats.iter().collect();
        projects.sort_by(|a, b| b.1.work_time.cmp(&a.1.work_time));

        let mut breakdown = String::new();
        
        for (project_name, stats) in projects {
            let work_hours = stats.work_time.num_minutes() as f64 / 60.0;
            let most_active_activity = stats.activity_types
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(activity, count)| format!("{} ({})", activity, count))
                .unwrap_or_else(|| "N/A".to_string());

            breakdown.push_str(&format!(
                "### 📁 {}\n\
                 - **Sessions:** {}\n\
                 - **Messages:** {}\n\
                 - **Work Time:** {:.1} hours\n\
                 - **Primary Activity:** {}\n\n",
                project_name,
                stats.total_sessions,
                stats.total_messages,
                work_hours,
                most_active_activity
            ));

            // Add topic analysis if available
            if let Some(ref topic_analysis) = stats.topic_analysis {
                breakdown.push_str(&format!(
                    " - **Primary Topics:** {}\n",
                    topic_analysis.primary_topics.join(", ")
                ));
                if !topic_analysis.technical_stack.is_empty() {
                    breakdown.push_str(&format!(
                        " - **Technical Stack:** {}\n",
                        topic_analysis.technical_stack.join(", ")
                    ));
                }
            }
            breakdown.push('\n');
        }

        breakdown
    }

    fn generate_activity_analysis(&self, analysis: &WorkAnalysis) -> String {
        let mut all_activities: HashMap<String, usize> = HashMap::new();
        
        for stats in analysis.project_stats.values() {
            for (activity, count) in &stats.activity_types {
                *all_activities.entry(activity.clone()).or_insert(0) += count;
            }
        }

        let mut activities: Vec<_> = all_activities.iter().collect();
        activities.sort_by(|a, b| b.1.cmp(a.1));

        let total_activities: usize = activities.iter().map(|(_, count)| *count).sum();

        let mut analysis_text = String::new();
        
        for (activity, count) in activities {
            let percentage = if total_activities > 0 {
                (*count as f64 / total_activities as f64) * 100.0
            } else {
                0.0
            };
            
            analysis_text.push_str(&format!(
                "- **{}:** {} times ({:.1}%)\n",
                activity, count, percentage
            ));
        }

        analysis_text
    }

    fn generate_time_analysis(&self, analysis: &WorkAnalysis) -> String {
        let mut daily_stats: HashMap<String, (usize, i64)> = HashMap::new(); // (sessions, minutes)
        let mut hourly_stats: HashMap<u32, usize> = HashMap::new(); // hour -> session_count

        for session in &analysis.sessions {
            let date_key = session.start_time.format("%Y-%m-%d").to_string();
            let hour = session.start_time.hour();
            let duration_minutes = (session.end_time - session.start_time).num_minutes();

            let (session_count, total_minutes) = daily_stats.entry(date_key).or_insert((0, 0));
            *session_count += 1;
            *total_minutes += duration_minutes;

            *hourly_stats.entry(hour).or_insert(0) += 1;
        }

        let mut time_analysis = String::new();

        // Most productive day
        if let Some((most_productive_day, (sessions, minutes))) = daily_stats
            .iter()
            .max_by_key(|(_, (sessions, _))| *sessions)
        {
            time_analysis.push_str(&format!(
                "**Most Productive Day:** {} ({} sessions, {:.1} hours)\n\n",
                most_productive_day,
                sessions,
                *minutes as f64 / 60.0
            ));
        }

        // Peak hours
        if let Some((peak_hour, session_count)) = hourly_stats
            .iter()
            .max_by_key(|(_, count)| *count)
        {
            time_analysis.push_str(&format!(
                "**Peak Activity Hour:** {}:00 ({} sessions)\n\n",
                peak_hour, session_count
            ));
        }

        // Daily breakdown (last 7 days)
        time_analysis.push_str("**Recent Daily Activity:**\n");
        let mut daily_entries: Vec<_> = daily_stats.iter().collect();
        daily_entries.sort_by(|a, b| b.0.cmp(a.0)); // Sort by date descending
        
        for (date, (sessions, minutes)) in daily_entries.iter().take(7) {
            time_analysis.push_str(&format!(
                "- {}: {} sessions ({:.1}h)\n",
                date,
                sessions,
                *minutes as f64 / 60.0
            ));
        }

        time_analysis
    }

    fn generate_session_details(&self, analysis: &WorkAnalysis) -> String {
        let mut details = String::new();
        
        // JST timezone for session display
        let jst = FixedOffset::east_opt(9 * 3600).unwrap();
        
        let mut recent_sessions = analysis.sessions.clone();
        recent_sessions.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        for session in recent_sessions.iter().take(self.max_detailed_sessions) {
            let duration = session.end_time - session.start_time;
            let project_name = session.project_path
                .split('/')
                .last()
                .unwrap_or("Unknown");

            let mut session_detail = format!(
                "### 🔄 Session: {} \n\
                 **Project:** {}\n\
                 **Duration:** {} minutes\n\
                 **Messages:** {} (User: {}, Assistant: {})\n\
                 **Time:** {}\n",
                &session.session_id.to_string()[..8],
                project_name,
                duration.num_minutes(),
                session.total_messages,
                session.user_messages,
                session.assistant_messages,
                session.start_time.with_timezone(&jst).format("%Y-%m-%d %H:%M JST")
            );

            // Add session summary if available
            if let Some(ref summary) = session.summary {
                session_detail.push_str(&format!(
                    "**Summary:** {}\n",
                    summary.overall_summary
                ));
                if !summary.main_topics.is_empty() {
                    session_detail.push_str(&format!(
                        "**Topics:** {}\n",
                        summary.main_topics.join(", ")
                    ));
                }
                if !summary.technologies_mentioned.is_empty() {
                    session_detail.push_str(&format!(
                        "**Technologies:** {}\n",
                        summary.technologies_mentioned.join(", ")
                    ));
                }
            }
            session_detail.push_str("\n");
            details.push_str(&session_detail);
        }

        details
    }

    fn generate_recommendations(&self, analysis: &WorkAnalysis) -> String {
        let mut recommendations = Vec::new();

        // Work pattern insights
        if analysis.total_sessions > 0 {
            let avg_session_length = analysis.total_work_time.num_minutes() / analysis.total_sessions as i64;
            
            if avg_session_length < 15 {
                recommendations.push("💡 **Short Sessions Detected:** Consider consolidating related tasks into longer, more focused work sessions for better productivity.");
            } else if avg_session_length > 120 {
                recommendations.push("⏱️ **Long Sessions Detected:** Consider taking breaks during extended coding sessions to maintain focus and code quality.");
            }
        }

        // Project diversity insights
        if analysis.project_stats.len() > 5 {
            recommendations.push("🎯 **High Project Diversity:** You're working on many projects. Consider prioritizing or batching similar tasks to reduce context switching overhead.");
        } else if analysis.project_stats.len() == 1 {
            recommendations.push("🔍 **Single Project Focus:** Great job maintaining focus on one project! Consider if this aligns with your current goals.");
        }

        // Activity pattern insights
        let mut all_activities: HashMap<String, usize> = HashMap::new();
        for stats in analysis.project_stats.values() {
            for (activity, count) in &stats.activity_types {
                *all_activities.entry(activity.clone()).or_insert(0) += count;
            }
        }

        if let Some((top_activity, _)) = all_activities.iter().max_by_key(|(_, count)| *count) {
            match top_activity.as_str() {
                "Debugging" => recommendations.push("🐛 **Debug-Heavy Period:** High debugging activity detected. Consider implementing more tests or code review practices."),
                "Learning" => recommendations.push("📚 **Learning Mode:** Lots of learning activity! Great for skill development. Document your learnings for future reference."),
                "Coding" => recommendations.push("⚡ **High Productivity:** Strong coding activity detected. Excellent work!"),
                _ => {}
            }
        }

        if recommendations.is_empty() {
            recommendations.push("✨ **Overall:** Your work patterns look healthy. Keep up the great work!");
        }

        recommendations.join("\n\n")
    }

    fn generate_conversation_summary_section(&self, analysis: &WorkAnalysis) -> String {
        if let Some(ref conv_summary) = analysis.conversation_summary {
            let mut summary = String::new();

            // Overall themes
            if !conv_summary.overall_themes.is_empty() {
                summary.push_str(&format!(
                    "**Overall Themes:** {}\n\n",
                    conv_summary.overall_themes.join(", ")
                ));
            }

            // Most discussed topics
            if !conv_summary.most_discussed_topics.is_empty() {
                summary.push_str("**Most Discussed Topics:**\n");
                for (topic, count) in conv_summary.most_discussed_topics.iter().take(5) {
                    summary.push_str(&format!("- {} ({} mentions)\n", topic, count));
                }
                summary.push('\n');
            }

            // Technology usage
            if !conv_summary.technology_usage.is_empty() {
                summary.push_str("**Technology Usage:**\n");
                let mut tech_usage: Vec<_> = conv_summary.technology_usage.iter().collect();
                tech_usage.sort_by(|a, b| b.1.cmp(a.1));
                for (tech, count) in tech_usage.iter().take(8) {
                    summary.push_str(&format!("- {} ({} times)\n", tech, count));
                }
                summary.push('\n');
            }

            // Common problems
            if !conv_summary.common_problems.is_empty() {
                summary.push_str("**Common Problem Areas:**\n");
                for problem in conv_summary.common_problems.iter().take(3) {
                    summary.push_str(&format!("- {}\n", problem));
                }
                summary.push('\n');
            }

            // Learning progression
            if !conv_summary.learning_progression.is_empty() {
                summary.push_str("**Learning Highlights:**\n");
                for learning in conv_summary.learning_progression.iter().take(3) {
                    summary.push_str(&format!("- {}\n", learning));
                }
                summary.push('\n');
            }

            // Productivity insights
            if !conv_summary.productivity_insights.is_empty() {
                summary.push_str("**Productivity Insights:**\n");
                for insight in &conv_summary.productivity_insights {
                    summary.push_str(&format!("- {}\n", insight));
                }
            }

            summary
        } else {
            "会話内容の分析は利用できません。".to_string()
        }
    }
}

impl Default for ReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{WorkSession, ProjectStats};
    use chrono::{Duration, Utc};
    use std::collections::HashMap;
    use uuid::Uuid;

    fn create_test_analysis() -> WorkAnalysis {
        let mut project_stats = HashMap::new();
        project_stats.insert(
            "test-project".to_string(),
            ProjectStats {
                project_name: "test-project".to_string(),
                total_sessions: 2,
                total_messages: 10,
                work_time: Duration::hours(2),
                activity_types: {
                    let mut activities = HashMap::new();
                    activities.insert("Coding".to_string(), 5);
                    activities.insert("Debugging".to_string(), 3);
                    activities
                },
                most_active_day: Some(Utc::now()),
                topic_analysis: None,
            }
        );

        WorkAnalysis {
            sessions: vec![
                WorkSession {
                    session_id: Uuid::new_v4(),
                    project_path: "/test/project".to_string(),
                    start_time: Utc::now() - Duration::hours(2),
                    end_time: Utc::now() - Duration::hours(1),
                    entries: Vec::new(),
                    total_messages: 5,
                    user_messages: 3,
                    assistant_messages: 2,
                    summary: None,
                }
            ],
            project_stats,
            time_range: (Utc::now() - Duration::days(1), Utc::now()),
            total_sessions: 2,
            total_messages: 10,
            total_work_time: Duration::hours(2),
            conversation_summary: None,
        }
    }

    #[test]
    fn test_markdown_report_generation() {
        let generator = ReportGenerator::new();
        let analysis = create_test_analysis();
        
        let report = generator.generate_markdown_report(&analysis).unwrap();
        
        assert!(report.contains("# 🤖 Claude Work Analysis Report"));
        assert!(report.contains("## 📊 Executive Summary"));
        assert!(report.contains("## 🚀 Project Breakdown"));
        assert!(report.contains("test-project"));
    }

    #[test]
    fn test_json_report_generation() {
        let generator = ReportGenerator::new();
        let analysis = create_test_analysis();
        
        let report = generator.generate_json_report(&analysis).unwrap();
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();
        
        assert_eq!(json["summary"]["total_sessions"], 2);
        assert_eq!(json["summary"]["total_messages"], 10);
        assert!(json["projects"].as_array().unwrap().len() > 0);
    }

    #[test]
    fn test_executive_summary() {
        let generator = ReportGenerator::new();
        let analysis = create_test_analysis();
        
        let summary = generator.generate_executive_summary(&analysis);
        
        assert!(summary.contains("**Total Work Sessions:** 2"));
        assert!(summary.contains("**Total Messages:** 10"));
        assert!(summary.contains("**Active Projects:** 1"));
    }
}