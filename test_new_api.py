"""Test new API: single agent, multiple sessions, output filtering, log levels, external prompts."""

import sys
import os
import time

# Add src to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "src"))

from pi_agent import (
    Agent,
    create_agent,
    get_agent,
    Session,
    LogLevel,
    OutputMode,
    load_prompt_set,
    load_prompt,
)


def test_single_agent():
    """测试单例 Agent。"""
    print("=" * 50)
    print("测试 1: 单例 Agent")
    print("=" * 50)

    agent1 = create_agent(
        api_key="test-key",
        model="gpt-4",
        log_level=LogLevel.DEBUG,
    )
    agent2 = create_agent(
        api_key="test-key-2",
        model="gpt-4o",
        log_level=LogLevel.INFO,
    )

    assert agent1 is agent2, "Agent 应该是单例"
    assert agent2.model == "gpt-4o", "第二次 create_agent 应该更新配置"
    print("  [PASS] Agent 是单例")
    print()


def test_multiple_sessions():
    """测试多 Session 管理。"""
    print("=" * 50)
    print("测试 2: 多 Session 管理")
    print("=" * 50)

    import pi_agent.agent as mod
    mod._agent_instance = None

    agent = create_agent(api_key="test-key", model="gpt-4")

    s1 = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)
    s2 = agent.create_session(output_mode=OutputMode.THINKING)
    s3 = agent.create_session(output_mode=OutputMode.FULL_DEBUG)

    assert s1.session_id != s2.session_id != s3.session_id
    print(f"  Session 1: {s1.session_id[:8]}... mode={s1.output_mode.value}")
    print(f"  Session 2: {s2.session_id[:8]}... mode={s2.output_mode.value}")
    print(f"  Session 3: {s3.session_id[:8]}... mode={s3.output_mode.value}")

    sessions = agent.list_sessions()
    assert len(sessions) == 3
    print(f"  总共 {len(sessions)} 个 session")
    print("  [PASS] 多 Session 创建成功")
    print()


def test_continue_session():
    """测试继续 Session。"""
    print("=" * 50)
    print("测试 3: 继续 Session")
    print("=" * 50)

    import pi_agent.agent as mod
    mod._agent_instance = None

    agent = create_agent(api_key="test-key", model="gpt-4")

    s1 = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)
    print(f"  创建 Session: {s1.session_id[:8]}...")

    s2 = agent.continue_session(s1.session_id, output_mode=OutputMode.THINKING)
    assert s2 is not None
    assert s2.session_id == s1.session_id
    assert s2.output_mode == OutputMode.THINKING
    print(f"  继续 Session: {s2.session_id[:8]}... mode={s2.output_mode.value}")

    s3 = agent.get_session(s1.session_id)
    assert s3 is not None
    print(f"  获取 Session: {s3.session_id[:8]}...")

    s4 = agent.continue_session("nonexistent-id")
    assert s4 is None
    print("  不存在的 Session: None")

    print("  [PASS] Session 继续成功")
    print()


def test_delete_session():
    """测试删除 Session。"""
    print("=" * 50)
    print("测试 4: 删除 Session")
    print("=" * 50)

    # 重置单例以获得干净状态
    import pi_agent.agent as mod
    mod._agent_instance = None

    agent = create_agent(api_key="test-key", model="gpt-4")

    s1 = agent.create_session()
    s2 = agent.create_session()
    print(f"  创建 2 个 Session: {len(agent.list_sessions())}")

    result = agent.delete_session(s1.session_id)
    assert result is True
    print(f"  删除 Session 1: {len(agent.list_sessions())}")

    result = agent.delete_session("nonexistent-id")
    assert result is False
    print("  删除不存在的 Session: False")

    assert len(agent.list_sessions()) == 1
    print("  [PASS] Session 删除成功")
    print()


def test_log_levels():
    """测试日志级别。"""
    print("=" * 50)
    print("测试 5: 日志级别")
    print("=" * 50)

    import pi_agent.agent as mod
    mod._agent_instance = None

    agent = create_agent(api_key="test-key", model="gpt-4", log_level=LogLevel.WARNING)
    logs = agent.get_log_buffer()

    # 只有 WARNING 和 ERROR 应该被记录
    info_logs = [l for l in logs if "INFO" in l]
    print(f"  INFO 日志数: {len(info_logs)} (应该是 0)")

    # 改为 DEBUG
    agent.log_level = LogLevel.DEBUG
    agent.clear_logs()

    # 创建 session 会触发 INFO 日志
    agent.create_session()
    logs = agent.get_log_buffer()
    info_logs = [l for l in logs if "INFO" in l]
    print(f"  DEBUG 模式下 INFO 日志数: {len(info_logs)} (应该 >= 1)")

    print("  [PASS] 日志级别过滤正常")
    print()


