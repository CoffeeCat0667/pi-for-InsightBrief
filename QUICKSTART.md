# Pi Agent 快速开始

## 安装

```bash
cd D:\Project\Multiple_Project\NewsSpider\Agent\pi-agent

# 创建虚拟环境
python -m venv .venv
call .venv\Scripts\activate.bat  # Windows
# source .venv/bin/activate  # Linux/Mac

# 安装 maturin
pip install maturin

# 构建并安装
maturin develop
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
from pi_agent import Agent, Session

# 创建 Agent
agent = Agent(
    api_key="sk-...",
    model="gpt-4o",
    session_path="session.jsonl",
    system_prompt="你是一个有用的助手。",
)

# 注册内置工具
agent.register_builtin_tools()

# 运行对话
agent.run("你好！")

# 获取响应
while True:
    event = agent.next_event()
    if event is None:
        break
    
    if event.event_type == "message_end":
        print(f"助手: {event.content}")
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

## 自定义工具

```python
from pi_agent import Tool, ToolDefinition

class MyTool:
    def definition(self):
        return ToolDefinition(
            name="my_tool",
            description="我的自定义工具",
            parameters='{"type": "object", "properties": {"arg": {"type": "string"}}}'
        )
    
    def execute(self, args):
        return "工具执行结果"

# 注册自定义工具
agent.register_tool(MyTool())
```

## 文件结构

```
pi-agent/
├── python/
│   └── pi_agent/
│       └── __init__.py          # Python 包
├── rust/
│   └── src/                     # Rust 源码
├── examples/
│   ├── complete_demo.py         # 完整示例
│   └── integration.py           # 集成示例
├── test_basic.py                # 基础测试
└── test_llm.py                  # LLM 测试
```

## 更多信息

查看 `examples/` 目录中的示例文件。
