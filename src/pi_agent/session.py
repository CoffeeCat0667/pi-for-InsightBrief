"""Session class wrapping Rust agent with event buffer and output filtering."""

from __future__ import annotations

import os
import time
import json
from typing import Any, Generator

from .event_buffer import EventBuffer
from .types import OutputMode


class Session:
    """一个独立的对话会话。

    每个 Session 拥有自己的：
    - 会话历史（由 Rust SessionStore 管理）
    - 事件缓冲区（EventBuffer）
    - 输出模式（OutputMode）
    """

    def __init__(
        self,
        session_id: str,
        native_agent: Any,
        output_mode: OutputMode = OutputMode.CONTENT_ONLY,
        session_data: dict[str, dict[str, Any]] | None = None,
    ):
        self._session_id = session_id
        self._native_agent = native_agent
        self._buffer = EventBuffer(output_mode)
        self._session_data = session_data
        self._closed = False

    @property
    def session_id(self) -> str:
        return self._session_id

    @property
    def output_mode(self) -> OutputMode:
        return self._buffer.output_mode

    @output_mode.setter
    def output_mode(self, value: OutputMode) -> None:
        self._buffer.output_mode = value

    def run(self, prompt: str) -> None:
        """运行一轮对话。"""
        if self._closed:
            raise RuntimeError(f"Session {self._session_id} is closed")

        try:
            self._native_agent.run(prompt)
        except Exception:
            # Preserve diagnostics emitted before a native request failed.
            self._drain_native_events()
            raise

        if self._session_data is not None:
            self._session_data[self._session_id] = json.loads(
                self._native_agent.export_session_data()
            )

        # 将 Rust agent 的事件倒入缓冲区
        self._drain_native_events()

    def _drain_native_events(self) -> None:
        """从 Rust agent 的事件通道中取出所有事件并放入缓冲区。"""
        while True:
            event = self._native_agent.next_event()
            if event is None:
                break
            self._buffer.put(event)

    def next_event(self) -> Any | None:
        """获取下一个符合 output_mode 的事件。"""
        if self._closed:
            return None

        # 先尝试从缓冲区取
        while True:
            event = self._buffer.get()
            if event is None:
                break
            if self._buffer.should_output(event):
                return event

        # 缓冲区为空，尝试从 native agent 获取更多事件
        self._drain_native_events()

        while True:
            event = self._buffer.get()
            if event is None:
                return None
            if self._buffer.should_output(event):
                return event

    def next_event_raw(self) -> Any | None:
        """获取下一个原始事件（不过滤）。"""
        if self._closed:
            return None

        event = self._buffer.get()
        if event is not None:
            return event

        self._drain_native_events()
        return self._buffer.get()

    def wait_response(self, timeout: float = 300.0) -> str:
        """等待并返回完整回复文本。

        跳过工具调用轮的空 message_end，只在收到有实际内容的回复时返回。
        message_end 的 content 不会重复拼接（StreamToken 已累积完整内容）。
        """
        parts: list[str] = []
        start = time.time()
        while time.time() - start < timeout:
            event = self.next_event()
            if event is None:
                time.sleep(0.01)
                continue
            if event.event_type == "stream_token":
                parts.append(event.content or "")
            elif event.event_type == "message_end":
                content = event.content or ""
                if content:
                    # message_end.content 与 StreamToken 累积内容相同，不重复拼接
                    if not parts:
                        parts.append(content)
                    break
                # 空 content = 工具调用轮次，继续等下一轮
            elif event.event_type == "agent_end":
                content = event.content or ""
                if content and not parts:
                    parts.append(content)
                break
        return "".join(parts)

    def wait_response_stream(self, timeout: float = 300.0) -> Generator[str, None, str]:
        """流式等待回复，yield 每个 token，最终 return 完整回复。

        跳过工具调用轮的空 message_end，只在收到有实际内容的回复时终止。
        message_end 的 content 不会重复拼接（StreamToken 已累积完整内容）。
        """
        parts: list[str] = []
        start = time.time()
        while time.time() - start < timeout:
            event = self.next_event()
            if event is None:
                time.sleep(0.01)
                continue
            if event.event_type == "stream_token":
                token = event.content or ""
                parts.append(token)
                yield token
            elif event.event_type == "message_end":
                content = event.content or ""
                if content:
                    # message_end.content 与 StreamToken 累积内容相同，不重复拼接
                    if not parts:
                        parts.append(content)
                    break
                # 空 content = 工具调用轮次，继续等下一轮
            elif event.event_type == "agent_end":
                content = event.content or ""
                if content and not parts:
                    parts.append(content)
                break
        return "".join(parts)

    def close(self) -> None:
        """关闭会话。"""
        self._closed = True
        self._buffer.close()

    def __repr__(self) -> str:
        return f"Session(id={self._session_id}, mode={self._buffer.output_mode.value})"
