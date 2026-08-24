"""Test async API: run_async, events(), wait_response_async, cancellation."""

import asyncio
import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "src"))


def test_async_session_creation():
    """Test that sessions can be created and async methods exist."""
    from pi_agent import create_agent, OutputMode

    agent = create_agent(api_key="test-key", model="gpt-4")
    session = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)

    # Verify async methods exist
    assert hasattr(session, 'run_async'), "Session should have run_async()"
    assert hasattr(session, 'events'), "Session should have events()"
    assert hasattr(session, 'wait_response_async'), "Session should have wait_response_async()"
    assert hasattr(session, 'cancel'), "Session should have cancel()"
    assert hasattr(session, 'is_running'), "Session should have is_running property"

    assert hasattr(agent, 'run_async'), "Agent should have run_async()"
    assert hasattr(agent, 'wait_response_async'), "Agent should have wait_response_async()"

    print("  [PASS] Async API methods exist")


def test_async_session_is_running():
    """Test is_running property."""
    from pi_agent import create_agent

    agent = create_agent(api_key="test-key", model="gpt-4")
    session = agent.create_session()

    assert not session.is_running, "Session should not be running initially"
    print("  [PASS] is_running property works")


def test_sync_api_still_works():
    """Test that sync API still works after async changes."""
    from pi_agent import create_agent, OutputMode

    import pi_agent.agent as mod
    mod._agent_instance = None

    agent = create_agent(api_key="test-key", model="gpt-4")
    session = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)

    # Verify sync methods still exist
    assert hasattr(session, 'run'), "Session should have run()"
    assert hasattr(session, 'next_event'), "Session should have next_event()"
    assert hasattr(session, 'wait_response'), "Session should have wait_response()"
    assert hasattr(session, 'wait_response_stream'), "Session should have wait_response_stream()"

    print("  [PASS] Sync API methods still exist")


def test_event_buffer_async():
    """Test EventBuffer async_get method."""
    from pi_agent.event_buffer import EventBuffer
    from pi_agent.types import OutputMode
    from types import SimpleNamespace

    buf = EventBuffer(OutputMode.CONTENT_ONLY)
    event = SimpleNamespace(event_type="stream_token", content="hello")
    buf.put(event)

    # Test sync get
    result = buf.get()
    assert result is not None
    assert result.content == "hello"

    # Test async get
    async def test_async():
        buf.put(event)
        result = await buf.async_get()
        assert result is not None
        assert result.content == "hello"

    asyncio.run(test_async())
    print("  [PASS] EventBuffer async_get works")


def test_native_agent_async_api():
    """Test native PyAgent has the new async-related methods."""
    from pi_agent.pi_agent import PyAgent

    agent = PyAgent(api_key="test-key", model="gpt-4", session_id="test-async")

    # Verify new methods exist
    assert hasattr(agent, 'is_running'), "PyAgent should have is_running()"
    assert hasattr(agent, 'cancel'), "PyAgent should have cancel()"
    assert hasattr(agent, 'wait_done'), "PyAgent should have wait_done()"

    assert not agent.is_running(), "Agent should not be running initially"
    print("  [PASS] Native PyAgent async API works")


def main():
    print("Pi Agent Async API Tests")
    print("=" * 50)
    print()

    test_async_session_creation()
    test_async_session_is_running()
    test_sync_api_still_works()
    test_event_buffer_async()
    test_native_agent_async_api()

    print("=" * 50)
    print("All async API tests passed!")
    print("=" * 50)


if __name__ == "__main__":
    main()
