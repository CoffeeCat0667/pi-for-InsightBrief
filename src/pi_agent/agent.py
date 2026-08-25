"""Agent singleton managing multiple sessions."""

from __future__ import annotations

import asyncio
import json
import threading
import uuid
from pathlib import Path
from typing import Any, AsyncIterator

from .logging import Logger
from .prompts import PromptSet, load_prompt_set
from .session import Session
from .types import LogLevel, OutputMode


_agent_lock = threading.Lock()
_agent_instance: Agent | None = None


class Agent:
    """全局单例 Agent。

    管理 LLM 连接、工具注册和多个 Session。
    使用者通过 create_session() 创建会话，而非直接构造 Agent。

    所有方法均为异步兼容设计。使用 run_async() / wait_response_async()
    获得真正的异步体验；同步 run() 仍在内部阻塞当前线程。
    """

    def __init__(
        self,
        *,
        api_key: str,
        model: str,
        base_url: str | None = None,
        log_level: LogLevel = LogLevel.INFO,
        log_buffer: Any = None,
        max_turns: int = 50,
        max_retries: int = 10,
        reserve_tokens: int = 16384,
        keep_recent_tokens: int = 20000,
        context_window: int = 256000,
        system_main: str | Path | None = None,
        compaction_system: str | Path | None = None,
        compaction_initial: str | Path | None = None,
        compaction_update: str | Path | None = None,
        compaction_turn_prefix: str | Path | None = None,
        tool_guidelines_dir: str | Path | None = None,
        extra_prompts: dict[str, str | Path] | None = None,
        extra_guidelines: list[str] | None = None,
        sessions: dict[str, dict[str, Any]] | None = None,
    ):
        self._api_key = api_key
        self._model = model
        self._base_url = base_url
        self._max_turns = max_turns
        self._max_retries = max_retries
        self._reserve_tokens = reserve_tokens
        self._keep_recent_tokens = keep_recent_tokens
        self._context_window = context_window
        self._extra_guidelines = extra_guidelines or []
        self._session_data = sessions if sessions is not None else {}

        # 日志
        self._log = Logger(level=log_level, buffer=log_buffer)

        # 加载提示词
        self._prompt_set = load_prompt_set(
            system_main=system_main,
            compaction_system=compaction_system,
            compaction_initial=compaction_initial,
            compaction_update=compaction_update,
            compaction_turn_prefix=compaction_turn_prefix,
            tool_guidelines_dir=tool_guidelines_dir,
            extra=extra_prompts,
        )

        # Session 管理
        self._sessions: dict[str, Session] = {}
        self._lock = threading.Lock()

        self._log.info("Agent", f"Initialized model={model} base_url={base_url or 'default'}")

    @property
    def model(self) -> str:
        return self._model

    @property
    def log_level(self) -> LogLevel:
        return self._log.level

    @log_level.setter
    def log_level(self, value: LogLevel) -> None:
        self._log.level = value

    def create_session(
        self,
        *,
        session_id: str | None = None,
        output_mode: OutputMode = OutputMode.CONTENT_ONLY,
        cwd: str | None = None,
    ) -> Session:
        """创建新会话，返回 Session 对象。"""
        session_id = session_id or str(uuid.uuid4())
        self._session_data.setdefault(session_id, {})

        self._log.info("Agent", f"Creating session {session_id}")

        # 创建 Rust native agent
        native_agent = self._create_native_agent(session_id, cwd)

        self._session_data[session_id] = json.loads(native_agent.export_session_data())

        session = Session(
            session_id=session_id,
            native_agent=native_agent,
            output_mode=output_mode,
            session_data=self._session_data,
        )

        with self._lock:
            self._sessions[session_id] = session

        return session

    def get_session(self, session_id: str) -> Session | None:
        """根据 ID 获取已有会话。"""
        with self._lock:
            return self._sessions.get(session_id)

    def continue_session(
        self,
        session_id: str,
        *,
        output_mode: OutputMode | None = None,
    ) -> Session | None:
        """继续已有会话。"""
        session = self.get_session(session_id)
        if session is None:
            self._log.warning("Agent", f"Session {session_id} not found")
            return None

        if output_mode is not None:
            session.output_mode = output_mode

        self._log.info("Agent", f"Continuing session {session_id}")
        return session

    def list_sessions(self) -> list[str]:
        """列出所有会话 ID。"""
        with self._lock:
            return list(self._sessions.keys())

    def delete_session(self, session_id: str) -> bool:
        """删除会话。"""
        with self._lock:
            session = self._sessions.pop(session_id, None)
        if session is not None:
            session.close()
            self._log.info("Agent", f"Deleted session {session_id}")
            return True
        return False

    def run(self, session_id: str, prompt: str) -> None:
        """对指定会话运行一轮对话（同步，阻塞直到完成）。"""
        session = self.get_session(session_id)
        if session is None:
            raise ValueError(f"Session {session_id} not found")
        session.run(prompt)

    async def run_async(self, session_id: str, prompt: str) -> None:
        """对指定会话运行一轮对话（异步，不阻塞事件循环）。

        底层在 Tokio 运行时上执行 Rust Agent loop，事件通过
        broadcast channel 实时发送，可通过 session.events() 异步迭代。
        """
        session = self.get_session(session_id)
        if session is None:
            raise ValueError(f"Session {session_id} not found")
        await session.run_async(prompt)

    async def stream(
        self, session_id: str, prompt: str, timeout: float = 300.0
    ) -> AsyncIterator[Any]:
        """对指定会话启动对话并实时流式返回事件。

        合并 run_async + events()，一行代码获得实时事件流。

        用法::

            async for event in agent.stream(session_id, "你好"):
                if event.event_type == "stream_token":
                    print(event.content, end="")
        """
        session = self.get_session(session_id)
        if session is None:
            raise ValueError(f"Session {session_id} not found")
        async for event in session.stream(prompt, timeout=timeout):
            yield event

    def next_event(self, session_id: str) -> Any | None:
        """获取指定会话的下一个事件。"""
        session = self.get_session(session_id)
        if session is None:
            return None
        return session.next_event()

    def wait_response(self, session_id: str, timeout: float = 300.0) -> str:
        """等待指定会话的完整回复（同步）。"""
        session = self.get_session(session_id)
        if session is None:
            raise ValueError(f"Session {session_id} not found")
        return session.wait_response(timeout=timeout)

    async def wait_response_async(self, session_id: str, timeout: float = 300.0) -> str:
        """等待指定会话的完整回复（异步）。"""
        session = self.get_session(session_id)
        if session is None:
            raise ValueError(f"Session {session_id} not found")
        return await session.wait_response_async(timeout=timeout)

    def register_builtin_tools(self) -> None:
        """注册内置工具到所有后续创建的 session。"""
        self._log.info("Agent", "Builtin tools registered (ls, read, write, edit, grep, find, bash)")

    def _create_native_agent(self, session_id: str, cwd: str | None) -> Any:
        """创建底层 Rust agent 实例。"""
        from importlib import import_module
        _native = import_module("pi_agent.pi_agent")
        PyAgent = _native.PyAgent

        kwargs: dict[str, Any] = {
            "api_key": self._api_key,
            "model": self._model,
            "session_id": session_id,
            "session_data": self._session_data,
            "base_url": self._base_url,
            "max_turns": self._max_turns,
            "max_retries": self._max_retries,
            "reserve_tokens": self._reserve_tokens,
            "keep_recent_tokens": self._keep_recent_tokens,
            "context_window": self._context_window,
            "extra_guidelines": self._extra_guidelines,
        }

        if self._prompt_set.system_main:
            kwargs["system_prompt"] = self._prompt_set.system_main

        if cwd:
            kwargs["cwd"] = cwd

        native = PyAgent(**kwargs)
        native.register_builtin_tools()
        return native

    def get_log_buffer(self) -> list[str]:
        """获取日志记录。"""
        return self._log.get_logs()

    def clear_logs(self) -> None:
        """清空日志。"""
        self._log.clear()


