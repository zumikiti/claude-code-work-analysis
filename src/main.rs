use anyhow::Result;
use chrono::{DateTime, Utc, NaiveDate};
use clap::{Arg, Command};
use std::path::PathBuf;

mod models;
mod scanner;
mod parser;
mod filter;
mod analyzer;
mod reporter;
mod message_analyzer;

use crate::scanner::ProjectScanner;
use crate::filter::TimeRangeFilter;
use crate::parser::JsonlParser;
use crate::analyzer::WorkAnalyzer;
use crate::reporter::ReportGenerator;

/// Parse a date string in YYYY-MM-DD format to DateTime<Utc> (start of day)
fn parse_date_string(date_str: &str) -> Result<DateTime<Utc>> {
    let naive_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid date format '{}': {}. Expected YYYY-MM-DD", date_str, e))?;
    
    // Convert to DateTime<Utc> at start of day (00:00:00)
    Ok(naive_date.and_hms_opt(0, 0, 0).unwrap().and_utc())
}

/// Parse a date string in YYYY-MM-DD format to DateTime<Utc> (end of day)
fn parse_end_date_string(date_str: &str) -> Result<DateTime<Utc>> {
    let naive_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid date format '{}': {}. Expected YYYY-MM-DD", date_str, e))?;
    
    // Convert to DateTime<Utc> at end of day (23:59:59)
    Ok(naive_date.and_hms_opt(23, 59, 59).unwrap().and_utc())
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("claude-work-analysis")
        .version("0.1.0")
        .about("Analyze Claude Code work logs and generate summaries")
        .arg(
            Arg::new("from")
                .long("from")
                .value_name("DATE")
                .help("Start date (YYYY-MM-DD)")
                .required(false),
        )
        .arg(
            Arg::new("to")
                .long("to")
                .value_name("DATE")
                .help("End date (YYYY-MM-DD)")
                .required(false),
        )
        .arg(
            Arg::new("project")
                .long("project")
                .short('p')
                .value_name("PROJECT")
                .help("Filter by project name")
                .required(false),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .value_name("FILE")
                .help("Output file path")
                .required(false),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .value_name("FORMAT")
                .help("Output format (markdown, json)")
                .default_value("markdown"),
        )
        .get_matches();

    // Parse command line arguments
    let from_date = matches
        .get_one::<String>("from")
        .map(|s| parse_date_string(s).expect("Invalid from date format"));
    
    let to_date = matches
        .get_one::<String>("to")
        .map(|s| parse_end_date_string(s).expect("Invalid to date format"));
    
    let project_filter = matches.get_one::<String>("project").cloned();
    let output_path = matches.get_one::<String>("output").map(PathBuf::from);
    let format = matches.get_one::<String>("format").unwrap();

    // Create filter
    let filter = TimeRangeFilter::new(from_date, to_date, project_filter);

    // Scan Claude projects directory
    let scanner = ProjectScanner::new();
    let projects_dir = dirs::home_dir()
        .expect("Cannot find home directory")
        .join(".claude")
        .join("projects");
    
    let jsonl_files = scanner.scan_projects(&projects_dir)?;
    
    // Parse and filter entries
    let parser = JsonlParser::new();
    let mut all_entries = Vec::new();
    
    for file_path in jsonl_files {
        let entries = parser.parse_file(&file_path).await?;
        let filtered_entries = filter.filter_entries(entries);
        all_entries.extend(filtered_entries);
    }

    // Analyze work patterns
    let analyzer = WorkAnalyzer::new();
    let analysis = analyzer.analyze_entries(&all_entries)?;

    // Generate report
    let reporter = ReportGenerator::new();
    let report = match format.as_str() {
        "json" => reporter.generate_json_report(&analysis)?,
        _ => reporter.generate_markdown_report(&analysis)?,
    };

    // Output report
    if let Some(output_path) = output_path {
        std::fs::write(output_path, report)?;
    } else {
        println!("{}", report);
    }

    Ok(())
}