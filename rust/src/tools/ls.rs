use std::fs;
use std::path::Path;

use serde_json::Value;

use super::{Tool, ToolDefinition, ToolError, ToolExecutionResult};

/// Ls tool for listing directory contents.
pub struct LsTool;

impl Tool for LsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "ls".to_string(),
            description: "List directory contents.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The directory path to list"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    fn execute(&self, args: Value) -> ToolExecutionResult {
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

        if path.is_file() {
            return Ok(path.display().to_string());
        }

        let entries = fs::read_dir(path)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to read directory: {}", e)))?;

        let mut results = Vec::new();

        for entry in entries {
            let entry = entry
                .map_err(|e| ToolError::ExecutionError(format!("Failed to read entry: {}", e)))?;

            let path = entry.path();
            let file_type = if path.is_dir() {
                "dir"
            } else if path.is_file() {
                "file"
            } else {
                "other"
            };

            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            results.push(format!("{} [{}]", file_name, file_type));
        }

        results.sort();
        Ok(results.join("\n"))
    }
}
