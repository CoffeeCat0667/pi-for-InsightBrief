use std::fs;
use std::path::Path;

use serde_json::Value;

use super::{Tool, ToolDefinition, ToolError, ToolExecutionResult};

/// Grep tool for searching file contents.
pub struct GrepTool;

impl Tool for GrepTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "grep".to_string(),
            description: "Search for a pattern in file contents.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "The path to search in (file or directory)"
                    }
                },
                "required": ["pattern", "path"]
            }),
        }
    }

    fn execute(&self, args: Value) -> ToolExecutionResult {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'pattern' argument".to_string()))?;

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' argument".to_string()))?;

        let path = Path::new(path);
        if !path.exists() {
            return Err(ToolError::ExecutionError(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }

        let mut results = Vec::new();

        if path.is_file() {
            search_file(path, pattern, &mut results)?;
        } else if path.is_dir() {
            search_directory(path, pattern, &mut results)?;
        }

        if results.is_empty() {
            Ok("No matches found".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }
}

fn search_file(path: &Path, pattern: &str, results: &mut Vec<String>) -> Result<(), ToolError> {
    let content = fs::read_to_string(path)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to read file: {}", e)))?;

    for (i, line) in content.lines().enumerate() {
        if line.contains(pattern) {
            results.push(format!("{}:{}: {}", path.display(), i + 1, line));
        }
    }

    Ok(())
}

fn search_directory(
    path: &Path,
    pattern: &str,
    results: &mut Vec<String>,
) -> Result<(), ToolError> {
    let entries = fs::read_dir(path)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to read directory: {}", e)))?;

    for entry in entries {
        let entry = entry
            .map_err(|e| ToolError::ExecutionError(format!("Failed to read entry: {}", e)))?;

        let path = entry.path();

        if path.is_file() {
            search_file(&path, pattern, results)?;
        } else if path.is_dir() {
            search_directory(&path, pattern, results)?;
        }
    }

    Ok(())
}
