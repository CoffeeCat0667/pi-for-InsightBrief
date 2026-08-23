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

## 核心类

### `Agent` (PyAgent)

主入口，管理对话循环、工具调用和上下文压缩。

```python
from pi_agent import Agent

agent = Agent(
    api_key="sk-xxx",                    # 必填：API 密钥
    model="gpt-4",                       # 必填：模型名称
    session_path="session.jsonl",        # 必填：会话持久化文件路径
    base_url=None,                       # 可选：自定义 API 端点（None=OpenAI 官方）
    system_prompt=None,                  # 可选：覆盖默认系统提示词
    max_turns=50,                        # 可选：最大对话轮数（默认 50）
    reserve_tokens=16384,                # 可选：为回复预留的 token 数（默认 16384）
    keep_recent_tokens=20000,            # 可选：压缩后保留的最近 token 数（默认 20000）
    context_window=128000,               # 可选：上下文窗口大小（默认 128000）
    append_system_prompt=None,           # 可选：追加到系统提示词末尾的文本
    extra_guidelines=None,               # 可选：额外的工具使用指导（字符串列表）
    cwd=None,                           # 可选：工作目录（用于工具执行）
)
```

#### 参数详解

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `api_key` | str | 必填 | API 密钥 |
| `model` | str | 必填 | 模型名称（如 `gpt-4`, `gpt-4o`, `claude-3-opus`） |
| `session_path` | str | 必填 | 会话 JSONL 文件路径，不存在则自动创建 |
| `base_url` | str \| None | None | 自定义 API 端点，None 使用 OpenAI 官方 |
| `system_prompt` | str \| None | None | 覆盖默认系统提示词 |
| `max_turns` | int | 50 | 单次 `run()` 的最大对话轮数 |
| `reserve_tokens` | int | 16384 | 为 LLM 回复预留的 token 数 |
| `keep_recent_tokens` | int | 20000 | 上下文压缩时保留的最近 token 数 |
| `context_window` | int | 128000 | LLM 上下文窗口大小 |
| `append_system_prompt` | str \| None | None | 追加到系统提示词末尾 |
| `extra_guidelines` | list[str] \| None | None | 额外的工具使用指导 |
| `cwd` | str \| None | None | 工具执行的工作目录 |

#### 方法

```python
# 注册内置工具（ls, read, write, edit, grep, find, bash）
agent.register_builtin_tools()

# 注册自定义工具
agent.register_tool(my_tool)

# 运行对话（同步，事件通过 next_event() 获取）
agent.run("你的消息")
```

#### 自定义 API 示例

```python
# OpenAI 兼容的第三方 API
agent = Agent(
    api_key="sk-0917aed21cc2a74efa7af30c3cee4a4736bfbf3119a7de116bd5a629f6c7b208",
    model="gpt-4",
    session_path="session.jsonl",
    base_url="https://api.example.com/v1",
)

# Anthropic 兼容 API
agent = Agent(
    api_key="sk-ant-xxx",
    model="claude-3-opus-20240229",
    session_path="session.jsonl",
    base_url="https://api.anthropic.com",
)

# 本地 Ollama
agent = Agent(
    api_key="ollama",
    model="llama3",
    session_path="session.jsonl",
    base_url="http://localhost:11434/v1",
)
```

---

### `Session` (PySession)

会话管理，支持分支和持久化。

```python
from pi_agent import Session

# 创建新会话
session = Session(model="gpt-4", system_prompt="你是一个助手")

# 从文件加载
session = Session.from_file("session.jsonl")
```

#### 方法

```python
# 添加消息
entry_id = session.append_user("你好")
entry_id = session.append_assistant("你好！有什么可以帮助你的？")

# 获取当前分支的消息
messages = session.messages()  # 返回 list[Entry]

# 获取当前叶子节点
leaf = session.leaf()  # 返回 Entry | None

# 获取 token 使用情况
usage = session.total_usage()  # 返回 Usage

# 获取会话信息
session.session_id()   # 返回 str | None
session.model()        # 返回 str | None
session.entry_count()  # 返回 int

# 分支管理
branch_points = session.branch_points()  # 返回 list[BranchPoint]
session.switch_branch(entry_id)          # 切换到指定分支
summary = session.branch_summary(entry_id)  # 获取分支摘要
```