def get_agent() -> Agent | None:
    """获取全局 Agent 实例。"""
    return _agent_instance


def create_agent(**kwargs: Any) -> Agent:
    """创建或获取全局 Agent 单例。

    首次调用创建 Agent 并存储为全局单例。
    后续调用更新配置并返回同一实例。
    """
    global _agent_instance
    with _agent_lock:
        if _agent_instance is None:
            _agent_instance = Agent(**kwargs)
        else:
            # 更新配置
            _agent_instance._api_key = kwargs.get("api_key", _agent_instance._api_key)
            _agent_instance._model = kwargs.get("model", _agent_instance._model)
            _agent_instance._base_url = kwargs.get("base_url", _agent_instance._base_url)
            if "max_retries" in kwargs:
                _agent_instance._max_retries = kwargs["max_retries"]
            if "sessions" in kwargs and kwargs["sessions"] is not None:
                _agent_instance._session_data = kwargs["sessions"]
            if "log_level" in kwargs:
                _agent_instance.log_level = kwargs["log_level"]
            if "system_main" in kwargs or "tool_guidelines_dir" in kwargs:
                _agent_instance._prompt_set = load_prompt_set(
                    system_main=kwargs.get("system_main"),
                    compaction_system=kwargs.get("compaction_system"),
                    compaction_initial=kwargs.get("compaction_initial"),
                    compaction_update=kwargs.get("compaction_update"),
                    compaction_turn_prefix=kwargs.get("compaction_turn_prefix"),
                    tool_guidelines_dir=kwargs.get("tool_guidelines_dir"),
                    extra=kwargs.get("extra_prompts"),
                )
        return _agent_instance
