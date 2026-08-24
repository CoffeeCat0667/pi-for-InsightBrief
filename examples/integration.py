"""
Pi Agent 集成示例
================

在你的项目中使用 Pi Agent:

1. 基础使用
2. 自定义 API 端点
3. 异步用法（推荐）
4. 并发多 Session
5. 自定义工具
"""

import asyncio
import os

from pi_agent import OutputMode, create_agent


# ============================================
# 1. 基础使用（同步）
# ============================================

def basic_sync():
    """使用同步 API"""

    agent = create_agent(
        api_key=os.environ.get("OPENAI_API_KEY", "test-key"),
        model="gpt-4o",
        sessions={},
    )

    session = agent.create_session(
        session_id="integration-basic",
        output_mode=OutputMode.CONTENT_ONLY,
    )

    agent.run(session.session_id, "你好！")

    while True:
        event = session.next_event()
        if event is None:
            break
        if event.event_type == "message_end":
            print(f"助手: {event.content}")


# ============================================
# 2. 自定义 API 端点
# ============================================

def custom_api():
    """使用自定义 API 端点（如第三方服务）"""

    agent = create_agent(
        api_key="your-api-key",
        model="gpt-4o",
        base_url="https://api.your-provider.com/v1",
        sessions={},
    )

    session = agent.create_session(
        session_id="integration-custom-api",
        output_mode=OutputMode.CONTENT_ONLY,
    )

    agent.run(session.session_id, "你好！")


# ============================================
# 3. 异步用法（推荐）
# ============================================

async def async_example():
    """使用异步 API，不阻塞事件循环"""

    agent = create_agent(
        api_key=os.environ.get("OPENAI_API_KEY", "test-key"),
        model="gpt-4o",
        sessions={},
    )

    session = agent.create_session(
        session_id="integration-async",
        output_mode=OutputMode.CONTENT_ONLY,
    )

    # 异步运行
    await agent.run_async(session.session_id, "用一句话解释递归")

    # 异步迭代事件
    async for event in session.events():
        if event.event_type == "stream_token":
            print(event.content, end="", flush=True)
    print()

    # 或者直接获取完整回复
    response = await agent.wait_response_async(session.session_id)
    print(f"完整回复: {response}")


# ============================================
# 4. 并发多 Session
# ============================================

async def concurrent_sessions():
    """多个 Session 并发运行"""

    agent = create_agent(
        api_key=os.environ.get("OPENAI_API_KEY", "test-key"),
        model="gpt-4o",
        sessions={},
    )

    s1 = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)
    s2 = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)

    # 两个 Session 并发运行
    async def run_session(session, prompt, label):
        await agent.run_async(session.session_id, prompt)
        async for event in session.events():
            if event.event_type == "stream_token":
                print(f"{label}: {event.content}", end="", flush=True)
        print()

    await asyncio.gather(
        run_session(s1, "什么是递归？", "S1"),
        run_session(s2, "什么是闭包？", "S2"),
    )


# ============================================
# 5. 自定义工具
# ============================================

class MyCalculatorTool:
    """自定义计算器工具"""

    def definition(self):
        from pi_agent import PyToolDefinition
        return PyToolDefinition(
            name="calculator",
            description="执行数学计算",
            parameters='{"type": "object", "properties": {"expression": {"type": "string"}}, "required": ["expression"]}'
        )

    def execute(self, args):
        import json
        data = json.loads(args) if isinstance(args, str) else args
        expression = data.get("expression", "")

        try:
            result = eval(expression)
            return str(result)
        except Exception as e:
            return f"计算错误: {e}"


# ============================================
# 主函数
# ============================================

if __name__ == "__main__":
    print("Pi Agent 集成示例")
    print("=" * 50)
    print()
    print("选择要运行的示例:")
    print("1. basic_sync()           - 同步基础用法")
    print("2. custom_api()           - 自定义 API 端点")
    print("3. async_example()        - 异步用法")
    print("4. concurrent_sessions()  - 并发多 Session")

    choice = input("\n输入选择 (1-4): ").strip()

    if choice == "1":
        basic_sync()
    elif choice == "2":
        custom_api()
    elif choice == "3":
        asyncio.run(async_example())
    elif choice == "4":
        asyncio.run(concurrent_sessions())
    else:
        print("无效选择")