---

### `AgentEvent` (PyAgentEvent)

Agent 运行时产生的事件。通过 `agent.next_event()` 获取。

#### 事件类型

| event_type | 说明 | 可用属性 |
|------------|------|----------|
| `agent_start` | Agent 开始运行 | - |
| `agent_end` | Agent 运行结束 | `content`（最终回复） |
| `turn_start` | 新一轮对话开始 | - |
| `turn_end` | 一轮对话结束 | - |
| `message_end` | LLM 消息完成 | `content`（完整回复文本） |
| `stream_token` | 流式输出 token | `content`（单个 token） |
| `tool_call_start` | 工具调用开始 | `tool_name`, `tool_call_id` |
| `tool_call_end` | 工具调用完成 | `tool_call_id` |
| `compaction_start` | 上下文压缩开始 | - |
| `compaction_end` | 上下文压缩完成 | `summary`（压缩摘要） |

#### 属性

```python
event = agent.next_event()

event.event_type    # str: 事件类型
event.content       # str | None: 消息内容（message_end / stream_token）
event.tool_name     # str | None: 工具名称（tool_call_start）
event.tool_call_id  # str | None: 工具调用 ID
event.summary       # str | None: 压缩摘要（compaction_end）
```

---

### `Entry` (PyEntry)

会话中的单条消息记录。

```python
entry = session.leaf()

entry.id            # str: 唯一 ID
entry.parent_id     # str | None: 父条目 ID
entry.content       # str: 消息内容
entry.role          # str: "user" | "assistant" | "system" | "tool"
entry.input_tokens  # int: 输入 token 数
entry.output_tokens # int: 输出 token 数
```

---

### `Usage` (PyUsage)

Token 使用统计。

```python
usage = session.total_usage()

usage.input_tokens   # int: 输入 token 数
usage.output_tokens  # int: 输出 token 数
usage.total          # int: 总 token 数
```

---

### `BranchPoint` (PyBranchPoint)

分支点信息。

```python
bp = session.branch_points()[0]

bp.parent_id  # str: 父条目 ID
bp.children   # list[str]: 子条目 ID 列表
```

---

### `BranchSummary` (PyBranchSummary)

分支摘要（用于被放弃的分支）。

```python
summary = session.branch_summary(entry_id)

summary.goal           # str: 目标
summary.progress       # str: 进展
summary.decisions      # str: 决策
summary.next_steps     # str: 下一步
summary.files_touched  # list[str]: 涉及的文件
```

---

### `Tool` (PyTool)

自定义工具。

```python
from pi_agent import Tool, ToolDefinition

# 通过 Python 类实现自定义工具
class MyTool:
    def definition(self) -> ToolDefinition:
        return ToolDefinition(
            name="my_tool",
            description="我的自定义工具",
            parameters='{"type": "object", "properties": {"input": {"type": "string"}}, "required": ["input"]}'
        )
    
    def execute(self, arguments: str) -> str:
        # arguments 是 JSON 字符串
        import json
        args = json.loads(arguments)
        return f"结果: {args['input']}"

# 注册
agent.register_tool(MyTool())
```

---

### `ToolDefinition` (PyToolDefinition)

工具定义。

```python
ToolDefinition(
    name="tool_name",           # str: 工具名称
    description="工具描述",      # str: 工具描述
    parameters='{"type": "object", ...}'  # str: JSON Schema 参数定义
)
```

---

## 完整示例

### 基础对话

```python
from pi_agent import Agent

agent = Agent(
    api_key="sk-xxx",
    model="gpt-4",
    session_path="session.jsonl",
)
agent.register_builtin_tools()

agent.run("用一句话解释什么是递归？")

while True:
    event = agent.next_event()
    if event is None:
        break
    if event.event_type == "stream_token":
        print(event.content, end="", flush=True)
    elif event.event_type == "message_end":
        print()
```

### 流式输出 + 工具调用

