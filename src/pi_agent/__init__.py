"""Pi Agent - 编程代理，支持多会话、流式输出、工具调用和上下文压缩。

架构：
    - 单个 Agent 实例管理多个 Session
    - 每个 Session 拥有独立的会话历史和事件缓冲区
    - 事件缓冲区根据 OutputMode 过滤输出
    - 所有提示词可通过外部 .md 文件加载

快速开始：
    from pi_agent import create_agent, OutputMode

    agent = create_agent(
        api_key="sk-xxx",
        model="gpt-4",
    )

    session = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)
    agent.run(session.session_id, "hello")
    print(session.wait_response())
"""

from importlib import import_module as _im

# 加载 Rust 原生扩展
_pi_agent = _im(".pi_agent", __name__)

# 导入新 API 层
from .types import LogLevel, OutputMode
from .logging import Logger
from .prompts import PromptSet, load_prompt_set, load_prompt
from .event_buffer import EventBuffer
from .session import Session
from .agent import Agent, create_agent, get_agent

# Rust 原生类型（向后兼容）
PyAgent = _pi_agent.PyAgent
PySession = _pi_agent.PySession
PyAgentEvent = _pi_agent.PyAgentEvent
PyEntry = _pi_agent.PyEntry
PyUsage = _pi_agent.PyUsage
PyBranchPoint = _pi_agent.PyBranchPoint
PyBranchSummary = _pi_agent.PyBranchSummary
PyTool = _pi_agent.PyTool
PyToolDefinition = _pi_agent.PyToolDefinition

# 导出函数
version = _pi_agent.version
create_entry_id = _pi_agent.create_entry_id

__version__ = version()

__all__ = [
    # 新 API
    "Agent",
    "create_agent",
    "get_agent",
    "Session",
    "LogLevel",
    "OutputMode",
    "Logger",
    "PromptSet",
    "load_prompt_set",
    "load_prompt",
    "EventBuffer",
    # Rust 原生类型（向后兼容）
    "PyAgent",
    "PySession",
    "PyAgentEvent",
    "PyEntry",
    "PyUsage",
    "PyBranchPoint",
    "PyBranchSummary",
    "PyTool",
    "PyToolDefinition",
    "version",
    "create_entry_id",
]
