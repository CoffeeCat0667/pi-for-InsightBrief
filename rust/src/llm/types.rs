use serde::{Deserialize, Serialize};

/// OpenAI Chat Completions request.
#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionRequest {
    /// Model to use.
    pub model: String,
    /// Messages in the conversation.
    pub messages: Vec<ChatMessage>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Temperature (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Tools available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatTool>>,
}

/// A message in OpenAI format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role of the message.
    pub role: String,
    /// Content of the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool calls (for assistant messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatToolCall>>,
    /// Tool call ID (for tool messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Name (for tool messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// A tool call in OpenAI format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatToolCall {
    /// Tool call ID.
    pub id: String,
    /// Tool type (always "function").
    #[serde(rename = "type")]
    pub call_type: String,
    /// Function call details.
    pub function: ChatFunctionCall,
}

/// Function call details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatFunctionCall {
    /// Function name.
    pub name: String,
    /// Arguments as JSON string.
    pub arguments: String,
}

/// Tool definition for OpenAI.
#[derive(Debug, Clone, Serialize)]
pub struct ChatTool {
    /// Tool type (always "function").
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function details.
    pub function: ChatFunction,
}

/// Function definition for OpenAI.
#[derive(Debug, Clone, Serialize)]
pub struct ChatFunction {
    /// Function name.
    pub name: String,
    /// Function description.
    pub description: String,
    /// JSON schema for parameters.
    pub parameters: serde_json::Value,
}

/// OpenAI Chat Completions response.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionResponse {
    /// Response ID.
    pub id: String,
    /// Object type.
    pub object: String,
    /// Created timestamp.
    pub created: u64,
    /// Model used.
    pub model: String,
    /// Choices.
    pub choices: Vec<ChatChoice>,
    /// Usage statistics.
    pub usage: Option<ChatUsage>,
}

/// A single choice in the response.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatChoice {
    /// Choice index.
    pub index: u32,
    /// The message.
    pub message: ChatMessage,
    /// Stop reason.
    pub finish_reason: Option<String>,
}

/// Usage statistics from OpenAI.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatUsage {
    /// Prompt tokens.
    pub prompt_tokens: u32,
    /// Completion tokens.
    pub completion_tokens: u32,
    /// Total tokens.
    pub total_tokens: u32,
}

/// Streaming chunk from OpenAI.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionChunk {
    /// Chunk ID.
    pub id: String,
    /// Object type.
    pub object: String,
    /// Created timestamp.
    pub created: u64,
    /// Model used.
    pub model: String,
    /// Choices.
    pub choices: Vec<ChunkChoice>,
    /// Usage (only in final chunk).
    pub usage: Option<ChatUsage>,
}

/// A choice in a streaming chunk.
#[derive(Debug, Clone, Deserialize)]
pub struct ChunkChoice {
    /// Choice index.
    pub index: u32,
    /// Delta content.
    pub delta: ChatDelta,
    /// Stop reason.
    pub finish_reason: Option<String>,
}

/// Delta content in streaming.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatDelta {
    /// Role (only in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Content token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool calls delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChunkToolCall>>,
}

/// Tool call delta in streaming.
#[derive(Debug, Clone, Deserialize)]
pub struct ChunkToolCall {
    /// Tool call index.
    pub index: u32,
    /// Tool call ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Function delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<ChunkFunctionDelta>,
}

/// Function delta in streaming.
#[derive(Debug, Clone, Deserialize)]
pub struct ChunkFunctionDelta {
    /// Function name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Arguments delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}
