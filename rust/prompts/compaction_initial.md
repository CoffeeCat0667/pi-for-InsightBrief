The messages above are a conversation to summarize. Create a structured context checkpoint summary that another LLM will use to continue the work.

Use this EXACT format:

## Goal
[What is the user trying to accomplish? Can be multiple items if the session covers different tasks.]

## Constraints & Preferences
- [Any constraints, preferences, or requirements mentioned by user]

## Progress
- [x] [Completed tasks/changes]
- [ ] [Current work - update based on progress]

## Blockers
- [Issues preventing progress, if any]

## Decisions
- **[Decision]**: [Brief rationale]

## Next Steps
1. [Ordered list of what should happen next]

## Critical Context
- [Any data, examples, or references needed to continue]
- [Or "(none)" if not applicable]

## Files
Read: [list of read-only files touched]
Modified: [list of files that were written or edited]

Keep each section concise. Preserve exact file paths, function names, and error messages.