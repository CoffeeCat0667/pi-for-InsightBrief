use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use tokio::sync::broadcast;

use crate::agent::AgentEvent;
use super::types::{ChatCompletionRequest, ChatCompletionResponse, ChatCompletionChunk};

/// Error type for LLM operations.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("API error: {0}")]
    Api(String),

    #[error("Stream error: {0}")]
    Stream(String),
}

pub type LlmResult<T> = Result<T, LlmError>;

/// LLM client for OpenAI Chat Completions.
pub struct LlmClient {
    /// HTTP client.
    client: Client,
    /// API key.
    api_key: String,
    /// Base URL for the API.
    base_url: String,
    /// Event sink for structured diagnostics.
    debug_sender: Option<Arc<broadcast::Sender<AgentEvent>>>,
}

impl LlmClient {
    /// Create a new LLM client with 5-minute timeout for long contexts.
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        debug_sender: Option<Arc<broadcast::Sender<AgentEvent>>>,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            debug_sender,
        }
    }

    fn debug(&self, level: &str, message: String) {
        if let Some(sender) = &self.debug_sender {
            let _ = sender.send(AgentEvent::Debug {
                source: "llm".to_string(),
                level: level.to_string(),
                message,
            });
        }
    }

    /// Create a chat completion (non-streaming).
    pub async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
    ) -> LlmResult<ChatCompletionResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        self.debug("debug", format!("POST {} (model={}, messages={}, stream=false)", url, request.model, request.messages.len()));

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            self.debug("error", format!("HTTP {}: {}", status, body.chars().take(200).collect::<String>()));
            return Err(LlmError::Api(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let response: ChatCompletionResponse = response.json().await?;
        self.debug("debug", format!("Response received: {} choices", response.choices.len()));
        Ok(response)
    }

    /// Create a chat completion with streaming.
    pub async fn chat_completion_stream(
        &self,
        request: &ChatCompletionRequest,
    ) -> LlmResult<reqwest::Response> {
        let url = format!("{}/chat/completions", self.base_url);
        self.debug("debug", format!("POST {} (model={}, messages={}, stream=true)", url, request.model, request.messages.len()));

        let mut request = request.clone();
        request.stream = Some(true);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            self.debug("error", format!("HTTP {}: {}", status, body.chars().take(200).collect::<String>()));
            return Err(LlmError::Api(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        self.debug("debug", format!("Stream connected, status={}", response.status()));
        Ok(response)
    }

    /// Parse SSE stream into chunks.
    pub async fn parse_stream(
        response: reqwest::Response,
    ) -> LlmResult<Vec<ChatCompletionChunk>> {
        use futures::StreamExt;

        let mut chunks = Vec::new();
        let mut stream = response.bytes_stream();

        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete lines
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        return Ok(chunks);
                    }

                    if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(data) {
                        chunks.push(chunk);
                    }
                }
            }
        }

        Ok(chunks)
    }
}

impl std::fmt::Debug for LlmClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmClient")
            .field("base_url", &self.base_url)
            .finish()
    }
}
