"""
Example usage of Pi Agent.

This example demonstrates how to use the Pi Agent to have a conversation
with an LLM and manage the session.
"""

from pi_agent import Agent, Session, AgentEvent


def main():
    # Create a new session
    session = Session.new("gpt-4o", system_prompt="You are a helpful coding assistant.")

    # Create an agent
    agent = Agent(
        api_key="sk-your-api-key-here",
        model="gpt-4o",
        session=session,
        max_turns=10,
        reserve_tokens=16384,
        keep_recent_tokens=20000,
        context_window=128000,
    )

    # Run the agent with a prompt
    print("Starting conversation...")
    agent.run("Hello! Can you help me write a Python function to calculate fibonacci numbers?")

    # Process events
    while True:
        event = agent.next_event()
        if event is None:
            break

        if event.event_type == "message_end":
            print(f"Assistant: {event.content}")
        elif event.event_type == "tool_call_start":
            print(f"Tool call: {event.tool_call.name}")
        elif event.event_type == "tool_call_end":
            print(f"Tool result: {event.result.output[:100]}...")
        elif event.event_type == "compaction_start":
            print("Compacting context...")
        elif event.event_type == "compaction_end":
            print(f"Compaction summary: {event.summary[:100]}...")

    # Show session info
    print(f"\nSession model: {session.model()}")
    print(f"Total entries: {session.entry_count()}")
    print(f"Total usage: {session.total_usage().total()} tokens")

    # Show branch points
    branch_points = session.branch_points()
    if branch_points:
        print(f"\nBranch points: {len(branch_points)}")
        for bp in branch_points:
            print(f"  - {bp.parent_id} has {len(bp.children)} children")


if __name__ == "__main__":
    main()
