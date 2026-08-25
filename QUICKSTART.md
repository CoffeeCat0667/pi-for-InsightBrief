# Pi Agent v0.1.8 快速开始

## 安装

```bash
cd D:\Project\Multiple_Project\NewsSpider\Agent\pi-agent

# 创建虚拟环境
python -m venv .venv
call .venv\Scripts\activate.bat  # Windows
# source .venv/bin/activate  # Linux/Mac

# 安装 maturin
pip install maturin

# 安装已构建 wheel
pip install dist/pi_agent-0.1.8-cp310-abi3-win_amd64.whl

# 或从源码构建并安装
maturin develop --release
```

## 设置 API Key

```bash
# Windows
set OPENAI_API_KEY=sk-your-key-here

# Linux/Mac
export OPENAI_API_KEY=sk-your-key-here
```

## 运行测试

```bash
# 基础功能测试（无需 API Key）
python test_basic.py

# 完整功能演示（需要 API Key）
python examples/complete_demo.py

# Rust 单元测试
cd rust
cargo test
```

## 在你的项目中使用

```python
import asyncio
from pi_agent import create_agent, OutputMode

agent = create_agent(
    api_key="sk-...",
    model="gpt-4o",
)
session = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)

async def main():
    # v0.1.8 流式 API：自动启动请求并实时返回事件（已修复启动竞态）
    async for event in session.stream("你好！"):
        if event.event_type == "stream_token":
            print(event.content, end="", flush=True)
    print()

asyncio.run(main())
```

## 向后兼容的异步 API

```python
async def legacy_async_main():
    await session.run_async("用一句话解释递归")

    async for event in session.events():
        if event.event_type == "stream_token":
            print(event.content, end="", flush=True)

asyncio.run(legacy_async_main())
```

## 可用工具

内置工具（自动注册）：
- `bash` - 执行 shell 命令
- `read` - 读取文件
- `write` - 写入文件
- `edit` - 编辑文件
- `grep` - 搜索内容
- `find` - 查找文件
- `ls` - 列出目录

## 文件结构

```
pi-agent/
├── rust/
│   └── src/                     # Rust 源码
├── src/pi_agent/                # Python 高层 API
├── dist/
│   └── pi_agent-0.1.8-*.whl     # wheel 构建产物
├── examples/
│   ├── complete_demo.py         # 完整示例
│   └── integration.py           # 集成示例
├── test_basic.py                # 基础测试
└── test_async_api.py            # 异步 API 测试
```

## 更多信息

查看 `examples/` 目录中的示例文件。
