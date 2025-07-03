use anyhow::Result;
use chrono::{Utc, NaiveDate, TimeZone, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Write};
use tracing::{debug, error, info};

mod analyzer;
mod models;
mod parser;
mod filter;
mod scanner;
mod reporter;
mod message_analyzer;

use analyzer::WorkAnalyzer;
use models::WorkAnalysis;
use filter::TimeRangeFilter;
use parser::JsonlParser;
use reporter::ReportGenerator;
use scanner::ProjectScanner;

#[derive(Debug, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Debug, Serialize)]
struct McpError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct AnalyzePeriodParams {
    #[serde(default)]
    from_date: Option<String>,
    #[serde(default)]
    to_date: Option<String>,
    #[serde(default)]
    project_filter: Option<String>,
    #[serde(default)]
    format: Option<String>, // "markdown" or "json"
}

#[derive(Debug, Deserialize)]
struct ProjectStatsParams {
    project_name: String,
    #[serde(default)]
    days: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct SummarizeRecentParams {
    #[serde(default = "default_recent_days")]
    days: u32,
}

fn default_recent_days() -> u32 {
    7
}

pub struct ClaudeWorkAnalysisServer {
    analyzer: WorkAnalyzer,
    scanner: ProjectScanner,
    parser: JsonlParser,
    report_generator: ReportGenerator,
}

impl ClaudeWorkAnalysisServer {
    pub fn new() -> Self {
        Self {
            analyzer: WorkAnalyzer::new(),
            scanner: ProjectScanner::new(),
            parser: JsonlParser::new(),
            report_generator: ReportGenerator::new(),
        }
    }

    pub async fn run(&self) -> Result<()> {
        tracing_subscriber::fmt::init();
        info!("Claude Work Analysis MCP Server starting...");

        let input = io::stdin();
        let mut reader = BufReader::new(input.lock());
        let mut line = String::new();

        while reader.read_line(&mut line)? > 0 {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                line.clear();
                continue;
            }

            let response = match self.handle_request(trimmed).await {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Error handling request: {}", e);
                    McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: None,
                        error: Some(McpError {
                            code: -32603,
                            message: e.to_string(),
                            data: None,
                        }),
                    }
                }
            };

            let response_json = serde_json::to_string(&response)?;
            println!("{}", response_json);
            io::stdout().flush()?;

