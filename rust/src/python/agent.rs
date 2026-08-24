use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::broadcast;

use crate::agent::{AgentEvent, AgentLoop, AgentLoopConfig};
use crate::llm::LlmClient;
use crate::session::{Header, SessionStore};
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
///
/// Holds a persistent Tokio runtime and a shared AgentLoop.
/// `run()` spawns a background task on the runtime and releases the GIL,
/// allowing Python to call `next_event()` concurrently for real-time streaming.
#[pyclass]
pub struct PyAgent {
    /// Shared agent loop (Send + Sync, safe to clone into spawned tasks).
    agent: Arc<AgentLoop>,
    /// Persistent Tokio runtime for spawning background tasks.
    runtime: tokio::runtime::Runtime,
    /// Event receiver for polling events from Python.
    event_receiver: broadcast::Receiver<AgentEvent>,
    /// Cancellation flag shared with the spawned task.
    cancel_flag: Arc<std::sync::atomic::AtomicBool>,
    /// Whether a task is currently running. Shared so spawned task can reset it.
    running: Arc<std::sync::atomic::AtomicBool>,
}

#[pymethods]
impl PyAgent {
    /// Create a new agent.
    #[new]
    #[pyo3(signature = (api_key, model, session_id, session_data=None, base_url=None, system_prompt=None, max_turns=None, max_retries=None, reserve_tokens=None, keep_recent_tokens=None, context_window=None, append_system_prompt=None, extra_guidelines=None, cwd=None))]
    fn new(
        py: Python<'_>,
        api_key: &str,
        model: &str,
        session_id: &str,
        session_data: Option<&Bound<'_, PyDict>>,
        base_url: Option<&str>,
        system_prompt: Option<&str>,
        max_turns: Option<u32>,
        max_retries: Option<u32>,
        reserve_tokens: Option<u32>,
        keep_recent_tokens: Option<u32>,
        context_window: Option<u32>,
        append_system_prompt: Option<&str>,
        extra_guidelines: Option<Vec<String>>,
        cwd: Option<&str>,
    ) -> PyResult<Self> {
        // Create persistent runtime
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Create broadcast channel
        let (event_sender, event_receiver) = broadcast::channel(10000);
        let event_sender = Arc::new(event_sender);

        // Create LLM client with shared event sender
        let llm_client = Arc::new(LlmClient::new(
            api_key.to_string(),
            base_url.map(|s| s.to_string()),
            Some(event_sender.clone()),
        ));

        // Create or load the session from in-memory Python data.
        let session_json = if let Some(data) = session_data {
            let value = data
                .get_item(session_id)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyTypeError, _>(e.to_string()))?
                .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>(session_id.to_string()))?;
            let json_module = py.import("json")?;
            json_module
                .call_method1("dumps", (value,))?
                .extract::<String>()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyTypeError, _>(e.to_string()))?
        } else {
            "{}".to_string()
        };

        let mut session_store = SessionStore::from_data(serde_json::from_str(&session_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        if session_store.header.is_none() {
            session_store.header = Some(Header {
                session_id: session_id.parse().unwrap_or_else(|_| uuid::Uuid::new_v4()),
                created_at: chrono::Utc::now(),
                model: model.to_string(),
                system_prompt: system_prompt.map(str::to_string),
            });
        }
        let session = Arc::new(tokio::sync::Mutex::new(session_store));

        // Build prompt set with optional overrides
        let mut prompt_set = crate::agent::PromptSet::defaults();
        if let Some(custom) = system_prompt {
            prompt_set = prompt_set.override_system_main(custom);
        }
        if let Some(append) = append_system_prompt {
            let combined = format!("{}\n\n{}", prompt_set.system_main, append);
            prompt_set = prompt_set.override_system_main(&combined);
        }

        // Create agent config
        let config = AgentLoopConfig {
            model: model.to_string(),
            max_turns: max_turns.unwrap_or(50),
            max_retries: max_retries.unwrap_or(10),
            reserve_tokens: reserve_tokens.unwrap_or(16384),
            keep_recent_tokens: keep_recent_tokens.unwrap_or(20000),
            context_window: context_window.unwrap_or(128000),
            prompt_set,
            extra_guidelines: extra_guidelines.unwrap_or_default(),
        };

        // Create agent loop
        let mut agent_loop = AgentLoop::new(
            config,
            llm_client,
            session,
            event_sender,
        );

        // Set working directory if provided
        if let Some(dir) = cwd {
            agent_loop.set_cwd(dir);
        }

        let cancel_flag = agent_loop.cancel_flag();
        let agent = Arc::new(agent_loop);

        Ok(Self {
            agent,
            runtime,
            event_receiver,
            cancel_flag,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Run the agent with a prompt.
    ///
    /// Spawns a background Tokio task and releases the GIL during execution.
    /// Events are broadcast in real-time and can be polled via `next_event()`.
    fn run(&mut self, prompt: &str) -> PyResult<()> {
        if self.running.load(Ordering::Relaxed) {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "A task is already running. Use cancel() to stop it first."
            ));
        }

        self.running.store(true, Ordering::Relaxed);
        self.cancel_flag.store(false, Ordering::Relaxed);

        let agent = self.agent.clone();
        let prompt = prompt.to_string();
        let running = self.running.clone();
        let sender = agent.event_sender();

        // Spawn the task on the Tokio runtime.
        // The task runs on Tokio's thread pool, independent of the GIL.
        // Python can call next_event() concurrently because:
        // 1. The broadcast channel is thread-safe
        // 2. The spawned task doesn't need the GIL
        // 3. The GIL is released when this method returns to Python
        self.runtime.spawn(async move {
            let result = agent.run(&prompt).await;

            // Always reset running flag when task completes
            running.store(false, Ordering::Relaxed);

            // If the agent loop returned an error, emit an error event so Python
            // can see what went wrong instead of silently swallowing it.
            if let Err(e) = result {
                let _ = sender.send(AgentEvent::Debug {
                    source: "agent".to_string(),
                    level: "error".to_string(),
                    message: format!("Agent failed: {}", e),
                });
            }
        });

        Ok(())
    }

    /// Get the next event from the agent.
    ///
    /// Non-blocking. Returns None if no events are available.
    /// Call this in a loop while `is_running` returns True for real-time streaming.
    fn next_event(&mut self) -> Option<PyAgentEvent> {
        match self.event_receiver.try_recv() {
            Ok(event) => {
                // Check if this is the AgentEnd event (task finished)
                if matches!(event, AgentEvent::AgentEnd { .. }) {
                    self.running.store(false, Ordering::Relaxed);
                }
                Some(PyAgentEvent { inner: event })
            }
            Err(broadcast::error::TryRecvError::Empty) => None,
            Err(broadcast::error::TryRecvError::Closed) => {
                self.running.store(false, Ordering::Relaxed);
                None
            }
            Err(_) => None,
        }
    }

    /// Check if the agent task is currently running.
    fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Cancel the currently running task (cooperative cancellation).
    ///
    /// The task will finish its current turn and then stop.
    fn cancel(&mut self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }

    /// Wait for the current task to complete (blocking).
    ///
    /// This blocks the Python thread until the agent finishes.
    /// Prefer polling `next_event()` + `is_running()` for async usage.
    fn wait_done(&mut self) {
        if !self.running.load(Ordering::Relaxed) {
            return;
        }
        // Drain events while waiting
        while self.running.load(Ordering::Relaxed) {
            self.next_event();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// Export the complete current session tree as JSON.
    fn export_session_data(&self) -> PyResult<String> {
        let agent = self.agent.clone();
        self.runtime.block_on(async {
            let data = agent.export_session_data().await;
            serde_json::to_string(&data)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        })
    }

    /// Register a tool.
    fn register_tool(&mut self, tool: PyTool) {
        // Note: This must be called before the first run()
        // After run(), the agent is shared via Arc and tools can't be modified.
        // For now we accept this limitation; tools should be registered at construction time.
        // We use unsafe to get mutable access - this is safe because register_tool
        // is only called before run() is started.
        let agent = Arc::get_mut(&mut self.agent)
            .expect("Cannot register tools after the agent has started running");
        agent.register_tool(Box::new(PyToolWrapper(tool)));
    }

    /// Register built-in tools.
    fn register_builtin_tools(&mut self) {
        let agent = Arc::get_mut(&mut self.agent)
            .expect("Cannot register tools after the agent has started running");
        use crate::tools::{BashTool, EditTool, FindTool, GrepTool, LsTool, ReadTool, WriteTool};

        agent.register_tool(Box::new(BashTool));
        agent.register_tool(Box::new(ReadTool));
        agent.register_tool(Box::new(WriteTool));
        agent.register_tool(Box::new(EditTool));
        agent.register_tool(Box::new(GrepTool));
        agent.register_tool(Box::new(FindTool));
        agent.register_tool(Box::new(LsTool));
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
