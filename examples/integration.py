"""
Pi Agent 集成示例
================

在你的项目中使用 Pi Agent:

1. 基础使用
2. 自定义 API 端点
3. Azure OpenAI
4. 本地模型
5. 自定义工具
"""

import os
from pi_agent import Agent, Session, AgentEvent, Tool, ToolDefinition


# ============================================
# 1. 基础使用（OpenAI API）
# ============================================

def basic_openai():
    """使用 OpenAI API"""
    
    agent = Agent(
        api_key=os.environ["OPENAI_API_KEY"],
        model="gpt-4o",
        session_path="session.jsonl",
        system_prompt="你是一个有用的助手。",
    )
    
    agent.register_builtin_tools()
    agent.run("你好！")
    
    while True:
        event = agent.next_event()
        if event is None:
            break
        if event.event_type == "message_end":
            print(f"助手: {event.content}")


# ============================================
# 2. 自定义 API 端点
# ============================================

def custom_api():
    """使用自定义 API 端点（如第三方服务）"""
    
    agent = Agent(
        api_key="your-api-key",
        model="gpt-4o",
        session_path="session.jsonl",
        base_url="https://api.your-provider.com/v1",  # 自定义端点
        system_prompt="你是一个有用的助手。",
    )
    
    agent.register_builtin_tools()
    agent.run("你好！")


# ============================================
# 3. Azure OpenAI
# ============================================

def azure_openai():
    """使用 Azure OpenAI"""
    
    agent = Agent(
        api_key=os.environ["AZURE_OPENAI_API_KEY"],
        model="gpt-4",  # Azure 部署名称
        session_path="session.jsonl",
        base_url=f"{os.environ['AZURE_OPENAI_ENDPOINT']}/openai/deployments/your-deployment",
        system_prompt="你是一个有用的助手。",
    )
    
    agent.register_builtin_tools()
    agent.run("你好！")


# ============================================
# 4. 本地模型（Ollama, vLLM 等）
# ============================================

def local_model_ollama():
    """使用 Ollama 本地模型"""
    
    agent = Agent(
        api_key="ollama",  # Ollama 不需要真实 key
        model="llama3",  # 本地模型名称
        session_path="session.jsonl",
        base_url="http://localhost:11434/v1",  # Ollama 默认端点
        system_prompt="你是一个有用的助手。",
    )
    
    agent.register_builtin_tools()
    agent.run("你好！")


def local_model_vllm():
    """使用 vLLM 本地模型"""
    
    agent = Agent(
        api_key="vllm",  # vLLM 不需要真实 key
        model="meta-llama/Llama-3-8B-Instruct",
        session_path="session.jsonl",
        base_url="http://localhost:8000/v1",  # vLLM 默认端点
        system_prompt="你是一个有用的助手。",
    )
    
    agent.register_builtin_tools()
    agent.run("你好！")


# ============================================
# 5. 自定义工具
# ============================================

class MyCalculatorTool:
    """自定义计算器工具"""
    
    def definition(self):
        return ToolDefinition(
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


def custom_tool_usage():
    """使用自定义工具"""
    
    agent = Agent(
        api_key=os.environ["OPENAI_API_KEY"],
        model="gpt-4o",
        session_path="session.jsonl",
    )
    
    agent.register_builtin_tools()
    agent.register_tool(MyCalculatorTool())
    
    agent.run("计算 2 + 3 * 4")
    
    while True:
        event = agent.next_event()
        if event is None:
            break
        if event.event_type == "message_end":
            print(f"结果: {event.content}")


# ============================================
# 会话管理
# ============================================

def session_management():
    """会话持久化和恢复"""
    
    session = Session("gpt-4o", "你是一个有用的助手。")
    
    session.append_user("问题1")
    session.append_assistant("回答1")
    
    messages = session.messages()
    print(f"会话中有 {len(messages)} 条消息")
    
    branch_points = session.branch_points()
    print(f"有 {len(branch_points)} 个分支点")


# ============================================
# 主函数
# ============================================

if __name__ == "__main__":
    print("Pi Agent 集成示例")
    print("=" * 50)
    print()
    print("选择要运行的示例:")
    print("1. basic_openai()        - OpenAI API")
    print("2. custom_api()          - 自定义 API 端点")
    print("3. azure_openai()        - Azure OpenAI")
    print("4. local_model_ollama()  - Ollama 本地模型")
    print("5. local_model_vllm()    - vLLM 本地模型")
    print("6. custom_tool_usage()   - 自定义工具")
    print("7. session_management()  - 会话管理")
    
    choice = input("\n输入选择 (1-7): ").strip()
    
    if choice == "1":
        basic_openai()
    elif choice == "2":
        custom_api()
    elif choice == "3":
        azure_openai()
    elif choice == "4":
        local_model_ollama()
    elif choice == "5":
        local_model_vllm()
    elif choice == "6":
        custom_tool_usage()
    elif choice == "7":
        session_management()
    else:
        print("无效选择")
