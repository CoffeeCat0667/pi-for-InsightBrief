use crate::session::{Entry, EntryId};

/// Configuration for context compaction.
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Maximum context window size in tokens.
    pub context_window: u32,
    /// Number of tokens to reserve for the response.
    pub reserve_tokens: u32,
    /// Minimum number of recent tokens to keep.
    pub keep_recent_tokens: u32,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            context_window: 128000,
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
        }
    }
}

/// Result of compaction analysis.
#[derive(Debug, Clone)]
pub struct CompactionAnalysis {
    /// Whether compaction is needed.
    pub needs_compaction: bool,
    /// Total tokens in the current context.
    pub total_tokens: u32,
    /// Number of tokens that would be kept after compaction.
    pub tokens_to_keep: u32,
    /// Number of tokens that would be compacted.
    pub tokens_to_compact: u32,
    /// The entry ID where compaction should start (the cut point).
    pub cut_point: Option<EntryId>,
    /// Entries that would be kept (not compacted).
    pub kept_entries: Vec<EntryId>,
    /// Entries that would be compacted.
    pub compacted_entries: Vec<EntryId>,
}

/// Analyze whether compaction is needed and find the cut point.
pub fn analyze_compaction(
    entries: &[&Entry],
    config: &CompactionConfig,
) -> CompactionAnalysis {
    // Calculate total tokens
    let total_tokens: u32 = entries.iter().map(|e| e.usage.total()).sum();

    // Calculate available tokens
    let available = config.context_window.saturating_sub(config.reserve_tokens);

    // If total tokens fit within available, no compaction needed
    if total_tokens <= available {
        return CompactionAnalysis {
            needs_compaction: false,
            total_tokens,
            tokens_to_keep: total_tokens,
            tokens_to_compact: 0,
            cut_point: None,
            kept_entries: entries.iter().map(|e| e.id).collect(),
            compacted_entries: Vec::new(),
        };
    }

    // Find the cut point: accumulate from the most recent entry backward
    // until we exceed the keep_recent_tokens limit
    let mut tokens_accumulated = 0u32;
    let mut cut_point_index = None;

    for (i, entry) in entries.iter().enumerate().rev() {
        tokens_accumulated += entry.usage.total();
        if tokens_accumulated >= config.keep_recent_tokens {
            // Found the cut point - this entry and everything after it stays
            cut_point_index = Some(i);
            break;
        }
    }

    // If no cut point found, keep all entries (can't compact further)
    let cut_point_index = match cut_point_index {
        Some(idx) => idx,
        None => {
            return CompactionAnalysis {
                needs_compaction: false,
                total_tokens,
                tokens_to_keep: total_tokens,
                tokens_to_compact: 0,
                cut_point: None,
                kept_entries: entries.iter().map(|e| e.id).collect(),
                compacted_entries: Vec::new(),
            };
        }
    };

    // Split entries at cut point
    let kept_entries: Vec<EntryId> = entries[cut_point_index..]
        .iter()
        .map(|e| e.id)
        .collect();
    let compacted_entries: Vec<EntryId> = entries[..cut_point_index]
        .iter()
        .map(|e| e.id)
        .collect();

    let tokens_to_keep: u32 = kept_entries
        .iter()
        .filter_map(|id| entries.iter().find(|e| e.id == *id))
        .map(|e| e.usage.total())
        .sum();
    let tokens_to_compact = total_tokens - tokens_to_keep;

    CompactionAnalysis {
        needs_compaction: true,
        total_tokens,
        tokens_to_keep,
        tokens_to_compact,
        cut_point: entries.get(cut_point_index).map(|e| e.id),
        kept_entries,
        compacted_entries,
    }
}

/// Estimate token count from text (simple heuristic: chars / 4).
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() as u32) / 4
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Usage;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_entry(content: &str, tokens: u32) -> Entry {
        Entry {
            entry_type: "entry".to_string(),
            id: Uuid::new_v4(),
            parent_id: None,
            created_at: Utc::now(),
            message: crate::session::Message {
                role: crate::session::Role::User,
                content: content.to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            usage: Usage {
                input_tokens: tokens,
                output_tokens: 0,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
            },
            is_active: true,
        }
    }

    #[test]
    fn test_no_compaction_needed() {
        let entries = vec![
            make_entry("Hello", 100),
            make_entry("World", 100),
        ];
        let entries_refs: Vec<&Entry> = entries.iter().collect();
        let config = CompactionConfig {
            context_window: 128000,
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
        };

        let analysis = analyze_compaction(&entries_refs, &config);
        assert!(!analysis.needs_compaction);
        assert_eq!(analysis.total_tokens, 200);
    }

    #[test]
    fn test_compaction_needed() {
        // Create entries totaling 100,000 tokens
        let entries: Vec<Entry> = (0..100)
            .map(|i| make_entry(&format!("Message {}", i), 1000))
            .collect();
        let entries_refs: Vec<&Entry> = entries.iter().collect();
        let config = CompactionConfig {
            context_window: 50000, // Smaller context window to trigger compaction
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
        };

        let analysis = analyze_compaction(&entries_refs, &config);
        assert!(analysis.needs_compaction);
        assert_eq!(analysis.total_tokens, 100000);
        assert!(analysis.cut_point.is_some());
        assert!(!analysis.kept_entries.is_empty());
        assert!(!analysis.compacted_entries.is_empty());
    }
}
