"""测试流式传输"""
import sys
import time
from pi_agent import Agent

# 创建 Agent
agent = Agent(
    api_key="sk-0917aed21cc2a74efa7af30c3cee4a4736bfbf3119a7de116bd5a629f6c7b208",
    model="gpt-5.6-luna",
    session_path=".venv/stream_test.jsonl",
    base_url="https://sub2api.aimeta.store/v1",
    system_prompt="You are a helpful assistant.",
    max_turns=3,
)

print("测试流式传输")
print("=" * 50)

# 测试流式输出
print("你: 说一个长句子，至少50个字")
print("助手: ", end="", flush=True)

start_time = time.time()
agent.run("说一个长句子，至少50个字")

token_count = 0
while True:
    event = agent.next_event()
    if event is None:
        break
    if event.event_type == "stream_token":
        token_count += 1
        elapsed = time.time() - start_time
        # 输出 token 并在 stderr 记录时间
        print(event.content, end="", flush=True)
        sys.stderr.write(f"[token {token_count}] +{elapsed:.3f}s\n")
        sys.stderr.flush()
    elif event.event_type == "message_end":
        print()

elapsed = time.time() - start_time
print(f"\n{'=' * 50}")
print(f"统计: {token_count} 个 token, {elapsed:.2f} 秒")
print(f"平均: {token_count/elapsed:.1f} tokens/sec")
print("=" * 50)
