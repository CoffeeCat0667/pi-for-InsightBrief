# Pi Agent 项目上下文

最后更新: v0.1.9 (2026-08-25)

## 项目概述

Pi Agent 是一个基于 Rust + PyO3 的 Python 编码代理框架，支持：
- 自定义 API 多轮对话（OpenAI 兼容格式）
- 上下文记忆与会话树管理
- 工具调用（7 个内置工具）
- 流式传输（SSE）
- HTTP 错误自动重试

## 技术栈

| 组件 | 技术 |
|---|---|
| 核心 | Rust + PyO3 (abi3-py310) |
| 异步运行时 | Tokio (持久化) |
| HTTP 客户端 | reqwest (rustls-tls-webpki-roots) |
| 流处理 | futures + tokio-stream |
| 构建 | maturin |
| Python | >= 3.10 |

## 环境配置

### 中文路径问题
用户名 `咖啡猫` 导致 MinGW `ld.exe` 无法处理 `C:\Users\咖啡猫\.rustup\`。

**解决方案**: 将 toolchain 复制到 `D:\rustup`

### 编译环境变量
每次编译前需设置:
```cmd
set "RUSTUP_HOME=D:\rustup"
set "CARGO_HOME=D:\rustup\cargo"
set "PATH=D:\rustup\cargo\bin;%PATH%"
```

### 工具链
- `stable-x86_64-pc-windows-gnu`
- maturin build 需指定 `--target x86_64-pc-windows-gnu`

## 项目结构

```
pi-agent/
├── rust/                          # Rust 核心
│   ├── Cargo.toml                 # v0.1.9
│   ├── src/
│   │   ├── lib.rs
│   │   ├── agent/
│   │   │   ├── loop_.rs           # AgentLoop (含重试逻辑)
│   │   │   ├── types.rs           # AgentEvent, AgentLoopConfig
│   │   │   └── system_prompt.rs   # PromptSet, build_system_prompt()
│   │   ├── llm/
│   │   │   ├── client.rs          # LlmClient (HTTP 请求)
│   │   │   └── types.rs           # OpenAI 格式类型
│   │   ├── session/
│   │   │   ├── store.rs           # SessionStore (会话树)
│   │   │   ├── types.rs           # Entry, Message, Usage
│   │   │   └── compaction.rs      # 上下文压缩
│   │   ├── python/
│   │   │   ├── agent.rs           # PyAgent (Python 绑定)
│   │   │   └── types.rs           # PyAgentEvent 等
│   │   └── tools/                 # 7 个内置工具
│   │       ├── bash.rs
│   │       ├── read.rs
│   │       ├── write.rs
│   │       ├── edit.rs
│   │       ├── grep.rs
│   │       ├── find.rs
│   │       └── ls.rs
│   └── prompts/                   # 提示词文件
├── src/pi_agent/                  # Python 高层 API
│   ├── __init__.py
│   ├── agent.py                   # Agent 单例
│   ├── session.py                 # Session (含异步 API)
│   ├── event_buffer.py            # EventBuffer (事件过滤)
│   ├── types.py                   # LogLevel, OutputMode
│   ├── logging.py                 # Logger
│   └── prompts.py                 # PromptSet 加载
├── dist/                          # 构建产物
│   └── pi_agent-0.1.9-cp310-abi3-win_amd64.whl
├── API_REFERENCE.md               # API 文档
├── output_content.md              # 输出事件格式文档
└── pyproject.toml                 # Python 包配置
```

## 核心架构

### Agent 单例模式
```python
agent = create_agent(api_key="...", model="...", sessions={})
session = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)
```

### 会话树 (Session Tree)
- 每个 Session 拥有独立的 Rust SessionStore
- 支持分支切换、compaction、usage 统计
- 内存存储，通过 `sessions` 字典持久化

### 事件系统
- broadcast channel (容量 10000)
- 11 种事件类型
- EventBuffer 根据 OutputMode 过滤
- error 级别 debug 事件在所有模式下透传

### 异步架构 (v0.1.5+)
- Rust 持久化 Tokio runtime
- `Arc<AgentLoop>` + `Arc<broadcast::Sender>`
- `run()` spawn 后台任务，立即返回
- Python 通过 `events()` 异步迭代器实时接收事件

## 关键参数默认值

| 参数 | 默认值 | 说明 |
|---|---|---|
| `max_turns` | 50 | 单次 prompt 最大 turn 数 |
| `max_retries` | 10 | HTTP 错误重试次数 |
| `reserve_tokens` | 16384 | 模型响应预留 token |
| `keep_recent_tokens` | 20000 | 压缩时保留的最近 token |
| `context_window` | 256000 | 上下文窗口大小 |
| broadcast channel | 10000 | 事件通道容量 |
| HTTP timeout | 300s | 请求超时 |
| connect timeout | 30s | 连接超时 |

## HTTP 错误重试机制 (v0.1.6)

- 所有 HTTP 错误自动重试 (502/429/503/xxx)
- 指数退避: 1s → 2s → 4s → 8s → 16s → 30s 封顶
- 每次重试前检查 cancel_flag
- 重试期间发送 warning 级别 debug 事件
- 所有重试失败后发送 error 级别 debug 事件并终止

## 已修复的关键 Bug

### v0.1.5
- 空 message_end 提前退出
- 内容重复拼接
- AgentEnd content 缺失
- `truncate_str` UTF-8 panic

### v0.1.9
- 修复 `analyze_compaction` 上下文高估：原逻辑逐条累加每条 assistant entry 的 `input_tokens`（每轮都是完整请求输入），导致上下文被高估几十倍、几乎每轮误触发压缩
- 改为取当前活跃分支最新一条 assistant entry 的 `input_tokens` 作为真实当前上下文，再与 `context_window - reserve_tokens` 比较
- `CompactionAnalysis::total_tokens` 增加语义注释，明确为"当前待发送上下文"而非"历史请求之和"

### v0.1.8
- 修复 `Session.stream()` 启动竞态：`events()` 在 `run_async` 尚未启动时把"未启动"（`is_running()==False`）误判为"已结束"，导致首轮空回复、第二轮泄漏上一轮遗留事件
- `stream()` 在遍历 `events()` 前等待任务真正启动（`while not run_task.done() and not is_running(): sleep`）

### v0.1.7
- 新增 `Session.stream()` 方法：合并 `run_async()` + `events()`，一行代码获得实时事件流
- 新增 `Agent.stream()` 方法，与 `run_async()` 同级
- 修复 `run_async()` 末尾 `_drain_native_events()` 竞态问题

### v0.1.6
- running flag 卡死: `bool` → `Arc<AtomicBool>`
- 错误被静默丢弃: spawned task 发送 error debug 事件
- 错误事件不可见: error 级别 debug 在所有模式下透传

## 构建命令

```cmd
# 开发构建 (安装到本地)
maturin develop --release --target x86_64-pc-windows-gnu

