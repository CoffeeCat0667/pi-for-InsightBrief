# Pi Agent API Reference

当前文档对应 `v0.1.5`。Pi Agent 提供一个 Python API 层，以及底层 Rust/PyO3 类型。

## 安装

```bash
# Windows x64 wheel，兼容 Python 3.10+
pip install pi_agent-0.1.5-cp310-abi3-win_amd64.whl

# 从源码构建
maturin develop --release
```

要求 Python `>= 3.10`。`cp310-abi3` wheel 可用于 CPython 3.10 及更高版本。

## 工作模型

```text
create_agent() -> 全局 Agent
                       |
                       +-- create_session() -> Session 1
                       +-- create_session() -> Session 2
```

- `Agent` 保存 API 配置、提示词配置和 Session 注册表。
- `Session` 保存一个独立的 Rust 会话树、事件缓冲区和输出模式。
- `Agent` 通过 `create_agent()` 获取全局单例；同一 Python 进程内重复调用不会创建第二个 Agent。
- **v0.1.5 异步架构**：Rust 持久化 Tokio 运行时，`run()` 在后台 Tokio task 上执行并释放 GIL。Python 可在 `run_async()` 期间通过 `events()` 异步迭代实时接收事件。
- 多 Session 并发：每个 Session 拥有独立的 NativeAgent 和 broadcast channel，可同时运行多个 Session 的请求。

## 快速开始

```python
import asyncio
from pi_agent import OutputMode, create_agent

agent = create_agent(
    api_key="sk-xxx",
    model="gpt-4o",
    sessions={},
)

session = agent.create_session(
    session_id="session-001",
    output_mode=OutputMode.CONTENT_ONLY,
)

# 异步用法（推荐）
async def main():
    await agent.run_async(session.session_id, "你好")
    print(session.wait_response())

asyncio.run(main())

# 同步用法（仍然支持）
agent.run(session.session_id, "你好")
print(session.wait_response())
```

异步流式读取：

```python
async def stream_example():
    await agent.run_async(session.session_id, "用一句话解释递归")
    async for event in session.events():
        if event.event_type == "stream_token":
            print(event.content, end="", flush=True)
    print()

asyncio.run(stream_example())
```

并发多 Session：

```python
async def concurrent_example():
    s1 = agent.create_session()
    s2 = agent.create_session()

    # 两个 Session 并发运行
    await asyncio.gather(
        agent.run_async(s1.session_id, "你好"),
        agent.run_async(s2.session_id, "世界"),
    )

    # 实时读取两个 Session 的事件
    async for event in s1.events():
        if event.event_type == "stream_token":
            print(f"S1: {event.content}", end="")

    async for event in s2.events():
        if event.event_type == "stream_token":
            print(f"S2: {event.content}", end="")

asyncio.run(concurrent_example())
```

## `create_agent(**kwargs) -> Agent`

首次调用时创建 Agent，之后返回同一个实例。

### 必填参数

| 参数 | 类型 | 说明 |
|---|---|---|
| `api_key` | `str` | LLM API 密钥 |
| `model` | `str` | 模型名称 |

### 可选参数

| 参数 | 默认值 | 说明 |
|---|---:|---|
| `base_url` | `None` | 自定义 OpenAI 兼容 API 地址 |
| `log_level` | `LogLevel.INFO` | 日志级别 |
| `log_buffer` | `None` | 自定义文本日志缓冲区；默认使用内部日志列表 |
| `max_turns` | `50` | 单次 prompt 的最大 Agent turn 数 |
| `reserve_tokens` | `16384` | 为模型响应预留的 token 数 |
| `keep_recent_tokens` | `20000` | 上下文压缩时保留的最近 token 数 |
| `context_window` | `128000` | 上下文窗口大小 |
| `system_main` | `None` | 外部主系统提示词文件路径 |
| `compaction_system` | `None` | 外部压缩系统提示词路径 |
| `compaction_initial` | `None` | 外部首次压缩提示词路径 |
| `compaction_update` | `None` | 外部增量压缩提示词路径 |
| `compaction_turn_prefix` | `None` | 外部 turn 前缀压缩提示词路径 |
| `tool_guidelines_dir` | `None` | 工具指导 `.md` 文件目录 |
| `extra_prompts` | `None` | `{name: path}` 形式的额外提示词文件映射 |
| `extra_guidelines` | `None` | 追加到系统提示词的行为规则列表 |
| `sessions` | `None` | 外部会话字典，格式为 `{session_id: session_tree}` |

示例：

```python
from pi_agent import LogLevel, create_agent

agent = create_agent(
    api_key="sk-xxx",
    model="gpt-4o",
    base_url="https://api.example.com/v1",
    log_level=LogLevel.DEBUG,
    max_turns=20,
    extra_guidelines=["回答使用中文", "修改文件前先读取文件内容"],
)
```

重复调用 `create_agent()` 时，当前实现只更新已有 Agent 的 API key、model、base URL、日志级别，以及在传入 `system_main` 或 `tool_guidelines_dir` 时重新加载部分提示词配置。运行参数不会自动更新已有 Session，也不建议在已有会话运行期间修改全局配置。

