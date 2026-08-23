"""
Pi Agent - Complete Usage Guide
================================

A coding agent with session management and context compaction.
Built with Rust + PyO3 for performance.

Quick Start:
    1. Install: maturin develop
    2. Set API key: set OPENAI_API_KEY=sk-...
    3. Run: python my_agent.py
"""

from pi_agent import (
    PyAgent as Agent,
    PySession as Session,
    PyAgentEvent as AgentEvent,
    PyEntry as Entry,
    PyUsage as Usage,
    PyBranchPoint as BranchPoint,
    PyBranchSummary as BranchSummary,
    PyTool as Tool,
    PyToolDefinition as ToolDefinition,
    version,
    create_entry_id,
)

__version__ = version()
__all__ = [
    "Agent",
    "Session",
    "AgentEvent",
    "Entry",
    "Usage",
    "BranchPoint",
    "BranchSummary",
    "Tool",
    "ToolDefinition",
    "version",
    "create_entry_id",
]
