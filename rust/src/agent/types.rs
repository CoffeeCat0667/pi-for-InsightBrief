use serde::{Deserialize, Serialize};

use super::system_prompt::PromptSet;
use crate::session::{EntryId, Message, ToolCall, ToolResult, Usage};

/// Events emitted by the agent loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AgentEvent {
    /// Agent loop started.
    AgentStart {
        /// Session ID.
        session_id: EntryId,
    },
    /// Agent loop ended.
    AgentEnd {
        /// Final message.
        message: Message,
        /// Total usage.
        usage: Usage,
    },
    /// A new turn started.
    TurnStart {
        /// Turn number.
        turn: u32,
    },
    /// A turn ended.
    TurnEnd {
        /// Turn number.
        turn: u32,
    },
    /// A message was received or sent.
    MessageEnd {
        /// The message.
        message: Message,
        /// Entry ID in the session.
        entry_id: EntryId,
    },
    /// A tool call was made.
    ToolCallStart {
        /// The tool call request.
        tool_call: ToolCall,
    },
    /// A tool call completed.
    ToolCallEnd {
        /// The tool result.
        result: ToolResult,
    },
    /// Compaction started.
    CompactionStart {
        /// Number of messages being compacted.
        message_count: u32,
    },
    /// Compaction completed.
    CompactionEnd {
        /// The summary generated.
        summary: String,
        /// Entry ID of the compaction.
        entry_id: EntryId,
    },
    /// Streaming token received.
    StreamToken {
        /// The token text.
        token: String,
    },
    /// Diagnostic information for FULL_DEBUG consumers.
    Debug {
        /// Component that emitted the diagnostic.
        source: String,
        /// Diagnostic level.
        level: String,
        /// Human-readable diagnostic message.
        message: String,
    },
}

/// Configuration for the agent loop.
#[derive(Debug, Clone)]
pub struct AgentLoopConfig {
    /// Model ID to use for LLM calls.
    pub model: String,
    /// Maximum number of turns per prompt.
    pub max_turns: u32,
    /// Maximum number of retries on HTTP errors for each LLM call.
    pub max_retries: u32,
    /// Reserve tokens for the response.
    pub reserve_tokens: u32,
    /// Keep recent tokens un-compacted.
    pub keep_recent_tokens: u32,
    /// Maximum tokens in the context window.
    pub context_window: u32,
    /// Prompt texts. Edit `rust/prompts/*.md` and recompile, or override at
    /// runtime via this field.
    pub prompt_set: PromptSet,
    /// Extra behavioral guidelines appended to the system prompt.
    pub extra_guidelines: Vec<String>,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            max_turns: 50,
            max_retries: 10,
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
            context_window: 128000,
            prompt_set: PromptSet::defaults(),
            extra_guidelines: Vec::new(),
        }
    }
}

/// Tool definition for the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// JSON schema for the tool's parameters.
    pub parameters: serde_json::Value,
}
