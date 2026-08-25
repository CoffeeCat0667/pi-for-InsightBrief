# Output Content Format

本文档定义 `Session.next_event()`、`Session.next_event_raw()`、
`Session.events()` 和 `Session.wait_response_stream()` 可观察到的输出事件。

## 事件对象

每个事件都是一个 `PyAgentEvent` 对象。通过以下属性区分来源和内容：

```python
event.event_type
event.content
event.tool_name
event.tool_call_id
event.summary
event.source
event.level
event.debug_message
```

`source`、`level` 和 `debug_message` 只对 `debug` 事件有值。

## 事件类型

### `stream_token`

模型流式返回的文本片段。

```text
event_type: "stream_token"
content: "文本片段"
```

这是最终模型回答的一部分。多个 `stream_token` 按事件顺序拼接后构成回答文本。

### `message_end`

一条 assistant 消息完成。

```text
event_type: "message_end"
content: "完整消息文本"
```

工具调用轮次的 `content` 可能为空。空的 `message_end` 不代表最终回答结束。

### `agent_start`

一次 Agent 请求开始。

```text
event_type: "agent_start"
```

### `agent_end`

一次 Agent 请求完成。

```text
event_type: "agent_end"
content: "最终消息文本"
```

### `turn_start`

一次内部 Agent turn 开始。

```text
event_type: "turn_start"
```

### `turn_end`

一次内部 Agent turn 完成。

```text
event_type: "turn_end"
```

### `tool_call_start`

模型请求执行工具。

```text
event_type: "tool_call_start"
tool_name: "read"
tool_call_id: "call_123"
```

### `tool_call_end`

工具执行完成。

```text
event_type: "tool_call_end"
tool_call_id: "call_123"
```

工具结果不会通过 `content` 暴露在该事件上；结果会写入会话树并发送给模型。

### `compaction_start`

上下文压缩开始。

```text
event_type: "compaction_start"
```

### `compaction_end`

上下文压缩完成。

```text
event_type: "compaction_end"
summary: "压缩摘要"
```

### `debug`

Rust Agent 或 LLM 客户端产生的诊断信息。`error` 级别的 debug 事件在所有
`OutputMode` 下均返回，其余仅在 `OutputMode.FULL_DEBUG` 下返回。

```text
event_type: "debug"
source: "llm"
level: "debug"
debug_message: "POST https://api.example.com/v1/chat/completions (model=gpt-4o, messages=2, stream=true)"
```

当前 `source` 值包括：

| `source` | 来源 |
|---|---|
| `llm` | LLM 请求、响应、连接和 HTTP 错误 |
| `agent.stream` | SSE 流处理统计和工具调用收集状态 |
| `session` | 会话写入等状态 |
| `agent` | Agent 循环状态（如任务取消、重试） |

当前 `level` 值包括：

| `level` | 含义 |
|---|---|
| `debug` | 调试信息 |
| `info` | 普通状态信息 |
| `warning` | 警告（如重试等待） |
| `error` | 请求或执行错误（始终返回） |

## OutputMode 过滤

| 事件类型 | `CONTENT_ONLY` | `THINKING` | `FULL_DEBUG` |
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
| `debug` (error) | **是** | **是** | **是** |
| `debug` (其他) | 否 | 否 | 是 |

## 最终回答与 DEBUG 分离

`wait_response()`、`wait_response_async()` 和 `wait_response_stream()` 只处理：

```text
stream_token
message_end
agent_end
```

因此 `debug`、工具和压缩事件不会混入最终回答文本。

要同时获取最终回答和 DEBUG：

```python
session.output_mode = OutputMode.FULL_DEBUG
await agent.run_async(session.session_id, "你好")

debug_events = []
answer_parts = []

async for event in session.events():
    if event.event_type == "debug":
        debug_events.append({
            "source": event.source,
            "level": event.level,
            "message": event.debug_message,
        })
    elif event.event_type == "stream_token":
        answer_parts.append(event.content or "")

final_answer = "".join(answer_parts)
```

DEBUG 事件通过 Session 的事件缓冲区返回，不再由 Rust 直接写入控制台。
即使底层 LLM 请求失败，调用方也可以在捕获 `Session.run_async()` 异常后读取已经产生的 `debug` 和 `error` 事件。

## 异步事件流

v0.1.8 在 v0.1.7 的基础上修复了 `stream()` 的启动竞态。Rust Agent loop 在 Tokio 运行时上执行，事件通过
broadcast channel 实时发送。推荐直接使用 `stream()`；向后兼容的 `run_async()` + `events()` 组合仍然可用：

> **v0.1.8 修复说明**：`stream()` 现在会在遍历 `events()` 前等待 `run_async` 真正启动
> （`while not run_task.done() and not is_running(): await asyncio.sleep(0.01)`），
> 避免 `events()` 把"任务尚未启动"误判为"已结束"而提前返回，从而消除首轮空回复和
> 跨轮遗留事件泄漏。

```python
async for event in session.stream("你的消息"):
    if event.event_type == "stream_token":
        print(event.content, end="", flush=True)
```

等价的旧式调用方式：

```python
await session.run_async("你的消息")

async for event in session.events():
    if event.event_type == "stream_token":
        print(event.content, end="", flush=True)
    elif event.event_type == "tool_call_start":
        print(f"\n[工具: {event.tool_name}]")
    elif event.event_type == "debug" and event.level == "error":
        print(f"\n[错误: {event.debug_message}]")
```

`events()` 会在以下情况结束迭代：
- 收到 `agent_end` 事件
- 超时（默认 300 秒）
- 原生 agent 停止运行且缓冲区为空

## HTTP 错误自动重试

v0.1.6 起，LLM 调用遇到 HTTP 错误（502/429/503/xxx）时自动重试，v0.1.8 继续保持该行为：
- 最多重试 `max_retries` 次（默认 10）
- 采用指数退避：1s, 2s, 4s, 8s, 16s, 30s 封顶
- 每次重试前检查 `cancel_flag`，用户可随时取消
- 重试期间发送 `warning` 级别 debug 事件，用户可实时观察重试进度
- 所有重试失败后，发送 `error` 级别 debug 事件并终止任务
- `running` flag 始终正确重置，不会卡死
