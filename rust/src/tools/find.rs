use std::fs;
use std::path::Path;

use serde_json::Value;

use super::{Tool, ToolDefinition, ToolError, ToolExecutionResult};

/// Find tool for finding files by name pattern.
pub struct FindTool;

impl Tool for FindTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "find".to_string(),
            description: "Find files by name pattern.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The pattern to match file names"
                    },
                    "path": {
                        "type": "string",
                        "description": "The directory to search in"
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
        find_files(path, pattern, &mut results)?;

        if results.is_empty() {
            Ok("No files found".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }
}

fn find_files(path: &Path, pattern: &str, results: &mut Vec<String>) -> Result<(), ToolError> {
    let entries = fs::read_dir(path)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to read directory: {}", e)))?;

    for entry in entries {
        let entry = entry
            .map_err(|e| ToolError::ExecutionError(format!("Failed to read entry: {}", e)))?;

        let path = entry.path();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();

        if file_name.contains(pattern) {
            results.push(path.display().to_string());
        }

        if path.is_dir() {
            find_files(&path, pattern, results)?;
        }
    }

    Ok(())
}
