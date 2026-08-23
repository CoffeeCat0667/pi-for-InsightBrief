use std::process::Command;

use serde_json::Value;

use super::{Tool, ToolDefinition, ToolError, ToolExecutionResult};

/// Bash tool for executing shell commands.
pub struct BashTool;

impl Tool for BashTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "bash".to_string(),
            description: "Execute a bash command and return the output.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    fn execute(&self, args: Value) -> ToolExecutionResult {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' argument".to_string()))?;

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .map_err(|e| ToolError::ExecutionError(format!("Failed to execute command: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(stdout)
        } else {
            Ok(format!("Exit code: {}\nStdout: {}\nStderr: {}", 
                output.status.code().unwrap_or(-1), stdout, stderr))
        }
    }
}
