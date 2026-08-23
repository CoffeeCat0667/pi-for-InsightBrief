"""Test Agent with custom parameters."""
import sys

# Test 1: Import
try:
    from pi_agent import PyAgent as Agent, PySession as Session
    print("[OK] Import successful")
except ImportError as e:
    print(f"[FAIL] Import error: {e}")
    sys.exit(1)

# Test 2: Agent with default parameters
try:
    agent = Agent(
        api_key="test-key",
        model="gpt-4o",
        session_id="test-default",
    )
    print("[OK] Agent created with default parameters")
except Exception as e:
    print(f"[FAIL] Agent create error: {e}")
    sys.exit(1)

# Test 3: Agent with custom base_url
try:
    agent = Agent(
        api_key="test-key",
        model="custom-model",
        session_id="test-custom",
        base_url="https://custom-api.example.com/v1",
    )
    print("[OK] Agent created with custom base_url")
except Exception as e:
    print(f"[FAIL] Agent create with base_url error: {e}")
    sys.exit(1)

# Test 4: Agent with all parameters
try:
    agent = Agent(
        api_key="test-key",
        model="my-model",
        session_id="test-all",
        base_url="https://my-api.com/v1",
        system_prompt="You are a helper.",
        max_turns=10,
        reserve_tokens=8192,
        keep_recent_tokens=4000,
        context_window=32000,
    )
    print("[OK] Agent created with all parameters")
    print(f"  - api_key: test-key")
    print(f"  - model: my-model")
    print(f"  - base_url: https://my-api.com/v1")
    print(f"  - max_turns: 10")
    print(f"  - reserve_tokens: 8192")
    print(f"  - keep_recent_tokens: 4000")
    print(f"  - context_window: 32000")
except Exception as e:
    print(f"[FAIL] Agent create with all params error: {e}")
    sys.exit(1)

# Test 5: Register tools
try:
    agent.register_builtin_tools()
    print("[OK] Built-in tools registered")
except Exception as e:
    print(f"[FAIL] Register tools error: {e}")
    sys.exit(1)

print("\n" + "="*40)
print("All Agent parameter tests passed!")
