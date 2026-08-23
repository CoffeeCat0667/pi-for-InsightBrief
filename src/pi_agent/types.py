"""Type definitions and enums for Pi Agent."""

from enum import Enum


class LogLevel(str, Enum):
    """日志级别。"""
    DEBUG = "debug"
    INFO = "info"
    WARNING = "warning"
    ERROR = "error"


class OutputMode(str, Enum):
    """输出模式，控制事件缓冲区输出哪些内容。"""
    # 仅输出对话内容（stream_token / message_end）
    CONTENT_ONLY = "content"
    # 输出思考内容（tool_call / compaction / turn） + 对话内容
    THINKING = "thinking"
    # 输出所有内容（提示词 + 思考 + 对话）
    FULL_DEBUG = "full_debug"
