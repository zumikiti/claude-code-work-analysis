use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::models::ClaudeLogEntry;

pub struct JsonlParser {
    /// Whether to skip malformed lines or fail on them
    skip_malformed: bool,
    /// Maximum line length to prevent memory issues
    max_line_length: usize,
}

impl JsonlParser {
    pub fn new() -> Self {
        Self {
            skip_malformed: true,
            max_line_length: 10 * 1024 * 1024, // 10MB per line max (for large image content)
        }
    }

    pub fn with_strict_parsing() -> Self {
        Self {
            skip_malformed: false,
            max_line_length: 1024 * 1024,
        }
    }

    pub fn with_max_line_length(mut self, max_length: usize) -> Self {
        self.max_line_length = max_length;
        self
    }

    /// Parse a JSONL file and return all valid Claude log entries
    pub async fn parse_file(&self, file_path: &Path) -> Result<Vec<ClaudeLogEntry>> {
        let file = File::open(file_path)
            .await
            .with_context(|| format!("Failed to open file: {}", file_path.display()))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut entries = Vec::new();
        let mut line_number = 0;
        let mut skipped_lines = 0;
        let mut oversized_lines = 0;
        let mut summary_entries = 0;

        while let Some(line) = lines.next_line().await? {
            line_number += 1;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Check line length
            if line.len() > self.max_line_length {
                oversized_lines += 1;
                if self.skip_malformed {
                    // Only show warning for the first few oversized lines to avoid spam
                    if oversized_lines <= 3 {
                        eprintln!("Warning: Line {} exceeds maximum length of {} bytes in {}", 
                                 line_number, self.max_line_length, file_path.display());
                    }
                    continue;
                } else {
                    return Err(anyhow::anyhow!(
                        "Line {} exceeds maximum length of {} bytes",
                        line_number, self.max_line_length
                    ));
                }
            }

            match self.parse_line(&line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    let error_str = e.to_string();
                    if error_str.contains("Skipping summary entry") {
                        summary_entries += 1;
                        // Don't spam with summary entry warnings
                        continue;
                    }
                    
                    skipped_lines += 1;
                    if self.skip_malformed {
                        // Only show warning for the first few parse errors to avoid spam
                        if skipped_lines <= 3 {
                            eprintln!("Warning: Failed to parse line {} in {}: {}",
                                     line_number, file_path.display(), e);
                        }
                        continue;
                    } else {
                        return Err(anyhow::anyhow!(
                            "Failed to parse line {} in {}: {}",
                            line_number, file_path.display(), e
                        ));
                    }
                }
            }
        }

        // Show summary of parsing issues if any
        if skipped_lines > 0 || oversized_lines > 0 || summary_entries > 0 {
            let filename = file_path.file_name().unwrap_or_default().to_string_lossy();
            let mut issues = Vec::new();
            if summary_entries > 0 {
                issues.push(format!("{} summary entries", summary_entries));
            }
            if oversized_lines > 0 {
                issues.push(format!("{} oversized lines", oversized_lines));
            }
            if skipped_lines > 0 {
                issues.push(format!("{} parse errors", skipped_lines));
            }
            eprintln!("Info: {} - Skipped {} (out of {} total lines)", 
                     filename, issues.join(", "), line_number);
        }

        Ok(entries)
    }

    /// Parse a single line of JSONL into a ClaudeLogEntry
    pub fn parse_line(&self, line: &str) -> Result<ClaudeLogEntry> {
        // First check if this is a summary entry, which we should skip
        if let Ok(summary_check) = serde_json::from_str::<serde_json::Value>(line) {
            if summary_check.get("type").and_then(|t| t.as_str()) == Some("summary") {
                return Err(anyhow::anyhow!("Skipping summary entry"));
            }
        }
        
        let entry: ClaudeLogEntry = serde_json::from_str(line)
            .context("Failed to deserialize JSON line")?;
        
        Ok(entry)
    }

    /// Parse multiple JSONL files concurrently
    pub async fn parse_files(&self, file_paths: &[impl AsRef<Path>]) -> Result<Vec<ClaudeLogEntry>> {
        let mut all_entries = Vec::new();

        // Process files sequentially to avoid overwhelming the system
        for file_path in file_paths {
            let entries = self.parse_file(file_path.as_ref()).await?;
            all_entries.extend(entries);
        }

        // Sort by timestamp to maintain chronological order
        all_entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(all_entries)
    }

    /// Parse JSONL content from a string
    pub fn parse_string(&self, content: &str) -> Result<Vec<ClaudeLogEntry>> {
        let mut entries = Vec::new();
        
        for (line_number, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            match self.parse_line(line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    let error_msg = format!(
                        "Failed to parse line {}: {}",
                        line_number + 1,
                        e
                    );

                    if self.skip_malformed {
                        eprintln!("Warning: {}", error_msg);
                        continue;
                    } else {
                        return Err(anyhow::anyhow!(error_msg));
                    }
                }
            }
        }

        Ok(entries)
    }

    /// Validate that a file appears to be a valid JSONL file
    pub async fn validate_file(&self, file_path: &Path) -> Result<bool> {
        let file = File::open(file_path)
            .await
            .with_context(|| format!("Failed to open file: {}", file_path.display()))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut valid_lines = 0;
        let mut total_lines = 0;
        let max_check_lines = 10; // Only check first 10 lines for performance

        while let Some(line) = lines.next_line().await? {
            total_lines += 1;
            
            if line.trim().is_empty() {
                continue;
            }

            if self.parse_line(&line).is_ok() {
                valid_lines += 1;
            }

            if total_lines >= max_check_lines {
                break;
            }
        }

        // Consider valid if at least 50% of checked lines are valid JSON
        Ok(total_lines > 0 && (valid_lines as f64 / total_lines as f64) >= 0.5)
    }
}

impl Default for JsonlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_parse_valid_jsonl() {
        let content = r#"{"parentUuid":null,"sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2025-06-30T05:37:52.554Z","type":"user","message":{"role":"user","content":"test"},"uuid":"550e8400-e29b-41d4-a716-446655440001","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0.0"}"#;
        
        let parser = JsonlParser::new();
        let entries = parser.parse_string(content).unwrap();
        
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].cwd, "/test");
    }

    #[tokio::test]
    async fn test_parse_malformed_jsonl_skip() {
        let content = r#"{"valid": "json"}
invalid json line
{"another": "valid", "line": true}"#;
        
        let parser = JsonlParser::new(); // skip_malformed = true by default
        let entries = parser.parse_string(content).unwrap();
        
        // Should only parse the valid JSON lines
        assert_eq!(entries.len(), 0); // These aren't valid ClaudeLogEntry structures
    }

    #[tokio::test]
    async fn test_parse_empty_file() {
        let parser = JsonlParser::new();
        let entries = parser.parse_string("").unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_parse_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = r#"{"parentUuid":null,"sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2025-06-30T05:37:52.554Z","type":"user","message":{"role":"user","content":"test"},"uuid":"550e8400-e29b-41d4-a716-446655440001","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0.0"}"#;
        
        temp_file.write_all(content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let parser = JsonlParser::new();
        let entries = parser.parse_file(temp_file.path()).await.unwrap();
        
        assert_eq!(entries.len(), 1);
    }
}