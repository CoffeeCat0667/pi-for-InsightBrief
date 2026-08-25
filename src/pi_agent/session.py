"""Session class wrapping Rust agent with event buffer and output filtering."""

from __future__ import annotations

import asyncio
import json
from typing import Any, AsyncIterator, Generator

from .event_buffer import EventBuffer
from .types import OutputMode


class Session:
    """一个独立的对话会话。

    每个 Session 拥有自己的：
    - 会话历史（由 Rust SessionStore 管理）
    - 事件缓冲区（EventBuffer）
    - 输出模式（OutputMode）

    支持同步和异步两种使用方式：
    - 同步：run() 阻塞直到完成，然后通过 next_event() / wait_response() 读取
    - 异步：run_async() 不阻塞事件循环，通过 events() 异步迭代实时读取事件
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

    @property
    def is_running(self) -> bool:
        """当前是否有任务正在运行。"""
        return self._native_agent.is_running()

    # ------------------------------------------------------------------
    # Synchronous API
    # ------------------------------------------------------------------

    def run(self, prompt: str) -> None:
        """运行一轮对话（同步，阻塞直到完成）。"""
        if self._closed:
            raise RuntimeError(f"Session {self._session_id} is closed")

        try:
            self._native_agent.run(prompt)
        except Exception:
            self._drain_native_events()
            raise

        # 同步等待 Rust 任务完成
        self._wait_native_done()

        if self._session_data is not None:
            self._session_data[self._session_id] = json.loads(
                self._native_agent.export_session_data()
            )

        self._drain_native_events()

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
        """等待并返回完整回复文本（同步）。"""
        import time
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
                    if not parts:
                        parts.append(content)
                    break
            elif event.event_type == "agent_end":
                content = event.content or ""
                if content and not parts:
                    parts.append(content)
                break
        return "".join(parts)

    def wait_response_stream(self, timeout: float = 300.0) -> Generator[str, None, str]:
        """流式等待回复（同步），yield 每个 token，最终 return 完整回复。"""
        import time
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
                    if not parts:
                        parts.append(content)
                    break
            elif event.event_type == "agent_end":
                content = event.content or ""
                if content and not parts:
                    parts.append(content)
                break
        return "".join(parts)

    # ------------------------------------------------------------------
    # Async API
    # ------------------------------------------------------------------

    async def run_async(self, prompt: str) -> None:
        """运行一轮对话（异步，不阻塞事件循环）。

        Rust Agent loop 在 Tokio 运行时上执行，事件通过 broadcast
        channel 实时发送。调用 events() 可异步迭代实时读取事件。
        """
        if self._closed:
            raise RuntimeError(f"Session {self._session_id} is closed")

        # Run native agent in a thread to release the event loop.
        # The Rust run() releases the GIL, so Python can call next_event()
        # from another coroutine concurrently.
        await asyncio.to_thread(self._native_agent.run, prompt)

        # Sync session data back
        if self._session_data is not None:
            data = await asyncio.to_thread(self._native_agent.export_session_data)
            self._session_data[self._session_id] = json.loads(data)

    async def events(self, timeout: float = 300.0) -> AsyncIterator[Any]:
        """异步迭代器，实时 yield 符合 output_mode 的事件。

        在 run_async() 调用后使用，可实时接收 stream_token、tool_call_start
        等事件。当收到 agent_end 或超时后结束迭代。

        用法::

            await session.run_async("你好")
            async for event in session.events():
                if event.event_type == "stream_token":
                    print(event.content, end="")
        """
        import time
        start = time.time()
        while time.time() - start < timeout:
            event = self.next_event()
            if event is not None:
                yield event
                if event.event_type == "agent_end":
                    return
                continue

            # Check if native agent is still running
            if not self._native_agent.is_running():
                # Drain remaining events
                self._drain_native_events()
                while True:
                    event = self._buffer.get()
                    if event is None:
                        return
                    if self._buffer.should_output(event):
                        yield event
                        if event.event_type == "agent_end":
                            return
                return

            # No events yet, yield control to the event loop
            await asyncio.sleep(0.01)

    async def stream(
        self, prompt: str, timeout: float = 300.0
    ) -> AsyncIterator[Any]:
        """启动对话并实时流式返回事件（合并 run_async + events）。

        一行代码即可获得实时事件流，无需手动管理 run_async 和 events()。

        用法::

            async for event in session.stream("你好"):
                if event.event_type == "stream_token":
                    print(event.content, end="")
        """
        run_task = asyncio.ensure_future(self.run_async(prompt))

        try:
            async for event in self.events(timeout=timeout):
                yield event
        finally:
            # Ensure the wrapper task is observed so failures are propagated.
            await run_task

    async def wait_response_async(self, timeout: float = 300.0) -> str:
        """等待并返回完整回复文本（异步）。"""
        parts: list[str] = []
        async for event in self.events(timeout=timeout):
            if event.event_type == "stream_token":
                parts.append(event.content or "")
            elif event.event_type == "message_end":
                content = event.content or ""
                if content:
                    if not parts:
                        parts.append(content)
                    break
            elif event.event_type == "agent_end":
                content = event.content or ""
                if content and not parts:
                    parts.append(content)
                break
        return "".join(parts)

    def cancel(self) -> None:
        """取消当前正在运行的任务（协作式取消）。"""
        self._native_agent.cancel()

    # ------------------------------------------------------------------
    # Internal
    # ------------------------------------------------------------------

    def _drain_native_events(self) -> None:
        """从 Rust agent 的事件通道中取出所有事件并放入缓冲区。"""
        while True:
            event = self._native_agent.next_event()
            if event is None:
                break
            self._buffer.put(event)

    def _wait_native_done(self, timeout: float = 300.0) -> None:
        """等待 Rust 任务完成（同步轮询）。"""
        import time
        start = time.time()
        while self._native_agent.is_running() and time.time() - start < timeout:
            self._drain_native_events()
            time.sleep(0.01)
        self._drain_native_events()

    def close(self) -> None:
        """关闭会话。"""
        self._closed = True
        self._buffer.close()

    def __repr__(self) -> str:
        return f"Session(id={self._session_id}, mode={self._buffer.output_mode.value})"
