use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct ProjectScanner {
    /// Maximum depth to traverse in directory structure
    max_depth: usize,
}

impl ProjectScanner {
    pub fn new() -> Self {
        Self { max_depth: 3 }
    }

    pub fn with_max_depth(max_depth: usize) -> Self {
        Self { max_depth }
    }

    /// Scan the Claude projects directory and return all JSONL files
    pub fn scan_projects(&self, projects_dir: &Path) -> Result<Vec<PathBuf>> {
        if !projects_dir.exists() {
            return Err(anyhow::anyhow!(
                "Projects directory does not exist: {}",
                projects_dir.display()
            ));
        }

        let mut jsonl_files = Vec::new();

        for entry in WalkDir::new(projects_dir)
            .max_depth(self.max_depth)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if self.is_jsonl_file(path) {
                jsonl_files.push(path.to_path_buf());
            }
        }

        jsonl_files.sort_by(|a, b| {
            // Sort by modification time, newest first
            let a_metadata = a.metadata().unwrap_or_else(|_| {
                std::fs::metadata("/dev/null").unwrap()
            });
            let b_metadata = b.metadata().unwrap_or_else(|_| {
                std::fs::metadata("/dev/null").unwrap()
            });
            
            b_metadata
                .modified()
                .unwrap_or(std::time::UNIX_EPOCH)
                .cmp(&a_metadata.modified().unwrap_or(std::time::UNIX_EPOCH))
        });

        Ok(jsonl_files)
    }

    /// Scan a specific project directory and return JSONL files
    pub fn scan_project(&self, project_path: &Path) -> Result<Vec<PathBuf>> {
        if !project_path.exists() {
            return Err(anyhow::anyhow!(
                "Project directory does not exist: {}",
                project_path.display()
            ));
        }

        let mut jsonl_files = Vec::new();

        for entry in WalkDir::new(project_path)
            .max_depth(2) // Projects should be shallow
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if self.is_jsonl_file(path) {
                jsonl_files.push(path.to_path_buf());
            }
        }

        Ok(jsonl_files)
    }

    /// Extract project name from the encoded directory path
    pub fn extract_project_name(project_dir: &Path) -> Option<String> {
        project_dir
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| {
                // Claude encodes paths like: -Users-user-projects-project-name
                // We want to extract the meaningful part
                if name.starts_with('-') {
                    let parts: Vec<&str> = name.split('-').collect();
                    if parts.len() >= 3 {
                        // Take the last 2-3 segments as they're usually the meaningful project path
                        let meaningful_parts = &parts[parts.len().saturating_sub(3)..];
                        meaningful_parts.join("/")
                    } else {
                        name.to_string()
                    }
                } else {
                    name.to_string()
                }
            })
    }

    /// Get all project directories in the Claude projects directory
    pub fn get_project_directories(&self, projects_dir: &Path) -> Result<Vec<PathBuf>> {
        if !projects_dir.exists() {
            return Err(anyhow::anyhow!(
                "Projects directory does not exist: {}",
                projects_dir.display()
            ));
        }

        let mut project_dirs = Vec::new();

        for entry in std::fs::read_dir(projects_dir)
            .context("Failed to read projects directory")?
        {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            
            if path.is_dir() {
                // Skip hidden directories and current/parent directory references
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if !dir_name.starts_with('.') {
                        project_dirs.push(path);
                    }
                }
            }
        }

        // Sort project directories by name
        project_dirs.sort();

        Ok(project_dirs)
    }

    /// Check if a path represents a JSONL file
    fn is_jsonl_file(&self, path: &Path) -> bool {
        path.is_file() 
            && path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("jsonl"))
                .unwrap_or(false)
    }
}

impl Default for ProjectScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_extract_project_name() {
        let path = Path::new("-Users-user-projects-my-awesome-project");
        let result = ProjectScanner::extract_project_name(path);
        assert_eq!(result, Some("my/awesome/project".to_string()));
    }

    #[test]
    fn test_is_jsonl_file() {
        use std::fs::File;
        
        let temp_dir = TempDir::new().unwrap();
        let jsonl_path = temp_dir.path().join("test.jsonl");
        let json_path = temp_dir.path().join("test.json");
        
        // Create actual files
        File::create(&jsonl_path).unwrap();
        File::create(&json_path).unwrap();
        
        let scanner = ProjectScanner::new();
        
        assert!(scanner.is_jsonl_file(&jsonl_path));
        assert!(!scanner.is_jsonl_file(&json_path));
        assert!(!scanner.is_jsonl_file(Path::new("nonexistent.jsonl")));
    }

    #[test]
    fn test_scan_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let scanner = ProjectScanner::new();
        
        let result = scanner.scan_projects(temp_dir.path()).unwrap();
        assert!(result.is_empty());
    }
}