# 发布构建 (输出到 dist/)
maturin build --release --target x86_64-pc-windows-gnu -o dist

# Rust 测试
cargo test --target x86_64-pc-windows-gnu

# Python 测试
python test_new_api.py
python test_async_api.py
```

## GitHub 仓库

- URL: https://github.com/CoffeeCat0667/pi-for-InsightBrief.git
- 当前分支: main
- 最新提交: v0.1.9

## 使用示例

### 同步 API
```python
from pi_agent import create_agent, OutputMode

agent = create_agent(api_key="sk-xxx", model="gpt-4o")
session = agent.create_session()

agent.run(session.session_id, "你好")
print(session.wait_response())
```

### 异步 API
```python
import asyncio
from pi_agent import create_agent, OutputMode

async def main():
    agent = create_agent(api_key="sk-xxx", model="gpt-4o")
    session = agent.create_session()

    await agent.run_async(session.session_id, "你好")
    async for event in session.events():
        if event.event_type == "stream_token":
            print(event.content, end="")
    print()
```

### 流式 API (v0.1.8+)
```python
import asyncio
from pi_agent import create_agent, OutputMode

async def main():
    agent = create_agent(api_key="sk-xxx", model="gpt-4o")
    session = agent.create_session()

    async for event in agent.stream(session.session_id, "你好"):
        if event.event_type == "stream_token":
            print(event.content, end="")
    print()

asyncio.run(main())
```

### 自定义重试次数
```python
agent = create_agent(
    api_key="sk-xxx",
    model="gpt-4o",
    max_retries=5,  # 最多重试 5 次
)
```

## 注意事项

1. **中文路径**: Rust toolchain 必须在 `D:\rustup`
2. **旧 .pyd 清理**: `src/pi_agent/` 下旧 `.pyd` 文件会 shadow 安装包
3. **abi3 wheel**: `cp310-abi3` 兼容 Python 3.10-3.14
4. **会话持久化**: 通过 `sessions` 字典手动管理，不自动读写 JSONL
5. **DEBUG 输出**: Rust 层所有 `eprintln!` 已移除，改为结构化 AgentEvent
