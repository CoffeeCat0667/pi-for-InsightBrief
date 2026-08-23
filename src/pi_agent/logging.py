"""Logging utilities for Pi Agent."""

import sys
from typing import TextIO
from .types import LogLevel


_LEVEL_MAP = {
    LogLevel.DEBUG: 0,
    LogLevel.INFO: 1,
    LogLevel.WARNING: 2,
    LogLevel.ERROR: 3,
}


class Logger:
    """分级日志器，输出到缓冲区而非控制台。"""

    def __init__(self, level: LogLevel = LogLevel.INFO, buffer: TextIO | None = None):
        self._level = level
        self._buffer = buffer or TextIO()
        self._logs: list[str] = []

    @property
    def level(self) -> LogLevel:
        return self._level

    @level.setter
    def level(self, value: LogLevel) -> None:
        self._level = value

    def _should_log(self, level: LogLevel) -> bool:
        return _LEVEL_MAP[level] >= _LEVEL_MAP[self._level]

    def _format(self, level: LogLevel, tag: str, message: str) -> str:
        import datetime
        ts = datetime.datetime.now().strftime("%H:%M:%S.%f")[:-3]
        return f"[{ts}] [{level.value.upper():>7}] [{tag}] {message}"

    def debug(self, tag: str, message: str) -> None:
        if self._should_log(LogLevel.DEBUG):
            line = self._format(LogLevel.DEBUG, tag, message)
            self._logs.append(line)
            self._buffer.write(line + "\n")

    def info(self, tag: str, message: str) -> None:
        if self._should_log(LogLevel.INFO):
            line = self._format(LogLevel.INFO, tag, message)
            self._logs.append(line)
            self._buffer.write(line + "\n")

    def warning(self, tag: str, message: str) -> None:
        if self._should_log(LogLevel.WARNING):
            line = self._format(LogLevel.WARNING, tag, message)
            self._logs.append(line)
            self._buffer.write(line + "\n")

    def error(self, tag: str, message: str) -> None:
        if self._should_log(LogLevel.ERROR):
            line = self._format(LogLevel.ERROR, tag, message)
            self._logs.append(line)
            self._buffer.write(line + "\n")

    def get_logs(self, level: LogLevel | None = None) -> list[str]:
        """获取日志记录。"""
        if level is None:
            return list(self._logs)
        min_level = _LEVEL_MAP[level]
        return [l for l in self._logs if _level_from_log(l) >= min_level]

    def clear(self) -> None:
        self._logs.clear()


def _level_from_log(line: str) -> int:
    """从日志行中提取级别。"""
    for name, val in _LEVEL_MAP.items():
        if name.value.upper() in line:
            return val
    return 0
