use pyo3::prelude::*;

use crate::session::{Entry, Role, SessionStore, Usage};

/// Python wrapper for Usage.
#[pyclass(from_py_object)]
#[derive(Debug, Clone, Default)]
pub struct PyUsage {
    pub(crate) inner: Usage,
}

#[pymethods]
impl PyUsage {
    #[new]
    fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            inner: Usage {
                input_tokens,
                output_tokens,
                ..Default::default()
            },
        }
    }

    #[getter]
    fn input_tokens(&self) -> u32 {
        self.inner.input_tokens
    }

    #[getter]
    fn output_tokens(&self) -> u32 {
        self.inner.output_tokens
    }

    #[getter]
    fn total(&self) -> u32 {
        self.inner.total()
    }
}

/// Python wrapper for Entry.
#[pyclass(from_py_object)]
#[derive(Debug, Clone)]
pub struct PyEntry {
    pub(crate) inner: Entry,
}

#[pymethods]
impl PyEntry {
    /// Unique identifier for this entry.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// Parent entry ID.
    #[getter]
    fn parent_id(&self) -> Option<String> {
        self.inner.parent_id.map(|id| id.to_string())
    }

    /// Message content.
    #[getter]
    fn content(&self) -> &str {
        &self.inner.message.content
    }

    /// Message role.
    #[getter]
    fn role(&self) -> &str {
        match self.inner.message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
        }
    }

    /// Input tokens.
    #[getter]
    fn input_tokens(&self) -> u32 {
        self.inner.usage.input_tokens
    }

    /// Output tokens.
    #[getter]
    fn output_tokens(&self) -> u32 {
        self.inner.usage.output_tokens
    }

    fn __repr__(&self) -> String {
        format!("Entry(id={}, role={})", self.inner.id, self.role())
    }
}

impl From<Entry> for PyEntry {
    fn from(entry: Entry) -> Self {
        Self { inner: entry }
    }
}

/// Python wrapper for AgentEvent.
#[pyclass(from_py_object)]
#[derive(Debug, Clone)]
pub struct PyAgentEvent {
    pub(crate) inner: crate::agent::AgentEvent,
}

#[pymethods]
impl PyAgentEvent {
    /// Get the event type as a string.
    #[getter]
    fn event_type(&self) -> &str {
        match &self.inner {
            crate::agent::AgentEvent::AgentStart { .. } => "agent_start",
            crate::agent::AgentEvent::AgentEnd { .. } => "agent_end",
            crate::agent::AgentEvent::TurnStart { .. } => "turn_start",
            crate::agent::AgentEvent::TurnEnd { .. } => "turn_end",
            crate::agent::AgentEvent::MessageEnd { .. } => "message_end",
            crate::agent::AgentEvent::ToolCallStart { .. } => "tool_call_start",
            crate::agent::AgentEvent::ToolCallEnd { .. } => "tool_call_end",
            crate::agent::AgentEvent::CompactionStart { .. } => "compaction_start",
            crate::agent::AgentEvent::CompactionEnd { .. } => "compaction_end",
            crate::agent::AgentEvent::StreamToken { .. } => "stream_token",
        }
    }

    /// Get the message content (for MessageEnd / AgentEnd events).
    #[getter]
    fn content(&self) -> Option<String> {
        match &self.inner {
            crate::agent::AgentEvent::MessageEnd { message, .. } => {
                Some(message.content.clone())
            }
            crate::agent::AgentEvent::AgentEnd { message, .. } => {
                Some(message.content.clone())
            }
            crate::agent::AgentEvent::StreamToken { token, .. } => Some(token.clone()),
            _ => None,
        }
    }

    /// Get the tool name (for ToolCallStart events).
    #[getter]
    fn tool_name(&self) -> Option<String> {
        match &self.inner {
            crate::agent::AgentEvent::ToolCallStart { tool_call, .. } => {
                Some(tool_call.name.clone())
            }
            _ => None,
        }
    }

    /// Get the tool call ID (for ToolCallStart events).
    #[getter]
    fn tool_call_id(&self) -> Option<String> {
        match &self.inner {
            crate::agent::AgentEvent::ToolCallStart { tool_call, .. } => {
                Some(tool_call.id.clone())
            }
            crate::agent::AgentEvent::ToolCallEnd { result, .. } => {
                Some(result.tool_call_id.clone())
            }
            _ => None,
        }
    }

