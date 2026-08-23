use std::collections::HashMap;

/// Guideline for a single tool.
#[derive(Debug, Clone)]
pub struct ToolGuideline {
    /// One-line description shown in the tools list.
    pub snippet: String,
    /// Behavioral guidelines for using this tool.
    pub guidelines: Vec<String>,
}

/// Container for all prompt texts. Each field corresponds to a `.md` file
/// under `rust/prompts/`. Edit the files and recompile to change prompts.
/// Use the `override_*` methods or Python parameters to customize at runtime.
#[derive(Debug, Clone)]
pub struct PromptSet {
    /// Main system prompt template. Placeholders: {tools_list}, {guidelines},
    /// {project_context}, {cwd}.
    pub system_main: String,
    /// System message for compaction LLM calls.
    pub compaction_system: String,
    /// User prompt for first-time compaction.
    pub compaction_initial: String,
    /// User prompt for incremental compaction (updating existing summary).
    pub compaction_update: String,
    /// User prompt for summarizing a split-turn prefix.
    pub compaction_turn_prefix: String,
    /// Per-tool snippets and guidelines.
    pub tool_guidelines: HashMap<String, ToolGuideline>,
}

impl PromptSet {
    /// Build a PromptSet from compile-time embedded defaults.
    pub fn defaults() -> Self {
        Self {
            system_main: include_str!("../../prompts/system_main.md").to_string(),
            compaction_system: include_str!("../../prompts/compaction_system.md").to_string(),
            compaction_initial: include_str!("../../prompts/compaction_initial.md").to_string(),
            compaction_update: include_str!("../../prompts/compaction_update.md").to_string(),
            compaction_turn_prefix: include_str!("../../prompts/compaction_turn_prefix.md")
                .to_string(),
            tool_guidelines: load_tool_guidelines(),
        }
    }

    // -- override helpers ---------------------------------------------------

    pub fn override_system_main(mut self, v: &str) -> Self {
        self.system_main = v.to_string();
        self
    }

    pub fn override_compaction_system(mut self, v: &str) -> Self {
        self.compaction_system = v.to_string();
        self
    }

    pub fn override_compaction_initial(mut self, v: &str) -> Self {
        self.compaction_initial = v.to_string();
        self
    }

    pub fn override_compaction_update(mut self, v: &str) -> Self {
        self.compaction_update = v.to_string();
        self
    }

    pub fn override_compaction_turn_prefix(mut self, v: &str) -> Self {
        self.compaction_turn_prefix = v.to_string();
        self
    }

    pub fn override_tool_guideline(mut self, tool: &str, guideline: ToolGuideline) -> Self {
        self.tool_guidelines.insert(tool.to_string(), guideline);
        self
    }
}

// ---------------------------------------------------------------------------
// File loading helpers
// ---------------------------------------------------------------------------

fn load_tool_guidelines() -> HashMap<String, ToolGuideline> {
    let entries: &[(&str, &str)] = &[
        ("bash", include_str!("../../prompts/tool_guidelines/bash.md")),
        ("read", include_str!("../../prompts/tool_guidelines/read.md")),
        ("write", include_str!("../../prompts/tool_guidelines/write.md")),
        ("edit", include_str!("../../prompts/tool_guidelines/edit.md")),
        ("grep", include_str!("../../prompts/tool_guidelines/grep.md")),
        ("find", include_str!("../../prompts/tool_guidelines/find.md")),
        ("ls", include_str!("../../prompts/tool_guidelines/ls.md")),
    ];

    let mut map = HashMap::new();
    for (name, content) in entries {
        map.insert(name.to_string(), parse_tool_guideline(content));
    }
    map
}

/// Parse a tool guideline file. Format:
/// ```text
/// snippet: <one-line description>
/// guidelines:
/// - <guideline 1>
/// - <guideline 2>
/// ```
fn parse_tool_guideline(content: &str) -> ToolGuideline {
    let mut snippet = String::new();
    let mut guidelines = Vec::new();
    let mut in_guidelines = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(s) = trimmed.strip_prefix("snippet: ") {
            snippet = s.trim().to_string();
        } else if trimmed.starts_with("guidelines:") {
            in_guidelines = true;
        } else if in_guidelines {
            if let Some(g) = trimmed.strip_prefix("- ") {
                guidelines.push(g.trim().to_string());
            } else if !trimmed.is_empty() {
                // Non-list line ends the guidelines section
                in_guidelines = false;
            }
        }
    }

    ToolGuideline { snippet, guidelines }
}

// ---------------------------------------------------------------------------
// System prompt builder
// ---------------------------------------------------------------------------

/// Build the full system prompt from a `PromptSet`, available tools, context
/// files, extra guidelines, and the working directory.
pub fn build_system_prompt(
    prompt_set: &PromptSet,
    tool_names: &[String],
    context_files: &[(String, String)],
    extra_guidelines: &[String],
    cwd: &str,
) -> String {
    // -- tools list ---------------------------------------------------------
    let tools_list: Vec<String> = tool_names
        .iter()
        .filter_map(|name| {
            let g = prompt_set.tool_guidelines.get(name)?;
            Some(format!("- {}: {}", name, g.snippet))
        })
        .collect();

    // -- guidelines ---------------------------------------------------------
    let mut guidelines: Vec<String> = vec![
        "Be concise in your responses".to_string(),
        "Show file paths clearly when working with files".to_string(),
    ];
    for g in extra_guidelines {
        let trimmed = g.trim().to_string();
        if !trimmed.is_empty() {
            guidelines.push(trimmed);
        }
    }
    // Dynamic guidelines based on which tools are available
    if tool_names.iter().any(|n| n == "bash") {
        guidelines.push("Use bash for file operations like ls, rg, find".to_string());
    }

    // -- project context ----------------------------------------------------
    let project_context: String = if context_files.is_empty() {
        String::new()
    } else {
        context_files
            .iter()
            .map(|(path, content)| {
                format!(
                    "<project_instructions path=\"{}\">\n{}\n</project_instructions>",
                    path, content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    // -- template substitution ----------------------------------------------
    let guidelines_str = guidelines
        .iter()
        .map(|g| format!("- {}", g))
        .collect::<Vec<_>>()
        .join("\n");

    prompt_set
        .system_main
        .replace("{tools_list}", &tools_list.join("\n"))
        .replace("{guidelines}", &guidelines_str)
        .replace("{project_context}", &project_context)
        .replace("{cwd}", cwd)
}
