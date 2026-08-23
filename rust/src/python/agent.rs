use pyo3::prelude::*;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::agent::{AgentEvent, AgentLoop, AgentLoopConfig};
use crate::llm::LlmClient;
use crate::session::SessionStore;
use crate::tools::{Tool, ToolDefinition};

use super::types::PyAgentEvent;

/// Python wrapper for Tool trait.
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct PyTool {
    inner: Arc<dyn Tool>,
}

#[pymethods]
impl PyTool {
    /// Get the tool definition.
    fn definition(&self) -> PyToolDefinition {
        let def = self.inner.definition();
        PyToolDefinition {
            name: def.name,
            description: def.description,
            parameters: def.parameters.to_string(),
        }
    }

    /// Execute the tool with the given arguments.
    fn execute(&self, arguments: &str) -> PyResult<String> {
        // This is a blocking call, but for simplicity we'll use a blocking context
        // In production, you'd want to use async properly
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let result = self.inner.execute(args)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(result)
    }
}

/// Python wrapper for ToolDefinition.
#[pyclass(from_py_object)]
#[derive(Debug, Clone)]
pub struct PyToolDefinition {
    name: String,
    description: String,
    parameters: String,
}

#[pymethods]
impl PyToolDefinition {
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    #[getter]
    fn description(&self) -> &str {
        &self.description
    }

    #[getter]
    fn parameters(&self) -> &str {
        &self.parameters
    }
}

/// Python wrapper for AgentLoop.
#[pyclass]
pub struct PyAgent {
    inner: AgentLoop,
    event_receiver: broadcast::Receiver<AgentEvent>,
}

#[pymethods]
impl PyAgent {
    /// Create a new agent.
    #[new]
    #[pyo3(signature = (api_key, model, session_path, base_url=None, system_prompt=None, max_turns=None, reserve_tokens=None, keep_recent_tokens=None, context_window=None, append_system_prompt=None, extra_guidelines=None, cwd=None))]
    fn new(
        api_key: &str,
        model: &str,
        session_path: &str,
        base_url: Option<&str>,
        system_prompt: Option<&str>,
        max_turns: Option<u32>,
        reserve_tokens: Option<u32>,
        keep_recent_tokens: Option<u32>,
        context_window: Option<u32>,
        append_system_prompt: Option<&str>,
        extra_guidelines: Option<Vec<String>>,
        cwd: Option<&str>,
    ) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        rt.block_on(async {
            // Create LLM client with custom base_url
            let llm_client = Arc::new(LlmClient::new(
                api_key.to_string(),
                base_url.map(|s| s.to_string()),
            ));

            // Create or load session
            let session = Arc::new(tokio::sync::Mutex::new(
                SessionStore::load(session_path)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?
            ));

            // Create event channel - large capacity to avoid dropping stream tokens
            let (event_sender, event_receiver) = broadcast::channel(10000);

            // Build prompt set with optional overrides
            let mut prompt_set = crate::agent::PromptSet::defaults();
            if let Some(custom) = system_prompt {
                prompt_set = prompt_set.override_system_main(custom);
            }
            if let Some(append) = append_system_prompt {
                // Append to the current system_main
                let combined = format!("{}\n\n{}", prompt_set.system_main, append);
                prompt_set = prompt_set.override_system_main(&combined);
            }

            // Create agent config
            let config = AgentLoopConfig {
                model: model.to_string(),
                max_turns: max_turns.unwrap_or(50),
                reserve_tokens: reserve_tokens.unwrap_or(16384),
                keep_recent_tokens: keep_recent_tokens.unwrap_or(20000),
                context_window: context_window.unwrap_or(128000),
                prompt_set,
                extra_guidelines: extra_guidelines.unwrap_or_default(),
            };

            // Create agent loop
            let mut agent = AgentLoop::new(
                config,
                llm_client,
                session,
                event_sender,
            );

            // Set working directory if provided
            if let Some(dir) = cwd {
                agent.set_cwd(dir);
            }

            // Register system prompt if provided
            if let Some(_prompt) = system_prompt {
                // The system prompt is already handled by the session
            }

            Ok(Self {
                inner: agent,
                event_receiver,
            })
        })
    }

    /// Run the agent with a prompt.
    fn run(&mut self, prompt: &str) -> PyResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        rt.block_on(self.inner.run(prompt))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Get the next event from the agent.
    fn next_event(&mut self) -> Option<PyAgentEvent> {
        match self.event_receiver.try_recv() {
            Ok(event) => Some(PyAgentEvent { inner: event }),
            Err(_) => None,
        }
    }

    /// Register a tool.
    fn register_tool(&mut self, tool: PyTool) {
        self.inner.register_tool(Box::new(PyToolWrapper(tool)));
    }

    /// Register built-in tools.
    fn register_builtin_tools(&mut self) {
        use crate::tools::{BashTool, EditTool, FindTool, GrepTool, LsTool, ReadTool, WriteTool};
        
        self.inner.register_tool(Box::new(BashTool));
        self.inner.register_tool(Box::new(ReadTool));
        self.inner.register_tool(Box::new(WriteTool));
        self.inner.register_tool(Box::new(EditTool));
        self.inner.register_tool(Box::new(GrepTool));
        self.inner.register_tool(Box::new(FindTool));
        self.inner.register_tool(Box::new(LsTool));
    }
}

/// Wrapper to make PyTool implement Tool trait.
struct PyToolWrapper(PyTool);

impl Tool for PyToolWrapper {
    fn definition(&self) -> ToolDefinition {
        let def = self.0.definition();
        ToolDefinition {
            name: def.name,
            description: def.description,
            parameters: serde_json::from_str(&def.parameters).unwrap_or_default(),
        }
    }

    fn execute(&self, args: serde_json::Value) -> Result<String, crate::tools::ToolError> {
        self.0.execute(&args.to_string())
            .map_err(|e| crate::tools::ToolError::ExecutionError(e.to_string()))
    }
}


