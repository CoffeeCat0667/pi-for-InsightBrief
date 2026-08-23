use pyo3::prelude::*;

pub mod agent;
pub mod llm;
pub mod python;
pub mod session;
pub mod tools;

/// Pi Agent - A coding agent with session management and context compaction.
#[pymodule]
mod pi_agent {
    use pyo3::prelude::*;

    use super::python::agent::{PyAgent, PyTool, PyToolDefinition};
    use super::python::types::{
        PyAgentEvent, PyBranchPoint, PyBranchSummary, PyEntry, PySession, PyUsage,
    };

    /// A session entry.
    #[pyfunction]
    fn create_entry_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Get the version of pi_agent.
    #[pyfunction]
    fn version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    // Register Python types.
    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_class::<PyEntry>()?;
        m.add_class::<PyUsage>()?;
        m.add_class::<PySession>()?;
        m.add_class::<PyBranchPoint>()?;
        m.add_class::<PyBranchSummary>()?;
        m.add_class::<PyAgent>()?;
        m.add_class::<PyAgentEvent>()?;
        m.add_class::<PyTool>()?;
        m.add_class::<PyToolDefinition>()?;
        Ok(())
    }
}
