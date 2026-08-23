use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an entry in the session tree.
pub type EntryId = Uuid;

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    /// Number of tokens in the input.
    pub input_tokens: u32,
    /// Number of tokens in the output.
    pub output_tokens: u32,
    /// Number of cached input tokens.
    pub cache_read_tokens: u32,
    /// Number of tokens written to cache.
    pub cache_write_tokens: u32,
}

impl Usage {
    /// Total tokens used.
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Role of a message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

/// A single message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender.
    pub role: Role,
    /// Content of the message.
    pub content: String,
    /// Tool calls in this message (for assistant messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool call ID this message is responding to (for tool messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// A tool call request from the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this tool call.
    pub id: String,
    /// Name of the tool to call.
    pub name: String,
    /// Arguments to pass to the tool (JSON string).
    pub arguments: String,
}

/// Result of executing a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the tool call this result is for.
    pub tool_call_id: String,
    /// Name of the tool that was called.
    pub name: String,
    /// Output from the tool execution.
    pub output: String,
    /// Whether the tool execution failed.
    pub is_error: bool,
}

/// An entry in the session tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// Type marker for JSONL.
    #[serde(rename = "type")]
    pub entry_type: String,
    /// Unique identifier for this entry.
    pub id: EntryId,
    /// Parent entry ID (None for root).
    pub parent_id: Option<EntryId>,
    /// Timestamp when this entry was created.
    pub created_at: DateTime<Utc>,
    /// The message content.
    pub message: Message,
    /// Token usage for this entry.
    #[serde(default)]
    pub usage: Usage,
    /// Whether this entry is on the active branch.
    pub is_active: bool,
}

impl Entry {
    /// Create a new user message entry.
    pub fn new_user(content: String, parent_id: Option<EntryId>) -> Self {
        Self {
            entry_type: "entry".to_string(),
            id: Uuid::new_v4(),
            parent_id,
            created_at: Utc::now(),
            message: Message {
                role: Role::User,
                content,
                tool_calls: None,
                tool_call_id: None,
            },
            usage: Usage::default(),
            is_active: true,
        }
    }

    /// Create a new assistant message entry.
    pub fn new_assistant(content: String, parent_id: Option<EntryId>) -> Self {
        Self {
            entry_type: "entry".to_string(),
            id: Uuid::new_v4(),
            parent_id,
            created_at: Utc::now(),
            message: Message {
                role: Role::Assistant,
                content,
                tool_calls: None,
                tool_call_id: None,
            },
            usage: Usage::default(),
            is_active: true,
        }
    }

    /// Create a new system message entry.
    pub fn new_system(content: String) -> Self {
        Self {
            entry_type: "entry".to_string(),
            id: Uuid::new_v4(),
            parent_id: None,
            created_at: Utc::now(),
            message: Message {
                role: Role::System,
                content,
                tool_calls: None,
                tool_call_id: None,
            },
            usage: Usage::default(),
            is_active: true,
        }
    }
}

/// Header information for a session file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    /// Session ID.
    pub session_id: Uuid,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// Model used for this session.
    pub model: String,
    /// System prompt used.
    pub system_prompt: Option<String>,
}

/// A compaction summary entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionEntry {
    /// ID of the compaction entry.
    pub id: EntryId,
    /// Parent entry ID.
    pub parent_id: EntryId,
    /// Timestamp.
    pub created_at: DateTime<Utc>,
    /// The summary content.
    pub summary: CompactionSummary,
    /// Token usage for the compaction.
    pub usage: Usage,
}

/// Structured summary from context compaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionSummary {
    /// Goal of the conversation so far.
    pub goal: String,
    /// Constraints and preferences.
    #[serde(default)]
    pub constraints: String,
    /// Progress made.
    pub progress: String,
    /// Blockers preventing progress.
    #[serde(default)]
    pub blockers: String,
    /// Key decisions taken.
    pub decisions: String,
    /// Next steps.
    pub next_steps: String,
    /// Critical context needed to continue.
    #[serde(default)]
    pub critical_context: String,
    /// Files that were read (not modified).
    #[serde(default)]
    pub read_files: Vec<String>,
    /// Files that were modified (written or edited).
    #[serde(default)]
    pub modified_files: Vec<String>,
    /// Legacy field: all files touched (read + modified).
    #[serde(default)]
    pub files_touched: Vec<String>,
}

/// Session header (first line of JSONL file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHeader {
    /// Type marker for JSONL.
    #[serde(rename = "type")]
    pub entry_type: String,
    /// Session metadata.
    pub header: Header,
}
