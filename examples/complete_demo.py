"""
Pi Agent 完整使用示例（异步版）
=============================

功能演示：
1. 异步对话
2. 流式输出
3. 多 Session 并发
4. 取消任务
"""

import asyncio
import os
import time

from pi_agent import OutputMode, create_agent


async def demo_async_chat():
    """异步对话示例"""
    print("=" * 50)
    print("示例 1: 异步对话")
    print("=" * 50)

    agent = create_agent(
        api_key=os.environ.get("OPENAI_API_KEY", "test-key"),
        model="gpt-4o",
        sessions={},
    )

    session = agent.create_session(
        session_id="demo-async",
        output_mode=OutputMode.CONTENT_ONLY,
    )

    print("用户: 用一句话解释什么是递归？")
    await agent.run_async(session.session_id, "用一句话解释什么是递归？")

    response = await agent.wait_response_async(session.session_id)
    print(f"助手: {response}")
    print()


async def demo_streaming():
    """流式输出示例"""
    print("=" * 50)
    print("示例 2: 流式输出")
    print("=" * 50)

    agent = create_agent(
        api_key=os.environ.get("OPENAI_API_KEY", "test-key"),
        model="gpt-4o",
        sessions={},
    )

    session = agent.create_session(
        session_id="demo-streaming",
        output_mode=OutputMode.CONTENT_ONLY,
    )

    print("用户: 用一句话解释什么是递归？")
    await agent.run_async(session.session_id, "用一句话解释什么是递归？")

    print("助手: ", end="", flush=True)
    async for event in session.events():
        if event.event_type == "stream_token":
            print(event.content, end="", flush=True)
    print()
    print()


async def demo_concurrent():
    """并发多 Session 示例"""
    print("=" * 50)
    print("示例 3: 并发多 Session")
    print("=" * 50)

    agent = create_agent(
        api_key=os.environ.get("OPENAI_API_KEY", "test-key"),
        model="gpt-4o",
        sessions={},
    )

    s1 = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)
    s2 = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)

    async def run_and_print(session, prompt, label):
        start = time.time()
        await agent.run_async(session.session_id, prompt)
        async for event in session.events():
            if event.event_type == "stream_token":
                print(f"{label}: {event.content}", end="", flush=True)
        elapsed = time.time() - start
        print(f"  [{elapsed:.1f}s]")

    print("同时运行两个 Session...")
    await asyncio.gather(
        run_and_print(s1, "什么是递归？", "S1"),
        run_and_print(s2, "什么是闭包？", "S2"),
    )
    print()


async def demo_thinking_mode():
    """思考模式示例（显示工具调用和压缩事件）"""
    print("=" * 50)
    print("示例 4: 思考模式")
    print("=" * 50)

    agent = create_agent(
        api_key=os.environ.get("OPENAI_API_KEY", "test-key"),
        model="gpt-4o",
        sessions={},
    )

    session = agent.create_session(
        session_id="demo-thinking",
        output_mode=OutputMode.THINKING,
    )

    print("用户: 列出当前目录的文件")
    await agent.run_async(session.session_id, "列出当前目录的文件")

    async for event in session.events():
        if event.event_type == "stream_token":
            print(event.content, end="", flush=True)
        elif event.event_type == "tool_call_start":
            print(f"\n  [工具调用: {event.tool_name}]", end="", flush=True)
        elif event.event_type == "tool_call_end":
            print(" -> 完成")
        elif event.event_type == "turn_start":
            print(f"\n  [Turn {event.turn}]", end="", flush=True)
    print()
    print()


async def main():
    """运行所有示例"""
    print("Pi Agent 异步功能演示")
    print("=" * 50)
    print()

    try:
        await demo_async_chat()
        await demo_streaming()
        await demo_concurrent()
        await demo_thinking_mode()

        print("=" * 50)
        print("所有示例运行完成！")
        print("=" * 50)

    except Exception as e:
        print(f"运行错误: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    asyncio.run(main())
