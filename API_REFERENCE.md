# Pi Agent API Reference

## 安装

```bash
# 从 wheel 安装
pip install pi_agent-0.1.0-cp310-abi3-win_amd64.whl

# 或从源码构建
maturin develop --release
```

要求 Python >= 3.10。

---

## 架构概览

```
┌─────────────────────────────────────────────────┐
│                   Agent (单例)                   │
│  - API Key / Model / Base URL                   │
│  - 提示词集合 (PromptSet)                        │
│  - 日志器 (Logger)                               │
│                                                 │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐     │
│  │ Session 1 │ │ Session 2 │ │ Session 3 │     │
│  │ content   │ │ thinking  │ │ full_debug│     │
│  └───────────┘ └───────────┘ └───────────┘     │
└─────────────────────────────────────────────────┘
```

- **Agent**: 全局单例，管理 LLM 连接、工具和提示词
- **Session**: 独立的对话会话，拥有自己的事件缓冲区和输出模式

---

## 快速开始

```python
from pi_agent import create_agent, OutputMode

# 1. 创建 Agent（全局单例）
agent = create_agent(
    api_key="sk-xxx",
    model="gpt-4",
)

# 2. 创建 Session
session = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)

# 3. 运行对话
agent.run(session.session_id, "你好")

# 4. 获取回复
response = session.wait_response()
print(response)

# 或流式获取
for token in session.wait_response_stream():
    print(token, end="", flush=True)
```

---

## 核心 API

### `create_agent(**kwargs) -> Agent`

创建或获取全局 Agent 单例。

```python
from pi_agent import create_agent, LogLevel

agent = create_agent(
    # 必填参数
    api_key="sk-xxx",                          # API 密钥
    model="gpt-4",                             # 模型名称

    # 可选：API 配置
    base_url=None,                             # 自定义 API 端点

    # 可选：日志配置
    log_level=LogLevel.INFO,                   # 日志级别

    # 可选：对话配置
    max_turns=50,                              # 最大对话轮数
    reserve_tokens=16384,                      # 预留 token 数
    keep_recent_tokens=20000,                  # 保留最近 token 数
    context_window=128000,                     # 上下文窗口大小

    # 可选：外部提示词路径
    system_main="prompts/system.md",           # 主系统提示词
    compaction_system="prompts/compaction_sys.md",
    compaction_initial="prompts/compaction_init.md",
    compaction_update="prompts/compaction_update.md",
    compaction_turn_prefix="prompts/compaction_prefix.md",
    tool_guidelines_dir="prompts/tool_guidelines/",  # 工具指导目录

    # 可选：额外配置
    extra_guidelines=["规则1", "规则2"],        # 额外工具指导
    extra_prompts={"custom": "path/to/file.md"},  # 自定义提示词
)
```

**行为：**
- 首次调用创建 Agent 并存储为全局单例
- 后续调用更新配置并返回同一实例
- 可通过 `get_agent()` 获取当前单例

---

### `Agent` 类

#### 属性

```python
agent.model           # str: 当前模型
agent.log_level       # LogLevel: 当前日志级别（可设置）
```

#### 方法

```python
# 创建会话
session = agent.create_session(
    output_mode=OutputMode.CONTENT_ONLY,  # 输出模式
    cwd="/path/to/project",              # 工作目录
    session_path="custom.jsonl",         # 自定义会话文件路径
)

# 继续会话
session = agent.continue_session(
    session_id="abc-123",
    output_mode=OutputMode.THINKING,     # 可选：切换输出模式
)

# 获取会话
session = agent.get_session("abc-123")

# 列出所有会话
session_ids = agent.list_sessions()

# 删除会话
success = agent.delete_session("abc-123")

# 运行对话
agent.run(session_id, "你的消息")

# 获取下一个事件
event = agent.next_event(session_id)

# 等待完整回复
response = agent.wait_response(session_id, timeout=300.0)

# 获取日志
logs = agent.get_log_buffer()

# 清空日志
agent.clear_logs()
```

---

### `Session` 类

```python
session = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)
```

#### 属性

```python
session.session_id    # str: 会话唯一 ID
session.output_mode   # OutputMode: 当前输出模式（可设置）
```

#### 方法

```python
# 运行对话
session.run("你的消息")

# 获取下一个事件（符合 output_mode）
event = session.next_event()

# 获取下一个原始事件（不过滤）
event = session.next_event_raw()

# 等待完整回复
response = session.wait_response(timeout=300.0)

# 流式等待回复
for token in session.wait_response_stream(timeout=300.0):
    print(token, end="", flush=True)

# 关闭会话
session.close()
```

---

## 枚举类型

### `LogLevel`

```python
from pi_agent import LogLevel

LogLevel.DEBUG     # 调试信息
LogLevel.INFO      # 一般信息
LogLevel.WARNING   # 警告
LogLevel.ERROR     # 错误
```

### `OutputMode`

```python
from pi_agent import OutputMode

OutputMode.CONTENT_ONLY  # 仅输出对话内容（stream_token / message_end）
OutputMode.THINKING      # 输出思考内容 + 对话内容
OutputMode.FULL_DEBUG    # 输出所有内容（提示词 + 思考 + 对话）
```

**OutputMode 过滤规则：**

| 事件类型 | CONTENT_ONLY | THINKING | FULL_DEBUG |
|----------|:---:|:---:|:---:|
| `stream_token` | ✓ | ✓ | ✓ |
| `message_end` | ✓ | ✓ | ✓ |
| `tool_call_start` | | ✓ | ✓ |
| `tool_call_end` | | ✓ | ✓ |
| `compaction_start` | | ✓ | ✓ |
| `compaction_end` | | ✓ | ✓ |
| `turn_start` | | ✓ | ✓ |
| `turn_end` | | ✓ | ✓ |

