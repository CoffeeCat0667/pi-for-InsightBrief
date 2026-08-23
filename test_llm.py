"""Test pi_agent with actual LLM calls."""
import sys
import os

# Set your OpenAI API key here or via environment variable
API_KEY = os.environ.get("OPENAI_API_KEY", "sk-your-key-here")

if API_KEY == "sk-your-key-here":
    print("Please set OPENAI_API_KEY environment variable or edit this file.")
    print("Example: set OPENAI_API_KEY=sk-...")
    sys.exit(1)

from pi_agent import PyAgent, PySession

# Create session
session = PySession("gpt-4o", "You are a helpful coding assistant.")

# Create agent
agent = PyAgent(
    api_key=API_KEY,
    model="gpt-4o",
    session_path="~/.pi/sessions/test.jsonl",
    system_prompt="You are a helpful coding assistant.",
    max_turns=5,
    reserve_tokens=4096,
    keep_recent_tokens=2000,
    context_window=16000,
)

# Register built-in tools
agent.register_builtin_tools()

# Run agent
print("Running agent...")
try:
    agent.run("Say hello in one sentence.")
    
    # Process events
    while True:
        event = agent.next_event()
        if event is None:
            break
        
        if event.event_type == "message_end":
            print(f"\nAssistant: {event.content}")
        elif event.event_type == "compaction_end":
            print(f"\nCompacted: {event.summary[:100]}...")
            
except Exception as e:
    print(f"Error: {e}")

print("\nDone!")
