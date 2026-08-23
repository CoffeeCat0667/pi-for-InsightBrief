"""
Pi Agent 完整使用示例
====================

功能演示：
1. 创建会话
2. 运行 Agent 对话
3. 工具调用
4. 上下文压缩
5. 分支管理
"""

import os
import sys
import time


from pi_agent import Agent, Session, AgentEvent


def demo_basic_chat():
    """基础对话示例"""
    print("=" * 50)
    print("示例 1: 基础对话")
    print("=" * 50)
    
    # 创建会话
    session = Session("gpt-4o", "你是一个有用的编程助手。")
    
    # 创建 Agent - 支持自定义 base_url, api_key, model
    agent = Agent(
        api_key=API_KEY,
        model="gpt-4o",  # 可以使用任何 OpenAI 兼容模型
        session_path=".venv/test_session.jsonl",
        base_url=None,  # None 使用默认 OpenAI API，或设置自定义 URL
        system_prompt="你是一个有用的编程助手。",
        max_turns=5,
        reserve_tokens=4096,
        keep_recent_tokens=2000,
        context_window=16000,
    )
    
    # 注册内置工具
    agent.register_builtin_tools()
    
    # 运行对话
    print("用户: 用一句话解释什么是递归？")
    agent.run("用一句话解释什么是递归？")
    
    # 获取响应
    while True:
        event = agent.next_event()
        if event is None:
            break
        
        if event.event_type == "message_end":
            print(f"助手: {event.content}")
        elif event.event_type == "tool_call_start":
            print(f"  [工具调用]")
    
    print()


def demo_custom_api():
    """自定义 API 交互式对话 - 多轮对话 + 上下文记忆 + 工具调用 + 流式传输"""
    import os
    print("=" * 50)
    print("Pi Agent 交互式对话")
    print("=" * 50)
    
    # 使用脚本所在目录作为基准
    script_dir = os.path.dirname(os.path.abspath(__file__))
    session_path = os.path.join(script_dir, ".venv", "interactive_session.jsonl")
    os.makedirs(os.path.dirname(session_path), exist_ok=True)
    
    # 创建 Agent
    agent = Agent(
        api_key="sk-0917aed21cc2a74efa7af30c3cee4a4736bfbf3119a7de116bd5a629f6c7b208",
        model="gpt-5.6-luna",
        session_path=session_path,
        base_url="https://sub2api.aimeta.store/v1",
        system_prompt="你是一个有用的编程助手，可以使用工具帮助用户解决问题，并能记住之前的对话内容。",
        max_turns=10,
        reserve_tokens=4096,
        keep_recent_tokens=2000,
        context_window=16000,
    )
    
    # 注册内置工具
    agent.register_builtin_tools()
    
    print("Agent 已就绪")
    print(f"  - 模型: gpt-5.6-luna")
    print(f"  - 工具: ls, read, write, edit, grep, find, bash")
    print(f"  - 输入 'quit' 或 'exit' 退出")
    print()
    
    # 交互式循环
    turn = 0
    while True:
        try:
            # 获取用户输入
            user_msg = input("你: ").strip()
            
            # 检查退出命令
            if user_msg.lower() in ('quit', 'exit', 'q'):
                print("\n再见！")
                break
            
            # 跳过空输入
            if not user_msg:
                continue
            
            turn += 1
            print(f"助手: ", end="", flush=True)
            
            # 运行对话
            start_time = time.time()
            agent.run(user_msg)
            
            # 获取响应（流式输出）
            tool_calls_made = []
            token_count = 0
            while True:
                event = agent.next_event()
                if event is None:
                    break
                if event.event_type == "stream_token":
                    token_count += 1
                    print(event.content, end="", flush=True)
                elif event.event_type == "message_end":
                    elapsed = time.time() - start_time
                    if token_count > 0:
                        print(f"  [{token_count} tokens, {elapsed:.1f}s]")
                    else:
                        print()
                elif event.event_type == "tool_call_start":
                    tool_calls_made.append(event.tool_name)
                    print(f"\n  [工具: {event.tool_name}]", end="", flush=True)
                elif event.event_type == "tool_call_end":
                    print(f" -> 完成")
            
            if tool_calls_made:
                print(f"  (使用了: {', '.join(tool_calls_made)})")
            print()
            
        except KeyboardInterrupt:
            print("\n\n再见！")
            break
        except EOFError:
            print("\n\n再见！")
            break
    
    # 显示会话统计
    print("=" * 50)
    print(f"会话结束 - 共 {turn} 轮对话")
    print("=" * 50)


