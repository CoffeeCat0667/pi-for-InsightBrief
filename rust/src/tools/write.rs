use std::fs;

use serde_json::Value;

use super::{Tool, ToolDefinition, ToolError, ToolExecutionResult};

/// Write tool for writing content to files.
pub struct WriteTool;

impl Tool for WriteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "write".to_string(),
            description: "Write content to a file.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    fn execute(&self, args: Value) -> ToolExecutionResult {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' argument".to_string()))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'content' argument".to_string()))?;

        fs::write(path, content)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to write file: {}", e)))?;

        Ok(format!("Successfully wrote to {}", path))
    }
}