```python
from pi_agent import Agent
import time

agent = Agent(
    api_key="sk-xxx",
    model="gpt-4",
    session_path="session.jsonl",
    base_url="https://api.example.com/v1",
    cwd="/path/to/project",
)
agent.register_builtin_tools()

agent.run("列出当前目录的文件")

while True:
    event = agent.next_event()
    if event is None:
        time.sleep(0.01)
        continue
    
    if event.event_type == "stream_token":
        print(event.content, end="", flush=True)
    elif event.event_type == "tool_call_start":
        print(f"\n[调用工具: {event.tool_name}]", end="", flush=True)
    elif event.event_type == "tool_call_end":
        print(" -> 完成")
    elif event.event_type == "message_end":
        print("\n[完成]")
        break
```

### 自定义工具

```python
from pi_agent import Agent, ToolDefinition
import json

class CalculatorTool:
    def definition(self):
        return ToolDefinition(
            name="calculator",
            description="执行数学计算",
            parameters=json.dumps({
                "type": "object",
                "properties": {
                    "expression": {"type": "string", "description": "数学表达式"}
                },
                "required": ["expression"]
            })
        )
    
    def execute(self, arguments):
        args = json.loads(arguments)
        try:
            result = eval(args["expression"])  # 注意：生产环境应使用安全的表达式解析
            return json.dumps({"result": result})
        except Exception as e:
            return json.dumps({"error": str(e)})

agent = Agent(
    api_key="sk-xxx",
    model="gpt-4",
    session_path="session.jsonl",
)
agent.register_builtin_tools()
agent.register_tool(CalculatorTool())

agent.run("计算 123 * 456")
```

### 多轮对话

```python
from pi_agent import Agent
import time

agent = Agent(
    api_key="sk-xxx",
    model="gpt-4",
    session_path="session.jsonl",  # 会话自动持久化
)
agent.register_builtin_tools()

# 第一轮
agent.run("我的名字是张三")
while True:
    event = agent.next_event()
    if event is None:
        time.sleep(0.01)
        continue
    if event.event_type == "message_end":
        print(f"助手: {event.content}")
        break

# 第二轮（Agent 自动记住上下文）
agent.run("你记得我叫什么吗？")
while True:
    event = agent.next_event()
    if event is None:
        time.sleep(0.01)
        continue
    if event.event_type == "message_end":
        print(f"助手: {event.content}")
        break
```

### 上下文压缩

当对话历史超过上下文窗口时，Agent 自动压缩旧消息为摘要：

```python
agent = Agent(
    api_key="sk-xxx",
    model="gpt-4",
    session_path="session.jsonl",
    context_window=16000,      # 小窗口更容易触发压缩
    keep_recent_tokens=2000,
)

# 多轮对话后会自动触发压缩
for i in range(100):
    agent.run(f"第 {i} 轮对话")
    while True:
        event = agent.next_event()
        if event is None:
            time.sleep(0.01)
            continue
        if event.event_type == "compaction_start":
            print("[上下文压缩中...]")
        elif event.event_type == "compaction_end":
            print(f"[压缩完成: {event.summary[:50]}...]")
        elif event.event_type == "message_end":
            break
```

---

## 事件循环模式

```python
import time

def wait_for_response(agent):
    """等待 Agent 完成并返回最终回复"""
    full_response = []
    while True:
        event = agent.next_event()
        if event is None:
            time.sleep(0.01)
            continue
        
        if event.event_type == "stream_token":
            print(event.content, end="", flush=True)
            full_response.append(event.content)
        elif event.event_type == "tool_call_start":
            print(f"\n[工具: {event.tool_name}]", end="", flush=True)
        elif event.event_type == "tool_call_end":
            print(" -> 完成")
        elif event.event_type == "message_end":
            print()
            return "".join(full_response)
        elif event.event_type == "agent_end":
            return "".join(full_response)

# 使用
agent.run("你好")
response = wait_for_response(agent)
```

---

## 常量和辅助函数

```python
from pi_agent import version, create_entry_id

# 获取版本号
print(version())  # "0.1.0"

# 生成唯一 ID
entry_id = create_entry_id()
```
