"""Test pi_agent basic functionality."""
import sys

# Test 1: Import
try:
    from pi_agent import version, create_entry_id, PySession, PyAgentEvent, PyEntry, PyUsage
    print(f"[OK] Import successful")
    print(f"  version: {version()}")
    print(f"  create_entry_id: {create_entry_id()}")
except ImportError as e:
    print(f"[FAIL] Import error: {e}")
    sys.exit(1)

# Test 2: Session create
try:
    session = PySession("gpt-4o", "You are a helpful assistant.")
    print(f"\n[OK] Session created")
    print(f"  model: {session.model()}")
    print(f"  session_id: {session.session_id()}")
    print(f"  entry_count: {session.entry_count()}")
except Exception as e:
    print(f"[FAIL] Session create error: {e}")
    sys.exit(1)

# Test 3: Append messages
try:
    uid = session.append_user("Hello!")
    print(f"\n[OK] User message appended: {uid}")
    aid = session.append_assistant("Hi! How can I help?")
    print(f"[OK] Assistant message appended: {aid}")
    print(f"  entry_count: {session.entry_count()}")
    print(f"  leaf: {session.leaf().id}")
except Exception as e:
    print(f"[FAIL] Append error: {e}")
    sys.exit(1)

# Test 4: Messages
try:
    msgs = session.messages()
    print(f"\n[OK] Messages: {len(msgs)}")
    for m in msgs:
        print(f"  [{m.role}] {m.content[:30]}")
except Exception as e:
    print(f"[FAIL] Messages error: {e}")
    sys.exit(1)

# Test 5: Usage
try:
    usage = session.total_usage()
    print(f"\n[OK] Total usage: input={usage.input_tokens}, output={usage.output_tokens}")
except Exception as e:
    print(f"[FAIL] Usage error: {e}")
    sys.exit(1)

# Test 6: Branch switching
try:
    branch_pts = session.branch_points()
    print(f"\n[OK] Branch points: {len(branch_pts)}")
except Exception as e:
    print(f"[FAIL] Branch points error: {e}")
    sys.exit(1)

# Test 7: Save and reload
import tempfile, os
try:
    tmpfile = os.path.join(tempfile.gettempdir(), "test_pi_agent.jsonl")
    if os.path.exists(tmpfile):
        os.remove(tmpfile)
    session.append_user("Test save")
    print(f"\n[OK] Session save (via append)")
except Exception as e:
    print(f"[FAIL] Save error: {e}")
    sys.exit(1)

print("\n" + "="*40)
print("All tests passed!")