def demo_azure_openai():
    """Azure OpenAI 示例"""
    print("=" * 50)
    print("示例 3: Azure OpenAI")
    print("=" * 50)
    
    # Azure OpenAI 配置
    AZURE_API_KEY = os.environ.get("AZURE_OPENAI_API_KEY", "")
    AZURE_ENDPOINT = os.environ.get("AZURE_OPENAI_ENDPOINT", "")
    
    if not AZURE_API_KEY or not AZURE_ENDPOINT:
        print("跳过: 请设置 AZURE_OPENAI_API_KEY 和 AZURE_OPENAI_ENDPOINT")
        print()
        return
    
    # 创建 Agent - 使用 Azure OpenAI
    agent = Agent(
        api_key=AZURE_API_KEY,
        model="gpt-4",  # Azure 模型名称
        session_path=".venv/test_azure_session.jsonl",
        base_url=f"{AZURE_ENDPOINT}/openai/deployments/your-deployment",
        system_prompt="你是一个有用的编程助手。",
        max_turns=5,
    )
    
    # 注册内置工具
    agent.register_builtin_tools()
    
    print("Agent 已配置为使用 Azure OpenAI")
    print(f"  - endpoint: {AZURE_ENDPOINT}")
    print(f"  - model: gpt-4")
    print()


def demo_local_model():
    """本地模型示例（如 Ollama, vLLM 等）"""
    print("=" * 50)
    print("示例 4: 本地模型")
    print("=" * 50)
    
    # 本地模型配置（例如 Ollama）
    LOCAL_API_KEY = "ollama"  # Ollama 不需要真实 API key
    LOCAL_BASE_URL = "http://localhost:11434/v1"  # Ollama 默认端点
    
    # 创建 Agent - 使用本地模型
    agent = Agent(
        api_key=LOCAL_API_KEY,
        model="llama3",  # 本地模型名称
        session_path=".venv/test_local_session.jsonl",
        base_url=LOCAL_BASE_URL,
        system_prompt="你是一个有用的编程助手。",
        max_turns=5,
    )
    
    # 注册内置工具
    agent.register_builtin_tools()
    
    print("Agent 已配置为使用本地模型")
    print(f"  - base_url: {LOCAL_BASE_URL}")
    print(f"  - model: llama3")
    print()


def demo_session_management():
    """会话管理示例"""
    print("=" * 50)
    print("示例 5: 会话管理")
    print("=" * 50)
    
    # 创建会话
    session = Session("gpt-4o", "你是一个有用的编程助手。")
    
    # 添加消息
    uid = session.append_user("你好！")
    aid = session.append_assistant("你好！有什么我可以帮助你的吗？")
    
    # 查看会话信息
    print(f"会话 ID: {session.session_id()}")
    print(f"模型: {session.model()}")
    print(f"条目数: {session.entry_count()}")
    
    # 查看消息
    messages = session.messages()
    print(f"\n消息数量: {len(messages)}")
    for msg in messages:
        print(f"  [{msg.role}] {msg.content[:40]}...")
    
    # 查看分支
    branch_points = session.branch_points()
    print(f"\n分支点: {len(branch_points)}")
    
    # 查看 token 使用
    usage = session.total_usage()
    print(f"Token 使用: {usage.total()}")
    
    print()


def main():
    """运行所有示例"""
    print("Pi Agent 完整功能演示")
    print("=" * 50)
    print()
    
    try:
        demo_custom_api()
        
        print("=" * 50)
        print("所有示例运行完成！")
        print("=" * 50)
        
    except Exception as e:
        print(f"运行错误: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()