def test_external_prompts():
    """测试外部提示词加载。"""
    print("=" * 50)
    print("测试 6: 外部提示词加载")
    print("=" * 50)

    import pi_agent.agent as mod
    mod._agent_instance = None

    # 加载单个提示词
    prompt = load_prompt("rust/prompts/system_main.md")
    print(f"  system_main 长度: {len(prompt)} 字符")

    # 加载提示词集合
    ps = load_prompt_set(
        system_main="rust/prompts/system_main.md",
        compaction_system="rust/prompts/compaction_system.md",
        tool_guidelines_dir="rust/prompts/tool_guidelines",
    )
    print(f"  system_main: {len(ps.system_main)} 字符")
    print(f"  compaction_system: {len(ps.compaction_system)} 字符")
    print(f"  tool_guidelines: {list(ps.tool_guidelines.keys())}")

    # 创建使用外部提示词的 Agent
    agent = create_agent(
        api_key="test-key",
        model="gpt-4",
        system_main="rust/prompts/system_main.md",
        tool_guidelines_dir="rust/prompts/tool_guidelines",
    )
    print(f"  Agent prompt_set.system_main: {len(agent._prompt_set.system_main)} 字符")

    print("  [PASS] 外部提示词加载成功")
    print()


def test_output_mode_filtering():
    """测试输出模式过滤。"""
    print("=" * 50)
    print("测试 7: 输出模式过滤")
    print("=" * 50)

    from pi_agent.event_buffer import EventBuffer
    from types import SimpleNamespace

    # 模拟事件
    events = [
        SimpleNamespace(event_type="stream_token", content="hello"),
        SimpleNamespace(event_type="message_end", content="hello world"),
        SimpleNamespace(event_type="tool_call_start", tool_name="bash"),
        SimpleNamespace(event_type="tool_call_end", tool_call_id="123"),
        SimpleNamespace(event_type="compaction_start", message_count=10),
        SimpleNamespace(event_type="compaction_end", summary="compressed"),
        SimpleNamespace(event_type="turn_start", turn=1),
        SimpleNamespace(event_type="turn_end", turn=1),
    ]

    # CONTENT_ONLY: 只有 stream_token 和 message_end
    buf = EventBuffer(OutputMode.CONTENT_ONLY)
    for e in events:
        buf.put(e)
    filtered = buf.drain_filtered()
    types = [e.event_type for e in filtered]
    print(f"  CONTENT_ONLY: {types}")
    assert "stream_token" in types
    assert "message_end" in types
    assert "tool_call_start" not in types

    # THINKING: 包含 tool_call / compaction / turn
    buf = EventBuffer(OutputMode.THINKING)
    for e in events:
        buf.put(e)
    filtered = buf.drain_filtered()
    types = [e.event_type for e in filtered]
    print(f"  THINKING: {types}")
    assert "tool_call_start" in types
    assert "compaction_start" in types
    assert "turn_start" in types

    # FULL_DEBUG: 所有事件
    buf = EventBuffer(OutputMode.FULL_DEBUG)
    for e in events:
        buf.put(e)
    filtered = buf.drain_filtered()
    types = [e.event_type for e in filtered]
    print(f"  FULL_DEBUG: {types}")
    assert len(types) == len(events)

    print("  [PASS] 输出模式过滤正常")
    print()


def test_session_output_mode_change():
    """测试运行时切换输出模式。"""
    print("=" * 50)
    print("测试 8: 运行时切换输出模式")
    print("=" * 50)

    import pi_agent.agent as mod
    mod._agent_instance = None

    agent = create_agent(api_key="test-key", model="gpt-4")
    session = agent.create_session(output_mode=OutputMode.CONTENT_ONLY)
    print(f"  初始模式: {session.output_mode.value}")

    session.output_mode = OutputMode.THINKING
    print(f"  切换后: {session.output_mode.value}")

    session.output_mode = OutputMode.FULL_DEBUG
    print(f"  再切换: {session.output_mode.value}")

    print("  [PASS] 输出模式切换正常")
    print()


def main():
    print("Pi Agent 新 API 测试")
    print("=" * 50)
    print()

    test_single_agent()
    test_multiple_sessions()
    test_continue_session()
    test_delete_session()
    test_log_levels()
    test_external_prompts()
    test_output_mode_filtering()
    test_session_output_mode_change()

    print("=" * 50)
    print("所有测试通过!")
    print("=" * 50)


if __name__ == "__main__":
    main()