    /// Get the summary (for CompactionEnd events).
    #[getter]
    fn summary(&self) -> Option<String> {
        match &self.inner {
            crate::agent::AgentEvent::CompactionEnd { summary, .. } => Some(summary.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("AgentEvent(type={})", self.event_type())
    }
}

impl From<crate::agent::AgentEvent> for PyAgentEvent {
    fn from(event: crate::agent::AgentEvent) -> Self {
        Self { inner: event }
    }
}

/// Python wrapper for SessionStore.
#[pyclass]
pub struct PySession {
    pub(crate) inner: SessionStore,
}

#[pymethods]
impl PySession {
    /// Create a new session.
    #[new]
    fn new(model: &str, system_prompt: Option<&str>) -> Self {
        Self {
            inner: SessionStore::create(model, system_prompt),
        }
    }

    /// Load a session from a JSONL file.
    #[staticmethod]
    fn from_file(path: &str) -> PyResult<Self> {
        let store = SessionStore::load(path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        Ok(Self { inner: store })
    }

    /// Append a user message to the session.
    fn append_user(&mut self, content: &str) -> PyResult<String> {
        let entry = Entry::new_user(content.to_string(), None);
        let id = self.inner.append(entry)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        Ok(id.to_string())
    }

    /// Append an assistant message to the session.
    fn append_assistant(&mut self, content: &str) -> PyResult<String> {
        let entry = Entry::new_assistant(content.to_string(), None);
        let id = self.inner.append(entry)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        Ok(id.to_string())
    }

    /// Switch to a different branch by setting the leaf to the given entry ID.
    fn switch_branch(&mut self, entry_id: &str) -> PyResult<()> {
        let id: uuid::Uuid = entry_id.parse()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid entry ID"))?;
        self.inner.switch_branch(id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        Ok(())
    }

    /// Get all branch points (entries with multiple children).
    fn branch_points(&self) -> Vec<PyBranchPoint> {
        self.inner.branch_points().into_iter().map(|bp| {
            PyBranchPoint {
                parent_id: bp.parent_id.to_string(),
                children: bp.children.into_iter().map(|id| id.to_string()).collect(),
            }
        }).collect()
    }

    /// Generate a summary for a branch (for abandoned paths).
    fn branch_summary(&self, entry_id: &str) -> PyResult<Option<PyBranchSummary>> {
        let id: uuid::Uuid = entry_id.parse()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid entry ID"))?;
        Ok(self.inner.generate_branch_summary(id).map(|s| {
            PyBranchSummary {
                goal: s.goal,
                progress: s.progress,
                decisions: s.decisions,
                next_steps: s.next_steps,
                files_touched: s.files_touched,
            }
        }))
    }

    /// Get the current leaf entry.
    fn leaf(&self) -> Option<PyEntry> {
        self.inner.leaf().map(|e| PyEntry::from(e.clone()))
    }

    /// Get all messages in the current branch.
    fn messages(&self) -> Vec<PyEntry> {
        self.inner.branch().iter().map(|e| PyEntry::from((*e).clone())).collect()
    }

    /// Get total token usage.
    fn total_usage(&self) -> PyUsage {
        PyUsage {
            inner: self.inner.total_usage(),
        }
    }

    /// Get the session model.
    fn model(&self) -> Option<String> {
        self.inner.header().map(|h| h.model.clone())
    }

    /// Get the number of entries.
    fn entry_count(&self) -> usize {
        self.inner.branch().len()
    }

    /// Get the session ID.
    fn session_id(&self) -> Option<String> {
        self.inner.header().map(|h| h.session_id.to_string())
    }
}

/// Python wrapper for BranchPoint.
#[pyclass(from_py_object)]
#[derive(Debug, Clone)]
pub struct PyBranchPoint {
    parent_id: String,
    children: Vec<String>,
}

#[pymethods]
impl PyBranchPoint {
    #[getter]
    fn parent_id(&self) -> &str {
        &self.parent_id
    }

    #[getter]
    fn children(&self) -> Vec<String> {
        self.children.clone()
    }
}

/// Python wrapper for branch summary.
#[pyclass(from_py_object)]
#[derive(Debug, Clone)]
pub struct PyBranchSummary {
    goal: String,
    progress: String,
    decisions: String,
    next_steps: String,
    files_touched: Vec<String>,
}

#[pymethods]
impl PyBranchSummary {
    #[getter]
    fn goal(&self) -> &str {
        &self.goal
    }

    #[getter]
    fn progress(&self) -> &str {
        &self.progress
    }

    #[getter]
    fn decisions(&self) -> &str {
        &self.decisions
    }

    #[getter]
    fn next_steps(&self) -> &str {
        &self.next_steps
    }

    #[getter]
    fn files_touched(&self) -> Vec<String> {
        self.files_touched.clone()
    }
}
