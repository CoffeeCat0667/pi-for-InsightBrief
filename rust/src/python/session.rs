use pyo3::prelude::*;
use std::path::PathBuf;

use crate::session::{Entry, SessionStore};

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
}

use super::types::{Entry as EntryType, Role, Usage};
use super::types::{Entry as _, Role as _, Usage as _};