---

## 事件类型

通过 `session.next_event()` 或 `agent.next_event(session_id)` 获取。

```python
event = session.next_event()

event.event_type    # str: 事件类型
event.content       # str | None: 消息内容
event.tool_name     # str | None: 工具名称
event.tool_call_id  # str | None: 工具调用 ID
event.summary       # str | None: 压缩摘要
```

| event_type | 说明 | 可用属性 |
|------------|------|----------|
| `stream_token` | 流式输出 token | `content` |
| `message_end` | 消息完成 | `content` |
| `tool_call_start` | 工具调用开始 | `tool_name`, `tool_call_id` |
| `tool_call_end` | 工具调用完成 | `tool_call_id` |
| `compaction_start` | 上下文压缩开始 | - |
| `compaction_end` | 上下文压缩完成 | `summary` |
| `turn_start` | 新一轮对话开始 | - |
| `turn_end` | 一轮对话结束 | - |
| `agent_start` | Agent 开始运行 | - |
| `agent_end` | Agent 运行结束 | `content` |

---

## 提示词加载

### `load_prompt(path) -> str`

从文件加载单个提示词。

```python
from pi_agent import load_prompt

prompt = load_prompt("prompts/system.md")
```

### `load_prompt_set(**kwargs) -> PromptSet`

从外部文件加载提示词集合。

```python
from pi_agent import load_prompt_set

ps = load_prompt_set(
    system_main="prompts/system.md",
    compaction_system="prompts/compaction_system.md",
    compaction_initial="prompts/compaction_initial.md",
    compaction_update="prompts/compaction_update.md",
    compaction_turn_prefix="prompts/compaction_turn_prefix.md",
    tool_guidelines_dir="prompts/tool_guidelines/",
    extra={"custom": "prompts/custom.md"},
)

print(ps.system_main)           # 主系统提示词
print(ps.compaction_system)     # 压缩系统提示词
print(ps.tool_guidelines)       # dict: {name: content}
```

### `PromptSet` 类

```python
from pi_agent import PromptSet

ps = PromptSet(
    system_main="...",
    compaction_system="...",
    compaction_initial="...",
    compaction_update="...",
    compaction_turn_prefix="...",
    tool_guidelines={"bash": "...", "read": "..."},
    extra={"custom": "..."},
)

# 覆盖系统提示词
new_ps = ps.override_system_main("新的系统提示词")
```

---

## 完整示例

### 基础对话

```python
from pi_agent import create_agent, OutputMode

agent = create_agent(api_key="sk-xxx", model="gpt-4")
session = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)

agent.run(session.session_id, "用一句话解释什么是递归？")
response = session.wait_response()
print(response)
```

### 流式输出 + 工具调用

```python
from pi_agent import create_agent, OutputMode

agent = create_agent(api_key="sk-xxx", model="gpt-4")
session = agent.create_session(
    output_mode=OutputMode.THINKING,
    cwd="/path/to/project",
)

agent.run(session.session_id, "列出当前目录的文件")

for token in session.wait_response_stream():
    print(token, end="", flush=True)
print()
```

### 多会话管理

```python
from pi_agent import create_agent, OutputMode

agent = create_agent(api_key="sk-xxx", model="gpt-4")

# 创建多个会话
s1 = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)
s2 = agent.create_session(output_mode=OutputMode.THINKING)

# 并行运行
agent.run(s1.session_id, "问题1")
agent.run(s2.session_id, "问题2")

# 获取各自回复
r1 = s1.wait_response()
r2 = s2.wait_response()

# 继续会话
agent.run(s1.session_id, "追问")
r1_new = s1.wait_response()
```

### 外部提示词

```python
from pi_agent import create_agent, OutputMode

agent = create_agent(
    api_key="sk-xxx",
    model="gpt-4",
    system_main="prompts/system.md",
    compaction_system="prompts/compaction_system.md",
    tool_guidelines_dir="prompts/tool_guidelines/",
)

session = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)
agent.run(session.session_id, "你好")
```

### 日志调试

```python
from pi_agent import create_agent, LogLevel, OutputMode

agent = create_agent(
    api_key="sk-xxx",
    model="gpt-4",
    log_level=LogLevel.DEBUG,
)

session = agent.create_session(output_mode=OutputMode.FULL_DEBUG)
agent.run(session.session_id, "测试")

# 查看日志
for log in agent.get_log_buffer():
    print(log)

# 事件循环
while True:
    event = session.next_event()
    if event is None:
        break
    print(f"[{event.event_type}] {event.content or event.tool_name or ''}")
```

### 自定义工具

```python
from pi_agent import create_agent
import json

class CalculatorTool:
    def definition(self):
        from pi_agent import ToolDefinition
        return ToolDefinition(
            name="calculator",
            description="执行数学计算",
            parameters=json.dumps({
                "type": "object",
                "properties": {
                    "expression": {"type": "string"}
                },
                "required": ["expression"]
            })
        )
    
    def execute(self, arguments):
        args = json.loads(arguments)
        result = eval(args["expression"])
        return json.dumps({"result": result})

agent = create_agent(api_key="sk-xxx", model="gpt-4")
# 注册工具需要通过 Rust 原生 API
```

---

## 向后兼容

旧 API 仍然可用：

```python
from pi_agent import (
    # Rust 原生类型
    PyAgent,
    PySession,
    PyAgentEvent,
    PyEntry,
    PyUsage,
    PyBranchPoint,
    PyBranchSummary,
    PyTool,
    PyToolDefinition,
    version,
    create_entry_id,
)
```