            line.clear();
        }

        Ok(())
    }

    async fn handle_request(&self, request_json: &str) -> Result<McpResponse> {
        debug!("Received request: {}", request_json);
        
        let request: McpRequest = serde_json::from_str(request_json)?;
        
        match request.method.as_str() {
            "initialize" => {
                Ok(McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "claude-work-analysis",
                            "version": "0.1.0"
                        }
                    })),
                    error: None,
                })
            }
            "tools/list" => {
                Ok(McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "tools": [
                            {
                                "name": "analyze_work_period",
                                "description": "Claude Code作業ログの期間分析を実行",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "from_date": {
                                            "type": "string",
                                            "description": "開始日(YYYY-MM-DD形式)"
                                        },
                                        "to_date": {
                                            "type": "string", 
                                            "description": "終了日(YYYY-MM-DD形式)"
                                        },
                                        "project_filter": {
                                            "type": "string",
                                            "description": "プロジェクト名でフィルタリング"
                                        },
                                        "format": {
                                            "type": "string",
                                            "enum": ["markdown", "json"],
                                            "description": "出力形式"
                                        }
                                    }
                                }
                            },
                            {
                                "name": "get_project_stats",
                                "description": "特定プロジェクトの統計情報を取得",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "project_name": {
                                            "type": "string",
                                            "description": "プロジェクト名"
                                        },
                                        "days": {
                                            "type": "number",
                                            "description": "過去何日分を分析するか"
                                        }
                                    },
                                    "required": ["project_name"]
                                }
                            },
                            {
                                "name": "summarize_recent",
                                "description": "直近の作業活動をサマリー",
                                "inputSchema": {
                                    "type": "object", 
                                    "properties": {
                                        "days": {
                                            "type": "number",
                                            "default": 7,
                                            "description": "過去何日分をサマリーするか"
                                        }
                                    }
                                }
                            }
                        ]
                    })),
                    error: None,
                })
            }
            "tools/call" => {
                let params = request.params.ok_or_else(|| anyhow::anyhow!("Missing params"))?;
                let tool_name = params["name"].as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;
                let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

                let result = match tool_name {
                    "analyze_work_period" => self.analyze_work_period(arguments).await?,
                    "get_project_stats" => self.get_project_stats(arguments).await?,
                    "summarize_recent" => self.summarize_recent(arguments).await?,
                    _ => return Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
                };

                Ok(McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "content": [
                            {
                                "type": "text",
                                "text": result
                            }
                        ]
                    })),
                    error: None,
                })
            }
            _ => {
                Err(anyhow::anyhow!("Unknown method: {}", request.method))
            }
        }
    }

    async fn analyze_work_period(&self, params: Value) -> Result<String> {
        let params: AnalyzePeriodParams = serde_json::from_value(params)?;
        
        // Parse date filters (JST timezone)
        let jst = FixedOffset::east_opt(9 * 3600).unwrap();
        let from_date = if let Some(from_str) = params.from_date {
            let date = NaiveDate::parse_from_str(&from_str, "%Y-%m-%d")?;
            Some(jst.from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap()).unwrap().with_timezone(&Utc))
        } else {
            None
        };
        
        let to_date = if let Some(to_str) = params.to_date {
            let date = NaiveDate::parse_from_str(&to_str, "%Y-%m-%d")?;
            Some(jst.from_local_datetime(&date.and_hms_opt(23, 59, 59).unwrap()).unwrap().with_timezone(&Utc))
        } else {
            None
        };
        
        let time_filter = TimeRangeFilter::new(from_date, to_date, params.project_filter.clone());

        // Get Claude projects directory
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let projects_dir = home_dir.join(".claude").join("projects");
        
        // Scan projects and parse entries
        let project_paths = self.scanner.scan_projects(&projects_dir)?;
        let mut all_entries = Vec::new();

        for path in project_paths {
            match self.parser.parse_file(&path).await {
                Ok(entries) => {
                    let filtered_entries = time_filter.filter_entries(entries);
                    if let Some(project_filter) = &params.project_filter {
                        let project_entries: Vec<_> = filtered_entries
                            .into_iter()
                            .filter(|entry| entry.cwd.contains(project_filter))
                            .collect();
                        all_entries.extend(project_entries);
                    } else {
                        all_entries.extend(filtered_entries);
                    }
                }
                Err(e) => {
                    debug!("Failed to parse {}: {}", path.display(), e);
                }
            }
        }

        // Analyze entries
        let analysis = self.analyzer.analyze_entries(&all_entries)?;
        
        // Generate report
        let format = params.format.as_deref().unwrap_or("markdown");
        let report = match format {
            "json" => {
                // For JSON output, create a simplified version
                let simple_analysis = serde_json::json!({
                    "total_sessions": analysis.total_sessions,
                    "total_messages": analysis.total_messages,
                    "total_work_time_hours": analysis.total_work_time.num_seconds() as f64 / 3600.0,
                    "project_count": analysis.project_stats.len(),
                    "time_range": {
                        "start": analysis.time_range.0.with_timezone(&jst),
                        "end": analysis.time_range.1.with_timezone(&jst)
                    }
                });
                serde_json::to_string_pretty(&simple_analysis)?
            },
            _ => self.report_generator.generate_markdown_report(&analysis)?,
        };

        Ok(report)
    }

    async fn get_project_stats(&self, params: Value) -> Result<String> {
        let params: ProjectStatsParams = serde_json::from_value(params)?;
        
        let time_filter = if let Some(days) = params.days {
            TimeRangeFilter::last_days(days as i64)
        } else {
            TimeRangeFilter::new(None, None, Some(params.project_name.clone()))
        };

        // Get Claude projects directory
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let projects_dir = home_dir.join(".claude").join("projects");
        
        // Scan and analyze
        let project_paths = self.scanner.scan_projects(&projects_dir)?;
        let mut all_entries = Vec::new();

        for path in project_paths {
            if let Ok(entries) = self.parser.parse_file(&path).await {
                let filtered_entries = time_filter.filter_entries(entries);
                let project_entries: Vec<_> = filtered_entries
                    .into_iter()
                    .filter(|entry| entry.cwd.contains(&params.project_name))
                    .collect();
                all_entries.extend(project_entries);
            }
        }

        let analysis = self.analyzer.analyze_entries(&all_entries)?;
        
        // Generate focused project report
        let project_sessions = self.analyzer.get_project_sessions(&analysis, &params.project_name);
        
        let mut report = format!("# {} プロジェクト統計\n\n", params.project_name);
        report.push_str(&format!("- セッション数: {}\n", project_sessions.len()));
        report.push_str(&format!("- 総メッセージ数: {}\n", 
            project_sessions.iter().map(|s| s.total_messages).sum::<usize>()));
        
        if let Some(project_stats) = analysis.project_stats.get(&params.project_name) {
            report.push_str(&format!("- 作業時間: {:.1}時間\n", 
                project_stats.work_time.num_seconds() as f64 / 3600.0));
            
            if let Some(ref topic_analysis) = project_stats.topic_analysis {
                report.push_str("\n## 主要トピック\n");
                for topic in &topic_analysis.primary_topics {
                    report.push_str(&format!("- {}\n", topic));
                }

                report.push_str("\n## 技術スタック\n");
                for tech in &topic_analysis.technical_stack {
                    report.push_str(&format!("- {}\n", tech));
                }
            }
        }

        Ok(report)
    }

    async fn summarize_recent(&self, params: Value) -> Result<String> {
        let params: SummarizeRecentParams = serde_json::from_value(params)?;
        
        let time_filter = TimeRangeFilter::last_days(params.days as i64);

        // Get Claude projects directory
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let projects_dir = home_dir.join(".claude").join("projects");
        
        // Scan and analyze recent activities
        let project_paths = self.scanner.scan_projects(&projects_dir)?;
        let mut all_entries = Vec::new();

        for path in project_paths {
            if let Ok(entries) = self.parser.parse_file(&path).await {
                let filtered_entries = time_filter.filter_entries(entries);
                all_entries.extend(filtered_entries);
            }
        }

        let analysis = self.analyzer.analyze_entries(&all_entries)?;
        
        // Generate compact summary
        let mut summary = format!("# 直近{}日間の活動サマリー\n\n", params.days);
        summary.push_str(&format!("- 総セッション数: {}\n", analysis.total_sessions));
        summary.push_str(&format!("- 総メッセージ数: {}\n", analysis.total_messages));
        summary.push_str(&format!("- 作業時間: {:.1}時間\n\n", 
            analysis.total_work_time.num_seconds() as f64 / 3600.0));

        summary.push_str("## アクティブプロジェクト\n");
        for (project_name, stats) in analysis.project_stats.iter().take(5) {
            summary.push_str(&format!("- **{}**: {}セッション, {:.1}時間\n", 
                project_name, stats.total_sessions,
                stats.work_time.num_seconds() as f64 / 3600.0));
        }

        if let Some(ref conv_summary) = analysis.conversation_summary {
            summary.push_str("\n## 主要トピック\n");
            for (topic, count) in conv_summary.most_discussed_topics.iter().take(5) {
                summary.push_str(&format!("- {} ({}回)\n", topic, count));
            }

            if !conv_summary.productivity_insights.is_empty() {
                summary.push_str("\n## 生産性インサイト\n");
                for insight in &conv_summary.productivity_insights {
                    summary.push_str(&format!("- {}\n", insight));
                }
            }
        }

        Ok(summary)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let server = ClaudeWorkAnalysisServer::new();
    server.run().await
}