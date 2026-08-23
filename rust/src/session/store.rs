use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use chrono::Utc;
use uuid::Uuid;

use super::types::{
    CompactionEntry, CompactionSummary, Entry, EntryId, Header, SessionHeader, Usage,
};

/// Error type for session operations.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Invalid session: {0}")]
    InvalidSession(String),

    #[error("Entry not found: {0}")]
    EntryNotFound(EntryId),
}

pub type SessionResult<T> = Result<T, SessionError>;

/// Session store with JSONL tree storage.
pub struct SessionStore {
    /// Path to the JSONL file.
    pub path: PathBuf,
    /// Session header.
    pub header: Option<Header>,
    /// All entries indexed by ID.
    pub entries: HashMap<EntryId, Entry>,
    /// Compaction entries.
    pub compactions: Vec<CompactionEntry>,
    /// Root entry IDs (multiple roots possible for branches).
    pub roots: Vec<EntryId>,
    /// Current leaf entry ID.
    pub leaf: Option<EntryId>,
    /// File handle for append writes.
    file: Option<File>,
    /// Whether this store is backed only by in-memory data.
    in_memory: bool,
}

impl SessionStore {
    /// Create a new session store.
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            header: None,
            entries: HashMap::new(),
            compactions: Vec::new(),
            roots: Vec::new(),
            leaf: None,
            file: None,
            in_memory: false,
        }
    }

    /// Create a new session with the given model.
    pub fn create(model: &str, system_prompt: Option<&str>) -> Self {
        let path = PathBuf::from(format!(
            "~/.pi/sessions/{}.jsonl",
            Uuid::new_v4()
        ));
        let mut store = Self::new(path);
        store.header = Some(Header {
            session_id: Uuid::new_v4(),
            created_at: Utc::now(),
            model: model.to_string(),
            system_prompt: system_prompt.map(|s| s.to_string()),
        });

        // Add system prompt as first entry if provided
        if let Some(prompt) = system_prompt {
            let entry = Entry::new_system(prompt.to_string());
            store.roots.push(entry.id);
            store.leaf = Some(entry.id);
            store.entries.insert(entry.id, entry);
        }

        store
    }

    /// Load a session from a JSONL file.
    pub fn load(path: impl AsRef<Path>) -> SessionResult<Self> {
        let path = path.as_ref().to_path_buf();
        let mut store = Self::new(&path);

        if !path.exists() {
            return Ok(store);
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);

        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let json: serde_json::Value = serde_json::from_str(line)?;

            match json.get("type").and_then(|t| t.as_str()) {
                Some("header") => {
                    let session_header: SessionHeader = serde_json::from_value(json)?;
                    store.header = Some(session_header.header);
                }
                Some("entry") => {
                    let entry: Entry = serde_json::from_value(json)?;
                    store.entries.insert(entry.id, entry);
                }
                Some("compaction") => {
                    let compaction: CompactionEntry = serde_json::from_value(json)?;
                    store.compactions.push(compaction);
                }
                _ => {
                    eprintln!("Warning: unknown line type at line {}", line_num + 1);
                }
            }
        }

        // Rebuild tree structure
        store.rebuild_tree();

        // Verify file consistency and fix if corrupted
        store.ensure_file()?;

        Ok(store)
    }

    /// Create a session store from an in-memory session object.
    ///
    /// The object uses the same logical records as the JSONL format:
    /// `{ "header": ..., "entries": [...], "compactions": [...] }`.
    pub fn from_data(data: serde_json::Value) -> SessionResult<Self> {
        let object = data
            .as_object()
            .ok_or_else(|| SessionError::InvalidSession("session data must be an object".to_string()))?;

        let mut store = Self::new("");
        store.in_memory = true;

        if let Some(header) = object.get("header") {
            store.header = Some(serde_json::from_value(header.clone())?);
        }

        if let Some(entries) = object.get("entries") {
            let entries: Vec<Entry> = serde_json::from_value(entries.clone())?;
            for entry in entries {
                store.entries.insert(entry.id, entry);
            }
        }

        if let Some(compactions) = object.get("compactions") {
            store.compactions = serde_json::from_value(compactions.clone())?;
        }

        store.rebuild_tree();

        if let Some(leaf) = object.get("leaf").and_then(|value| value.as_str()) {
            let leaf = leaf
                .parse::<EntryId>()
                .map_err(|_| SessionError::InvalidSession("invalid leaf ID".to_string()))?;
            if store.entries.contains_key(&leaf) {
                store.leaf = Some(leaf);
            }
        }

        Ok(store)
    }

    /// Export the complete in-memory session tree.
    pub fn to_data(&self) -> serde_json::Value {
        let mut entries: Vec<Entry> = self.entries.values().cloned().collect();
        entries.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        serde_json::json!({
            "header": self.header,
            "entries": entries,
            "compactions": self.compactions,
            "roots": self.roots,
            "leaf": self.leaf,
        })
    }

    /// Rebuild the tree structure from entries.
    fn rebuild_tree(&mut self) {
        self.roots.clear();
        self.leaf = None;

        // Find root entries (no parent or parent not in entries)
        let mut roots: Vec<EntryId> = self
            .entries
            .values()
            .filter(|e| e.parent_id.is_none() || !self.entries.contains_key(&e.parent_id.unwrap()))
            .map(|e| e.id)
            .collect();

        // Sort by creation time for deterministic order
        roots.sort_by(|a, b| {
            let ea = self.entries.get(a).unwrap();
            let eb = self.entries.get(b).unwrap();
            ea.created_at.cmp(&eb.created_at)
        });

        self.roots = roots;

        // Find leaf: the active entry with no active children
        if let Some(&root_id) = self.roots.first() {
            self.leaf = Some(self.find_leaf(root_id));
        }
    }

    /// Find the leaf node from a given root.
    fn find_leaf(&self, start_id: EntryId) -> EntryId {
        let mut current = start_id;
        loop {
            let child = self.entries.values().find(|e| {
                e.parent_id == Some(current) && e.is_active
            });
            match child {
                Some(child) => current = child.id,
                None => return current,
            }
        }
    }

    /// Append an entry to the session.
    pub fn append(&mut self, mut entry: Entry) -> SessionResult<EntryId> {
        // Set parent to current leaf
        entry.parent_id = self.leaf;
        entry.is_active = true;

        let id = entry.id;

        // Mark siblings as inactive (for branching)
        if let Some(parent_id) = entry.parent_id {
            for sibling in self.entries.values_mut() {
                if sibling.parent_id == Some(parent_id) && sibling.id != id {
                    sibling.is_active = false;
                }
            }
        } else {
            // Root entry - mark other roots as inactive
            for &root_id in &self.roots {
                if root_id != id {
                    if let Some(root) = self.entries.get_mut(&root_id) {
                        root.is_active = false;
                    }
                }
            }
        }

        // Insert entry
        self.entries.insert(id, entry);

        // Update leaf
        self.leaf = Some(id);

        // Add to roots if it's a root entry
        if self.entries[&id].parent_id.is_none() {
            self.roots.push(id);
        }

        // Get entry data for file write
        let entry_data = self.entries[&id].clone();

        // Append to file
        self.append_to_file(&entry_data)?;

        Ok(id)
    }

    /// Append an entry to the JSONL file.
    fn append_to_file(&mut self, entry: &Entry) -> SessionResult<()> {
        if self.in_memory {
            return Ok(());
        }
        // Ensure file exists and header is written
        if self.file.is_none() {
            self.ensure_file()?;
        }

        if let Some(ref mut file) = self.file {
            let json = serde_json::to_string(entry)?;
            writeln!(file, "{}", json)?;
            file.flush()?;
        }

        Ok(())
    }

    /// Ensure the file exists and header is written.
    fn ensure_file(&mut self) -> SessionResult<()> {
        let path = self.expand_path();

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = if path.exists() {
            // File exists - verify it's not corrupted (has duplicate or missing entries)
            // by reading it and comparing with in-memory state
            let (existing_ids, file_line_count) = {
                let file = File::open(&path)?;
                let reader = BufReader::new(file);
                let mut ids = std::collections::HashSet::new();
                let mut line_count = 0u32;
                for line in reader.lines() {
                    let line = line?;
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    line_count += 1;
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some("entry") = json.get("type").and_then(|t| t.as_str()) {
                            if let Some(id_str) = json.get("id").and_then(|id| id.as_str()) {
                                if let Ok(id) = EntryId::from_str(id_str) {
                                    ids.insert(id);
                                }
                            }
                        }
                    }
                }
                (ids, line_count)
            };

            // Check if file is inconsistent:
            // - entries in memory missing from file
            // - file has duplicate entries (more lines than unique IDs)
            let memory_ids: std::collections::HashSet<EntryId> =
                self.entries.keys().cloned().collect();
            let memory_count = memory_ids.len() as u32;
            let file_entry_count = existing_ids.len() as u32;
            let has_missing = memory_ids.difference(&existing_ids).count() > 0;
            let has_duplicates = file_line_count != file_entry_count || file_entry_count != memory_count;
            let needs_rewrite = has_missing || has_duplicates;

            if needs_rewrite {
                // Rewrite the entire file to fix corruption
                // Truncate and rewrite
                let mut f = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&path)?;

                // Write header
                if let Some(ref header) = self.header {
                    let session_header = SessionHeader {
                        entry_type: "header".to_string(),
                        header: header.clone(),
                    };
                    let json = serde_json::to_string(&session_header)?;
                    writeln!(f, "{}", json)?;
                }

                // Write all entries in order
                let mut entries: Vec<Entry> = self.entries.values().cloned().collect();
                entries.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                for entry in entries {
                    let json = serde_json::to_string(&entry)?;
                    writeln!(f, "{}", json)?;
                }

                f
            } else {
                // File is consistent, just open in append mode
                OpenOptions::new().append(true).open(&path)?
            }
        } else {
            let mut f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;

            // Write header
            if let Some(ref header) = self.header {
                let session_header = SessionHeader {
                    entry_type: "header".to_string(),
                    header: header.clone(),
                };
                let json = serde_json::to_string(&session_header)?;
                writeln!(f, "{}", json)?;
            }

            // Write all existing entries (e.g., system prompt added during create)
            let mut entries: Vec<Entry> = self.entries.values().cloned().collect();
            entries.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            for entry in entries {
                let json = serde_json::to_string(&entry)?;
                writeln!(f, "{}", json)?;
            }

            f
        };

        self.file = Some(file);
        Ok(())
    }

    /// Expand ~ in path.
    fn expand_path(&self) -> PathBuf {
        let path_str = self.path.to_string_lossy().to_string();
        if path_str.starts_with('~') {
            if let Some(home) = std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
            {
                PathBuf::from(path_str.replacen('~', &home.to_string_lossy(), 1))
            } else {
                self.path.clone()
            }
        } else {
            self.path.clone()
        }
    }

    /// Get an entry by ID.
    pub fn get(&self, id: EntryId) -> Option<&Entry> {
        self.entries.get(&id)
    }

    /// Get the current leaf entry.
    pub fn leaf(&self) -> Option<&Entry> {
        self.leaf.and_then(|id| self.entries.get(&id))
    }

    /// Get the root entries.
    pub fn roots(&self) -> Vec<&Entry> {
        self.roots
            .iter()
            .filter_map(|id| self.entries.get(id))
            .collect()
    }

    /// Get all entries in the current branch (root to leaf).
    pub fn branch(&self) -> Vec<&Entry> {
        let mut path = Vec::new();
        let mut current = self.leaf;

        while let Some(id) = current {
            if let Some(entry) = self.entries.get(&id) {
                path.push(entry);
                current = entry.parent_id;
            } else {
                break;
            }
        }

        path.reverse();
        path
    }

    /// Switch to a different branch by setting the leaf to the given entry ID.
    /// This changes the active branch from the given entry down.
    pub fn switch_branch(&mut self, entry_id: EntryId) -> SessionResult<()> {
        // Verify the entry exists
        if !self.entries.contains_key(&entry_id) {
            return Err(SessionError::EntryNotFound(entry_id));
        }

        // Mark all entries as inactive
        for entry in self.entries.values_mut() {
            entry.is_active = false;
        }

        // Activate the path from root to the given entry
        let mut current = Some(entry_id);
        while let Some(id) = current {
            if let Some(entry) = self.entries.get_mut(&id) {
                entry.is_active = true;
                current = entry.parent_id;
            } else {
                break;
            }
        }

        // Set the leaf to the given entry
        self.leaf = Some(entry_id);

        Ok(())
    }

    /// Get all branch points (entries with multiple children).
    pub fn branch_points(&self) -> Vec<BranchPoint> {
        let mut children_map: HashMap<EntryId, Vec<EntryId>> = HashMap::new();

        // Build parent -> children map
        for entry in self.entries.values() {
            if let Some(parent_id) = entry.parent_id {
                children_map.entry(parent_id).or_default().push(entry.id);
            }
        }

        // Find entries with multiple children
        children_map
            .into_iter()
            .filter(|(_, children)| children.len() > 1)
            .map(|(parent_id, children)| BranchPoint {
                parent_id,
                children,
            })
            .collect()
    }

    /// Get the session header.
    pub fn header(&self) -> Option<&Header> {
        self.header.as_ref()
    }

    /// Get total token usage for the current branch.
    pub fn total_usage(&self) -> Usage {
        let mut usage = Usage::default();
        for entry in self.branch() {
            usage.input_tokens += entry.usage.input_tokens;
            usage.output_tokens += entry.usage.output_tokens;
            usage.cache_read_tokens += entry.usage.cache_read_tokens;
            usage.cache_write_tokens += entry.usage.cache_write_tokens;
        }
        usage
    }

    /// Get all messages in the current branch.
    pub fn messages(&self) -> Vec<&super::types::Message> {
        self.branch().iter().map(|e| &e.message).collect()
    }

    /// Get the compaction entries.
    pub fn compactions(&self) -> &[CompactionEntry] {
        &self.compactions
    }

    /// Get the last compaction summary.
    pub fn last_compaction(&self) -> Option<&CompactionSummary> {
        self.compactions.last().map(|c| &c.summary)
    }

    /// Save an entry directly (for compaction).
    pub fn save_entry(&mut self, entry: Entry) -> SessionResult<()> {
        if self.in_memory {
            self.entries.insert(entry.id, entry);
            return Ok(());
        }
        self.ensure_file()?;

        if let Some(ref mut file) = self.file {
            let json = serde_json::to_string(&entry)?;
            writeln!(file, "{}", json)?;
            file.flush()?;
        }

        self.entries.insert(entry.id, entry);
        Ok(())
    }

    /// Save a compaction entry.
    pub fn save_compaction(&mut self, compaction: CompactionEntry) -> SessionResult<()> {
        if self.in_memory {
            self.compactions.push(compaction);
            return Ok(());
        }
        self.ensure_file()?;

        if let Some(ref mut file) = self.file {
            let json = serde_json::to_string(&compaction)?;
            writeln!(file, "{}", json)?;
            file.flush()?;
        }

        self.compactions.push(compaction);
        Ok(())
    }

    /// Generate a summary for the current branch (for abandoned paths).
    /// This is used when switching branches to create a summary of what was abandoned.
    pub fn generate_branch_summary(&self, branch_leaf: EntryId) -> Option<CompactionSummary> {
        // Get all entries in this branch
        let mut entries = Vec::new();
        let mut current = Some(branch_leaf);

        while let Some(id) = current {
            if let Some(entry) = self.entries.get(&id) {
                entries.push(entry);
                current = entry.parent_id;
            } else {
                break;
            }
        }

        if entries.is_empty() {
            return None;
        }

        // Calculate total tokens
        let total_tokens: u32 = entries
            .iter()
            .map(|e| e.usage.total())
            .sum();

        // Collect files touched from all entries
        let mut files_touched = Vec::new();
        for entry in &entries {
            if let Some(tool_calls) = &entry.message.tool_calls {
                for call in tool_calls {
                    if let Ok(args) = serde_json::from_str::<serde_json::Value>(&call.arguments) {
                        if let Some(path) = args.get("path").or_else(|| args.get("file_path")) {
                            if let Some(path_str) = path.as_str() {
                                if !files_touched.contains(&path_str.to_string()) {
                                    files_touched.push(path_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Build a simple summary
        let goal = format!(
            "Abandoned branch with {} entries ({} tokens)",
            entries.len(),
            total_tokens
        );

        Some(CompactionSummary {
            goal,
            constraints: String::new(),
            progress: String::new(),
            blockers: String::new(),
            decisions: String::new(),
            next_steps: String::new(),
            critical_context: String::new(),
            read_files: Vec::new(),
            modified_files: Vec::new(),
            files_touched,
        })
    }
}

/// Represents a branch point (entry with multiple children).
#[derive(Debug, Clone)]
pub struct BranchPoint {
    pub parent_id: EntryId,
    pub children: Vec<EntryId>,
}

impl std::fmt::Debug for SessionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionStore")
            .field("path", &self.path)
            .field("entries", &self.entries.len())
            .field("roots", &self.roots.len())
            .field("leaf", &self.leaf)
            .finish()
    }
}