## `Agent`

### 属性

```python
agent.model       # str
agent.log_level   # LogLevel，可读写
```

### 方法

```python
# 同步
agent.run(session.session_id, "你的消息")
event = agent.next_event(session.session_id)
response = agent.wait_response(session.session_id, timeout=300.0)

# 异步（推荐）
await agent.run_async(session.session_id, "你的消息")
response = await agent.wait_response_async(session.session_id, timeout=300.0)

# 会话管理
session = agent.get_session(session_id)       # 不存在时返回 None
session = agent.continue_session(session_id)  # 仅继续当前进程内已注册的 Session
session_ids = agent.list_sessions()
success = agent.delete_session(session_id)

# 日志
logs = agent.get_log_buffer()
agent.clear_logs()
```

`continue_session()` 只返回当前进程内已注册的 Session。跨进程或重启后恢复时，将同一个 `sessions` 字典重新传给 `create_agent()`，再调用 `create_session(session_id=...)`。

`register_builtin_tools()` 当前只是日志兼容方法；内置工具会在每个新 Session 创建时自动注册。

## `Agent.create_session() -> Session`

```python
session = agent.create_session(
    session_id="session-001",
    output_mode=OutputMode.CONTENT_ONLY,
    cwd=None,
)
```

- `output_mode`：该 Session 的事件过滤模式。
- `cwd`：写入系统提示词中的工作目录，不会自动改变 Python 进程当前目录。
- `session_id`：外部会话 ID。省略时自动生成 UUID。

## 内存会话字典

`sessions` 是调用方拥有的普通 Python 字典。每个 value 保存完整会话树，而不是只有消息列表：

```python
sessions = {
    "session-001": {
        "header": {
            "session_id": "8c0f0b3f-3d17-4d6d-bf75-7f8fd2f8a2bb",
            "created_at": "2026-08-23T00:00:00Z",
            "model": "gpt-4o",
            "system_prompt": None,
        },
        "entries": [],
        "compactions": [],
        "roots": [],
        "leaf": None,
    }
}
```

每次 `Session.run()` 或 `Session.run_async()` 完成后，当前 Session 的完整树会同步回 `sessions[session_id]`。树中保留 entry 的父子关系、active 分支、leaf、工具调用、usage 和 compaction 信息。调用方可以直接保存、序列化或替换这个字典。

## `Session`

### 属性

```python
session.session_id   # str
session.output_mode  # OutputMode，可读写
session.is_running   # bool：当前是否有任务正在运行
```

### 同步方法

```python
session.run("你的消息")
event = session.next_event()
event = session.next_event_raw()
response = session.wait_response(timeout=300.0)

for token in session.wait_response_stream(timeout=300.0):
    print(token, end="", flush=True)

session.close()
```

### 异步方法

```python
await session.run_async("你的消息")
response = await session.wait_response_async(timeout=300.0)

async for event in session.events(timeout=300.0):
    if event.event_type == "stream_token":
        print(event.content, end="")

session.cancel()  # 协作式取消
```

行为说明：

- `run()` / `run_async()` 关闭的 Session 会抛出 `RuntimeError`。
- `run_async()` 在 Tokio 运行时上执行 Rust Agent loop，不阻塞 Python 事件循环。
- `events()` 异步迭代器在 `run_async()` 期间实时 yield 事件，直到收到 `agent_end` 或超时。
- `cancel()` 协作式取消当前运行的任务，任务会完成当前 turn 后停止。
- `next_event()` 返回符合 `output_mode` 的下一个事件；没有事件时返回 `None`。
- `next_event_raw()` 返回未过滤事件；没有事件时返回 `None`。
- `wait_response()` 会跳过工具调用轮次的空 `message_end`，直到收到有内容的消息。
- `message_end.content` 与已累积的 `stream_token` 内容相同，因此不会重复拼接。
- 超时后 `wait_response()` 和 `wait_response_stream()` 返回当前已收集的文本，不会抛出超时异常。
- `close()` 只关闭 Python Session 缓冲区；已同步到 `sessions` 的会话树仍由调用方保留。

## 枚举

### `LogLevel`

```python
from pi_agent import LogLevel

LogLevel.DEBUG
LogLevel.INFO
LogLevel.WARNING
LogLevel.ERROR
```

日志级别是最低记录级别。例如 `WARNING` 只记录 warning 和 error。

### `OutputMode`

```python
from pi_agent import OutputMode

OutputMode.CONTENT_ONLY
OutputMode.THINKING
OutputMode.FULL_DEBUG
```

| 事件类型 | CONTENT_ONLY | THINKING | FULL_DEBUG |
|---|:---:|:---:|:---:|
| `stream_token` | 是 | 是 | 是 |
| `message_end` | 是 | 是 | 是 |
| `tool_call_start` | 否 | 是 | 是 |
| `tool_call_end` | 否 | 是 | 是 |
| `compaction_start` | 否 | 是 | 是 |
| `compaction_end` | 否 | 是 | 是 |
| `turn_start` | 否 | 是 | 是 |
| `turn_end` | 否 | 是 | 是 |
| `agent_start` | 否 | 否 | 是 |
| `agent_end` | 否 | 否 | 是 |

