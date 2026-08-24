"""Event buffer with output mode filtering and async iteration support."""

import asyncio
import threading
from collections import deque
from typing import Any, AsyncIterator

from .types import OutputMode


class EventBuffer:
    """线程安全的事件缓冲区，根据 OutputMode 过滤输出事件。

    支持同步 get() 和异步 async_get() 两种读取方式。
    """

    def __init__(self, output_mode: OutputMode = OutputMode.CONTENT_ONLY):
        self._mode = output_mode
        self._buffer: deque = deque()
        self._lock = threading.Lock()
        self._closed = False

    @property
    def output_mode(self) -> OutputMode:
        return self._mode

    @output_mode.setter
    def output_mode(self, value: OutputMode) -> None:
        self._mode = value

    def put(self, event: Any) -> None:
        """向缓冲区推入一个事件。"""
        with self._lock:
            if self._closed:
                return
            self._buffer.append(event)

    def get(self) -> Any | None:
        """从缓冲区取出一个事件，若为空返回 None。"""
        with self._lock:
            if not self._buffer:
                return None
            return self._buffer.popleft()

    async def async_get(self) -> Any | None:
        """异步从缓冲区取出一个事件，若为空返回 None。

        使用 asyncio.to_thread 避免阻塞事件循环。
        """
        return await asyncio.to_thread(self.get)

    def peek(self) -> Any | None:
        """查看缓冲区头部事件但不取出。"""
        with self._lock:
            if not self._buffer:
                return None
            return self._buffer[0]

    def close(self) -> None:
        """关闭缓冲区。"""
        with self._lock:
            self._closed = True

    def clear(self) -> None:
        with self._lock:
            self._buffer.clear()

    def __len__(self) -> int:
        with self._lock:
            return len(self._buffer)

    def __bool__(self) -> bool:
        return len(self) > 0

    def should_output(self, event: Any) -> bool:
        """根据 output_mode 判断事件是否应该被输出。"""
        event_type = getattr(event, "event_type", None)
        if event_type is None:
            return True

        if self._mode == OutputMode.FULL_DEBUG:
            return True

        # Always surface error-level debug events so users see failure reasons
        if event_type == "debug" and getattr(event, "level", None) == "error":
            return True

        if self._mode == OutputMode.THINKING:
            return event_type in (
                "stream_token",
                "message_end",
                "tool_call_start",
                "tool_call_end",
                "compaction_start",
                "compaction_end",
                "turn_start",
                "turn_end",
            )

        # CONTENT_ONLY
        return event_type in ("stream_token", "message_end")

    def drain_filtered(self) -> list[Any]:
        """取出所有事件，但只返回符合 output_mode 的事件。"""
        result = []
        while True:
            event = self.get()
            if event is None:
                break
            if self.should_output(event):
                result.append(event)
        return result

    def drain_all(self) -> list[Any]:
        """取出所有事件，不过滤。"""
        result = []
        while True:
            event = self.get()
            if event is None:
                break
            result.append(event)
        return result
