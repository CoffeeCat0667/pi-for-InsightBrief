use std::sync::Arc;

use tokio::sync::broadcast;

use super::system_prompt::build_system_prompt;
use super::types::{AgentEvent, AgentLoopConfig, ToolDefinition};
use crate::llm::LlmClient;
use crate::session::{CompactionAnalysis, Entry, SessionStore};
use crate::tools::{Tool, ToolRegistry};

/// Error type for agent operations.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("LLM error: {0}")]
    Llm(#[from] crate::llm::LlmError),

    #[error("Session error: {0}")]
    Session(#[from] crate::session::SessionError),

    #[error("Tool error: {0}")]
    Tool(String),

    #[error("Max turns exceeded")]
    MaxTurnsExceeded,

    #[error("Stream closed")]
    StreamClosed,
}

pub type AgentResult<T> = Result<T, AgentError>;

/// Agent loop that handles conversation with the LLM.
pub struct AgentLoop {
    /// Configuration.
    config: AgentLoopConfig,
    /// LLM client.
    llm_client: Arc<LlmClient>,
    /// Session store.
    session: Arc<tokio::sync::Mutex<SessionStore>>,
    /// Event sender.
    event_sender: broadcast::Sender<AgentEvent>,
    /// Tool registry.
    tool_registry: ToolRegistry,
    /// Working directory for the system prompt.
    cwd: String,
}

impl AgentLoop {
    /// Create a new agent loop.
    pub fn new(
        config: AgentLoopConfig,
        llm_client: Arc<LlmClient>,
        session: Arc<tokio::sync::Mutex<SessionStore>>,
        event_sender: broadcast::Sender<AgentEvent>,
    ) -> Self {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());

        Self {
            config,
            llm_client,
            session,
            event_sender,
            tool_registry: ToolRegistry::new(),
            cwd,
        }
    }

    /// Set the working directory used in the system prompt.
    pub fn set_cwd(&mut self, cwd: &str) {
        self.cwd = cwd.to_string();
    }

    /// Register a tool.
    pub fn register_tool(&mut self, tool: Box<dyn Tool>) {
        self.tool_registry.register(tool);
    }

    /// Export the current session tree for the Python in-memory API.
    pub async fn export_session_data(&self) -> serde_json::Value {
        self.session.lock().await.to_data()
    }

    /// Build the system prompt from the current config and tool registry.
    fn build_system_prompt_text(&self, context_files: &[(String, String)]) -> String {
        let tool_names: Vec<String> = self
            .tool_registry
            .definitions()
            .iter()
            .map(|t| t.name.clone())
            .collect();

        build_system_prompt(
            &self.config.prompt_set,
            &tool_names,
            context_files,
            &self.config.extra_guidelines,
            &self.cwd,
        )
    }

    /// Run the agent loop with a user prompt.
    pub async fn run(&self, prompt: &str) -> AgentResult<()> {
        // Append user message to session
        let user_entry = {
            let mut session = self.session.lock().await;
            let entry = Entry::new_user(prompt.to_string(), None);
            session.append(entry)?
        };

        // Emit agent start event
        let _ = self.event_sender.send(AgentEvent::AgentStart {
            session_id: user_entry,
        });

        // Build system prompt (no context files for now; can be extended)
        let system_prompt_text = self.build_system_prompt_text(&[]);

        // Run the inner loop (tool calls)
        let mut turn = 0;
        loop {
            turn += 1;
            if turn > self.config.max_turns {
                return Err(AgentError::MaxTurnsExceeded);
            }

            // Emit turn start
            let _ = self.event_sender.send(AgentEvent::TurnStart { turn });

            // Check if compaction is needed
            self.check_compaction().await?;

            // Get messages from session
            let messages: Vec<crate::session::Message> = {
                let session = self.session.lock().await;
                session.messages().iter().map(|m| (*m).clone()).collect()
            };

            // Convert to LLM format and prepend system prompt
            let mut llm_messages = vec![crate::llm::ChatMessage {
                role: "system".to_string(),
                content: Some(system_prompt_text.clone()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }];
            llm_messages.extend(self.convert_messages(&messages));

            // Create tool definitions
            let tool_defs: Vec<ToolDefinition> = self.tool_registry.definitions();

            // Call LLM with streaming
            let response = self
                .llm_client
                .chat_completion_stream(&crate::llm::ChatCompletionRequest {
                    model: self.config.model.clone(),
                    messages: llm_messages,
                    stream: None,
                    max_tokens: Some(self.config.reserve_tokens),
                    temperature: None,
                    tools: if tool_defs.is_empty() {
                        None
                    } else {
                        Some(
                            tool_defs
                                .iter()
                                .map(|t| crate::llm::ChatTool {
                                    tool_type: "function".to_string(),
                                    function: crate::llm::ChatFunction {
                                        name: t.name.clone(),
                                        description: t.description.clone(),
                                        parameters: t.parameters.clone(),
                                    },
                                })
                                .collect(),
                        )
                    },
                })
                .await?;

            // Process stream in real-time - true streaming
            use futures::StreamExt;
            let mut stream = response.bytes_stream();
            let mut full_content = String::new();
            let mut tool_calls: Option<Vec<crate::llm::ChatToolCall>> = None;
            let mut usage = crate::session::Usage::default();
            let mut buffer = String::new();
            let mut stream_done = false;
            let mut chunk_count = 0u32;

            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result.map_err(|e| AgentError::Llm(crate::llm::LlmError::Http(e)))?;
                chunk_count += 1;
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
                            stream_done = true;
                            break;
                        }

                        if let Ok(chunk) = serde_json::from_str::<crate::llm::ChatCompletionChunk>(data) {
                            // Emit stream tokens immediately as they arrive
                            if let Some(choice) = chunk.choices.first() {
                                if let Some(content) = &choice.delta.content {
                                    if !content.is_empty() {
                                        full_content.push_str(content);
                                        let _ = self.event_sender.send(AgentEvent::StreamToken {
                                            token: content.clone(),
                                        });
                                    }
                                }
                            }

                            // Collect tool calls from chunks
                            if let Some(choice) = chunk.choices.first() {
                                if let Some(calls) = &choice.delta.tool_calls {
                                    if tool_calls.is_none() {
                                        tool_calls = Some(Vec::new());
                                    }
                                    for tc in calls {
                                        if let Some(ref mut existing) = tool_calls {
                                            let idx = tc.index as usize;
                                            while existing.len() <= idx {
                                                existing.push(crate::llm::ChatToolCall {
                                                    id: String::new(),
                                                    function: crate::llm::ChatFunctionCall {
                                                        name: String::new(),
                                                        arguments: String::new(),
                                                    },
                                                    call_type: "function".to_string(),
                                                });
                                            }
                                            if let Some(id) = &tc.id {
                                                existing[idx].id = id.clone();
                                            }
                                            if let Some(func) = &tc.function {
                                                if let Some(name) = &func.name {
                                                    existing[idx].function.name.push_str(name);
                                                }
                                                if let Some(args) = &func.arguments {
                                                    existing[idx].function.arguments.push_str(args);
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Collect usage
                            if let Some(u) = &chunk.usage {
                                usage.input_tokens = u.prompt_tokens;
                                usage.output_tokens = u.completion_tokens;
                            }
                        }
                    }
                }
                
                if stream_done {
                    break;
                }
            }

            eprintln!("[Stream] Done: {} chunks, {} chars, tool_calls={}", chunk_count, full_content.len(), tool_calls.is_some());
            if let Some(ref tc) = tool_calls {
                eprintln!("[Stream] Tool calls: {:?}", tc.iter().map(|t| (&t.function.name, t.function.arguments.len())).collect::<Vec<_>>());
            }

            let parsed_tool_calls = tool_calls;

            // Append assistant message to session (with tool calls if present)
            let assistant_entry = {
                let mut session = self.session.lock().await;
                let mut entry = Entry::new_assistant(
                    full_content.clone(),
                    None,
                );
                entry.usage = usage;
                // Store tool calls in the assistant message
                entry.message.tool_calls = parsed_tool_calls.as_ref().map(|calls| {
                    calls.iter().map(|tc| crate::session::ToolCall {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                        arguments: tc.function.arguments.clone(),
                    }).collect()
                });
                session.append(entry)?
            };

            eprintln!("[Stream] Message saved, entry_id={}", assistant_entry);

            // Emit message end
            let _ = self.event_sender.send(AgentEvent::MessageEnd {
                message: crate::session::Message {
                    role: crate::session::Role::Assistant,
                    content: full_content,
                    tool_calls: parsed_tool_calls.as_ref().map(|calls| {
                        calls.iter().map(|tc| crate::session::ToolCall {
                            id: tc.id.clone(),
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        }).collect()
                    }),
                    tool_call_id: None,
                },
                entry_id: assistant_entry,
            });

            // Check if there are tool calls
            let tool_calls = match parsed_tool_calls {
                Some(calls) if !calls.is_empty() => calls,
                _ => {
                    // No tool calls, we're done
                    let _ = self.event_sender.send(AgentEvent::TurnEnd { turn });
                    break;
                }
            };

            // Execute tool calls
            for tool_call in &tool_calls {
                // Emit tool call start
                let _ = self.event_sender.send(AgentEvent::ToolCallStart {
                    tool_call: crate::session::ToolCall {
                        id: tool_call.id.clone(),
                        name: tool_call.function.name.clone(),
                        arguments: tool_call.function.arguments.clone(),
                    },
                });

                // Execute the tool
                let result = match serde_json::from_str(&tool_call.function.arguments) {
                    Ok(args) => match self.tool_registry.execute(&tool_call.function.name, args) {
                        Ok(output) => crate::session::ToolResult {
                            tool_call_id: tool_call.id.clone(),
                            name: tool_call.function.name.clone(),
                            output,
                            is_error: false,
                        },
                        Err(error) => crate::session::ToolResult {
                            tool_call_id: tool_call.id.clone(),
                            name: tool_call.function.name.clone(),
                            output: error.to_string(),
                            is_error: true,
                        },
                    },
                    Err(e) => crate::session::ToolResult {
                        tool_call_id: tool_call.id.clone(),
                        name: tool_call.function.name.clone(),
                        output: format!("Invalid arguments: {}", e),
                        is_error: true,
                    },
                };

                // Emit tool call end
                let _ = self.event_sender.send(AgentEvent::ToolCallEnd {
                    result: result.clone(),
                });

                // Append tool result to session
                let _ = {
                    let mut session = self.session.lock().await;
                    let entry = Entry {
                        entry_type: "entry".to_string(),
                        id: uuid::Uuid::new_v4(),
                        parent_id: None, // Will be set by append
                        created_at: chrono::Utc::now(),
                        message: crate::session::Message {
                            role: crate::session::Role::Tool,
                            content: result.output,
                            tool_calls: None,
                            tool_call_id: Some(result.tool_call_id),
                        },
                        usage: crate::session::Usage::default(),
                        is_active: true,
                    };
                    session.append(entry)?
                };
            }

            // Emit turn end
            let _ = self.event_sender.send(AgentEvent::TurnEnd { turn });
        }

        // Get final message and usage
        let (final_message, total_usage) = {
            let session = self.session.lock().await;
            let last_message = session.leaf().map(|e| e.message.clone());
            let usage = session.total_usage();
            (last_message, usage)
        };

        // Emit agent end
        if let Some(message) = final_message {
            let _ = self.event_sender.send(AgentEvent::AgentEnd {
                message,
                usage: total_usage,
            });
        }

        Ok(())
    }

    /// Check if compaction is needed and perform it.
    async fn check_compaction(&self) -> AgentResult<()> {
        let (analysis, previous_summary, branch_entries) = {
            let session = self.session.lock().await;
            let entries: Vec<&Entry> = session.branch();

            let config = crate::session::CompactionConfig {
                context_window: self.config.context_window,
                reserve_tokens: self.config.reserve_tokens,
                keep_recent_tokens: self.config.keep_recent_tokens,
            };

            let analysis = crate::session::analyze_compaction(&entries, &config);
            let previous_summary = session.last_compaction().cloned();

            // Collect branch entries as owned for passing to generate_summary
            let branch_entries: Vec<Entry> = entries.into_iter().cloned().collect();

            (analysis, previous_summary, branch_entries)
        };

        if !analysis.needs_compaction {
            return Ok(());
        }

        // Emit compaction start
        let _ = self.event_sender.send(AgentEvent::CompactionStart {
            message_count: analysis.compacted_entries.len() as u32,
        });

        // Generate summary using LLM
        let summary = self.generate_summary(&analysis, &branch_entries, previous_summary.as_ref()).await?;

        // Save compaction entry
        {
            let mut session = self.session.lock().await;
            let cut_point = analysis.cut_point.ok_or_else(|| {
                AgentError::Tool("No cut point found".to_string())
            })?;

            let compaction = crate::session::CompactionEntry {
                id: uuid::Uuid::new_v4(),
                parent_id: cut_point,
                created_at: chrono::Utc::now(),
                summary: summary.clone(),
                usage: crate::session::Usage::default(),
            };

            session.save_compaction(compaction)?;
        }

        // Emit compaction end
        let _ = self.event_sender.send(AgentEvent::CompactionEnd {
            summary: format!(
                "Goal: {}\nProgress: {}\nDecisions: {}\nNext Steps: {}",
                summary.goal, summary.progress, summary.decisions, summary.next_steps
            ),
            entry_id: uuid::Uuid::new_v4(), // Placeholder
        });

        Ok(())
    }

    /// Generate a summary for compaction using the LLM.
    async fn generate_summary(
        &self,
        analysis: &CompactionAnalysis,
        branch_entries: &[Entry],
        previous_summary: Option<&crate::session::CompactionSummary>,
    ) -> AgentResult<crate::session::CompactionSummary> {
        // Build a set of compacted entry IDs for quick lookup
        let compacted_ids: std::collections::HashSet<_> =
            analysis.compacted_entries.iter().collect();

        // Filter to only the entries that will be compacted (turned into summary)
        let entries_to_summarize: Vec<&Entry> = branch_entries
            .iter()
            .filter(|e| compacted_ids.contains(&e.id))
            .collect();

        // Serialize the compacted messages into a conversation text
        let conversation_text = serialize_conversation_for_compaction(&entries_to_summarize);

        // Choose the right user prompt
        let user_prompt = if previous_summary.is_some() {
            &self.config.prompt_set.compaction_update
        } else {
            &self.config.prompt_set.compaction_initial
        };

        // Build the full user message
        let mut user_message = format!(
            "<conversation>\n{}\n</conversation>\n\n",
            conversation_text
        );

        if let Some(prev) = previous_summary {
            let prev_text = format!(
                "## Goal\n{}\n\n## Constraints & Preferences\n{}\n\n## Progress\n{}\n\n## Blockers\n{}\n\n## Decisions\n{}\n\n## Next Steps\n{}\n\n## Critical Context\n{}\n\n## Files\nRead: {}\nModified: {}",
                prev.goal, prev.constraints, prev.progress, prev.blockers,
                prev.decisions, prev.next_steps, prev.critical_context,
                prev.read_files.join(", "),
                prev.modified_files.join(", "),
            );
            user_message.push_str(&format!(
                "<previous-summary>\n{}\n</previous-summary>\n\n",
                prev_text
            ));
        }

        user_message.push_str(user_prompt);

        // Call LLM to generate summary
        let response = self
            .llm_client
            .chat_completion(&crate::llm::ChatCompletionRequest {
                model: self.config.model.clone(),
                messages: vec![
                    crate::llm::ChatMessage {
                        role: "system".to_string(),
                        content: Some(self.config.prompt_set.compaction_system.clone()),
                        tool_calls: None,
                        tool_call_id: None,
                        name: None,
                    },
                    crate::llm::ChatMessage {
                        role: "user".to_string(),
                        content: Some(user_message),
                        tool_calls: None,
                        tool_call_id: None,
                        name: None,
                    },
                ],
                stream: None,
                max_tokens: Some(1024),
                temperature: Some(0.3),
                tools: None,
            })
            .await?;

        // Parse the response
        let content = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(parse_compaction_summary(&content))
    }

    /// Convert session messages to LLM format.
    fn convert_messages(
        &self,
        messages: &[crate::session::Message],
    ) -> Vec<crate::llm::ChatMessage> {
        messages
            .iter()
            .map(|m| crate::llm::ChatMessage {
                role: match m.role {
                    crate::session::Role::User => "user".to_string(),
                    crate::session::Role::Assistant => "assistant".to_string(),
                    crate::session::Role::System => "system".to_string(),
                    crate::session::Role::Tool => "tool".to_string(),
                },
                content: if m.content.is_empty() && m.tool_calls.is_some() {
                    None  // Assistant messages with tool calls may have empty content
                } else {
                    Some(m.content.clone())
                },
                tool_calls: m.tool_calls.as_ref().map(|calls| {
                    calls.iter().map(|tc| crate::llm::ChatToolCall {
                        id: tc.id.clone(),
                        call_type: "function".to_string(),
                        function: crate::llm::ChatFunctionCall {
                            name: tc.name.clone(),
                            arguments: tc.arguments.clone(),
                        },
                    }).collect()
                }),
                tool_call_id: m.tool_call_id.clone(),
                name: None,
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Compaction helpers
// ---------------------------------------------------------------------------

/// Maximum characters for a tool result in the serialized conversation.
const TOOL_RESULT_MAX_CHARS: usize = 2000;

/// Serialize entries into a conversation text for the compaction prompt.
fn serialize_conversation_for_compaction(entries: &[&Entry]) -> String {
    let mut parts = Vec::new();

    for entry in entries {
        let msg = &entry.message;
        match msg.role {
            crate::session::Role::User => {
                parts.push(format!("[User]: {}", msg.content));
            }
            crate::session::Role::Assistant => {
                if !msg.content.is_empty() {
                    parts.push(format!("[Assistant]: {}", msg.content));
                }
                if let Some(ref calls) = msg.tool_calls {
                    let call_strs: Vec<String> = calls
                        .iter()
                        .map(|tc| format!("{}({})", tc.name, truncate_str(&tc.arguments, 200)))
                        .collect();
                    parts.push(format!("[Assistant tool calls]: {}", call_strs.join("; ")));
                }
            }
            crate::session::Role::Tool => {
                let content = truncate_str(&msg.content, TOOL_RESULT_MAX_CHARS);
                parts.push(format!("[Tool result]: {}", content));
            }
            crate::session::Role::System => {
                parts.push(format!("[System]: {}", msg.content));
            }
        }
    }

    parts.join("\n\n")
}

/// Truncate a string to max_chars, adding an ellipsis note if truncated.
/// Uses character boundaries to avoid panicking on multi-byte UTF-8.
fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        let remaining = s.chars().count() - max_chars;
        format!("{}\n\n[... {} more characters truncated]", truncated, remaining)
    }
}

/// Parse a markdown-formatted compaction summary into `CompactionSummary`.
///
/// Expected format:
/// ```text
/// ## Goal
/// ...
/// ## Constraints & Preferences
/// ...
/// ## Progress
/// ...
/// ## Blockers
/// ...
/// ## Decisions
/// ...
/// ## Next Steps
/// ...
/// ## Critical Context
/// ...
/// ## Files
/// Read: ...
/// Modified: ...
/// ```
fn parse_compaction_summary(content: &str) -> crate::session::CompactionSummary {
    let mut goal = String::new();
    let mut constraints = String::new();
    let mut progress = String::new();
    let mut blockers = String::new();
    let mut decisions = String::new();
    let mut next_steps = String::new();
    let mut critical_context = String::new();
    let mut read_files = Vec::new();
    let mut modified_files = Vec::new();

    let mut current_section = "";
    let mut section_body = String::new();

    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            // Flush previous section
            flush_section(
                current_section,
                &section_body,
                &mut goal,
                &mut constraints,
                &mut progress,
                &mut blockers,
                &mut decisions,
                &mut next_steps,
                &mut critical_context,
                &mut read_files,
                &mut modified_files,
            );
            current_section = heading.trim();
            section_body.clear();
        } else {
            section_body.push_str(line);
            section_body.push('\n');
        }
    }
    // Flush last section
    flush_section(
        current_section,
        &section_body,
        &mut goal,
        &mut constraints,
        &mut progress,
        &mut blockers,
        &mut decisions,
        &mut next_steps,
        &mut critical_context,
        &mut read_files,
        &mut modified_files,
    );

    // Merge read_files + modified_files into files_touched for backward compat
    let mut files_touched = read_files.clone();
    for f in &modified_files {
        if !files_touched.contains(f) {
            files_touched.push(f.clone());
        }
    }

    crate::session::CompactionSummary {
        goal,
        constraints,
        progress,
        blockers,
        decisions,
        next_steps,
        critical_context,
        read_files,
        modified_files,
        files_touched,
    }
}

#[allow(clippy::too_many_arguments)]
fn flush_section(
    section: &str,
    body: &str,
    goal: &mut String,
    constraints: &mut String,
    progress: &mut String,
    blockers: &mut String,
    decisions: &mut String,
    next_steps: &mut String,
    critical_context: &mut String,
    read_files: &mut Vec<String>,
    modified_files: &mut Vec<String>,
) {
    let trimmed = body.trim().to_string();
    match section {
        "Goal" => *goal = trimmed,
        "Constraints & Preferences" => *constraints = trimmed,
        "Progress" => *progress = trimmed,
        "Blockers" => *blockers = trimmed,
        "Decisions" => *decisions = trimmed,
        "Next Steps" => *next_steps = trimmed,
        "Critical Context" => *critical_context = trimmed,
        "Files" => {
            for line in body.lines() {
                if let Some(files) = line.strip_prefix("Read:") {
                    *read_files = parse_file_list(files);
                } else if let Some(files) = line.strip_prefix("Modified:") {
                    *modified_files = parse_file_list(files);
                }
            }
        }
        _ => {}
    }
}

fn parse_file_list(s: &str) -> Vec<String> {
    s.split(',')
        .map(|f| f.trim().to_string())
        .filter(|f| !f.is_empty() && f != "(none)")
        .collect()
}