`FULL_DEBUG` 表示返回所有已发出的 Agent 事件，不表示返回系统提示词或原始 HTTP 数据；当前实现没有提示词事件类型。

## 事件

通过 `session.next_event()` 或 `agent.next_event(session_id)` 获取：

```python
event.event_type      # str
event.content         # str | None
event.tool_name       # str | None
event.tool_call_id    # str | None
event.summary         # str | None
event.source          # str | None: debug event source
event.level           # str | None: debug level
event.debug_message   # str | None: debug message
```

| `event_type` | 说明 | 可用属性 |
|---|---|---|
| `agent_start` | Agent 开始运行 | 无额外属性 |
| `agent_end` | Agent 运行结束 | `content` |
| `turn_start` | 一轮开始 | 无额外属性 |
| `turn_end` | 一轮结束 | 无额外属性 |
| `stream_token` | 流式文本片段 | `content` |
| `message_end` | 一条 assistant 消息完成 | `content` |
| `tool_call_start` | 工具调用开始 | `tool_name`, `tool_call_id` |
| `tool_call_end` | 工具调用完成 | `tool_call_id` |
| `compaction_start` | 上下文压缩开始 | 无额外属性 |
| `compaction_end` | 上下文压缩完成 | `summary` |
| `debug` | Rust/LLM 诊断信息，仅 FULL_DEBUG 返回 | `source`, `level`, `debug_message` |

## 提示词

### `load_prompt(path) -> str`

```python
from pi_agent import load_prompt

text = load_prompt("prompts/system.md")
```

文件不存在时抛出 `FileNotFoundError`，按 UTF-8 读取。

### `load_prompt_set(...) -> PromptSet`

```python
from pi_agent import load_prompt_set

prompt_set = load_prompt_set(
    system_main="prompts/system.md",
    compaction_system="prompts/compaction_system.md",
    compaction_initial="prompts/compaction_initial.md",
    compaction_update="prompts/compaction_update.md",
    compaction_turn_prefix="prompts/compaction_turn_prefix.md",
    tool_guidelines_dir="prompts/tool_guidelines",
    extra={"custom": "prompts/custom.md"},
)
```

所有路径参数均可省略。工具指导目录只读取该目录下后缀为 `.md` 的文件，并以文件名（不含扩展名）作为 key。

### `PromptSet`

```python
from pi_agent import PromptSet

prompt_set = PromptSet(
    system_main="...",
    compaction_system="...",
    compaction_initial="...",
    compaction_update="...",
    compaction_turn_prefix="...",
    tool_guidelines={"bash": "..."},
    extra={"custom": "..."},
)

custom = prompt_set.override_system_main("新的系统提示词")
```

## 内置工具

每个新 Session 自动注册以下工具：

| 名称 | 功能 |
|---|---|
| `bash` | 执行 shell 命令 |
| `read` | 读取文件 |
| `write` | 写入文件 |
| `edit` | 替换文件中的文本 |
| `grep` | 搜索文件内容 |
| `find` | 按文件名查找文件 |
| `ls` | 列出目录内容 |

工具调用失败会作为工具结果返回给模型，不会立即终止 Agent loop。`bash` 当前使用 `sh -c`，在 Windows 环境中需要可用的 POSIX shell。

## 日志示例

```python
from pi_agent import LogLevel, create_agent

agent = create_agent(
    api_key="sk-xxx",
    model="gpt-4o",
    log_level=LogLevel.DEBUG,
)

for line in agent.get_log_buffer():
    print(line)

agent.clear_logs()
```

## 底层 Rust API

以下类型通过 `pi_agent` 导出，用于需要直接访问 PyO3 层的场景：

```python
from pi_agent import (
    PyAgent,
    PySession,
    PyAgentEvent,
    PyEntry,
    PyUsage,
    PyBranchPoint,
    PyBranchSummary,
    PyTool,
    PyToolDefinition,
    create_entry_id,
    version,
)
```

`PyAgent` 负责直接创建 Rust Agent、运行请求、读取原始事件、导出当前会话树和注册工具。新的高层 API 不再创建或读取 JSONL 会话文件。

## 安全与限制

- 不要将 `api_key` 写入仓库、日志或公开文档。
- `bash`、`write`、`edit` 等工具拥有本机文件系统和命令执行能力，只应在受信任的工作目录和受控权限下启用。
- 工具执行没有独立超时；长时间运行的 shell 命令会阻塞当前 Agent 请求。
- `estimate_tokens()` 使用粗略字符长度估算，实际上下文使用量以 LLM 返回的 usage 为准。
- 多 Session 并发时，每个 Session 拥有独立的 Tokio task 和 broadcast channel，不会互相干扰。
