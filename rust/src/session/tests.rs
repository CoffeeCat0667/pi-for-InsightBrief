use std::fs;

use crate::session::{Entry, SessionStore};

#[test]
fn test_session_create_and_append() {
    let temp_dir = std::env::temp_dir().join("pi_agent_test");
    fs::create_dir_all(&temp_dir).unwrap();
    let test_file = temp_dir.join("test_session.jsonl");

    // Clean up before test
    let _ = fs::remove_file(&test_file);

    // Create a new session with a specific path
    let mut store = SessionStore::new(&test_file);
    store.header = Some(crate::session::Header {
        session_id: uuid::Uuid::new_v4(),
        created_at: chrono::Utc::now(),
        model: "gpt-4o".to_string(),
        system_prompt: Some("You are a helpful assistant.".to_string()),
    });

    // Add system prompt entry
    let system_entry = Entry::new_system("You are a helpful assistant.".to_string());
    store.roots.push(system_entry.id);
    store.leaf = Some(system_entry.id);
    store.entries.insert(system_entry.id, system_entry);

    // Verify initial state
    assert!(store.header().is_some());
    assert_eq!(store.header().unwrap().model, "gpt-4o");
    assert_eq!(store.branch().len(), 1); // System prompt entry

    // Append a user message
    let user_entry = Entry::new_user("Hello, assistant!".to_string(), None);
    let user_id = store.append(user_entry).unwrap();
    assert_eq!(store.branch().len(), 2);
    assert_eq!(store.leaf().unwrap().id, user_id);

    // Append an assistant message
    let assistant_entry = Entry::new_assistant("Hello! How can I help you?".to_string(), None);
    let assistant_id = store.append(assistant_entry).unwrap();
    assert_eq!(store.branch().len(), 3);
    assert_eq!(store.leaf().unwrap().id, assistant_id);

    // Verify the file was created
    assert!(test_file.exists());

    // Load the session from file
    let loaded_store = SessionStore::load(&test_file).unwrap();
    assert_eq!(loaded_store.branch().len(), 3);
    assert!(loaded_store.header().is_some());
    assert_eq!(loaded_store.header().unwrap().model, "gpt-4o");
    assert_eq!(loaded_store.entries.len(), 3);

    // Clean up
    let _ = fs::remove_file(&test_file);
    let _ = fs::remove_dir(&temp_dir);
}

#[test]
fn test_branch_switching() {
    let temp_dir = std::env::temp_dir().join("pi_agent_test_branch");
    fs::create_dir_all(&temp_dir).unwrap();
    let test_file = temp_dir.join("test_branch.jsonl");

    // Clean up before test
    let _ = fs::remove_file(&test_file);

    // Create a new session
    let mut store = SessionStore::new(&test_file);
    store.header = Some(crate::session::Header {
        session_id: uuid::Uuid::new_v4(),
        created_at: chrono::Utc::now(),
        model: "gpt-4o".to_string(),
        system_prompt: None,
    });

    // Append initial messages
    let entry1 = Entry::new_user("Message 1".to_string(), None);
    let id1 = store.append(entry1).unwrap();

    let entry2 = Entry::new_user("Message 2".to_string(), None);
    let _id2 = store.append(entry2).unwrap();

    let entry3 = Entry::new_user("Message 3".to_string(), None);
    let id3 = store.append(entry3).unwrap();

    // Verify we have 3 messages
    assert_eq!(store.branch().len(), 3);

    // Switch to the first message (branch from there)
    store.switch_branch(id1).unwrap();
    assert_eq!(store.leaf().unwrap().id, id1);
    assert_eq!(store.branch().len(), 1);

    // Switch back to the third message
    store.switch_branch(id3).unwrap();
    assert_eq!(store.leaf().unwrap().id, id3);
    assert_eq!(store.branch().len(), 3);

    // Clean up
    let _ = fs::remove_file(&test_file);
    let _ = fs::remove_dir(&temp_dir);
}

#[test]
fn test_branch_summary_generation() {
    let temp_dir = std::env::temp_dir().join("pi_agent_test_summary");
    fs::create_dir_all(&temp_dir).unwrap();
    let test_file = temp_dir.join("test_summary.jsonl");

    // Clean up before test
    let _ = fs::remove_file(&test_file);

    // Create a new session
    let mut store = SessionStore::new(&test_file);
    store.header = Some(crate::session::Header {
        session_id: uuid::Uuid::new_v4(),
        created_at: chrono::Utc::now(),
        model: "gpt-4o".to_string(),
        system_prompt: None,
    });

    // Append some messages
    let entry1 = Entry::new_user("Message 1".to_string(), None);
    let id1 = store.append(entry1).unwrap();

    let entry2 = Entry::new_user("Message 2".to_string(), None);
    let _id2 = store.append(entry2).unwrap();

    // Generate a summary for the branch ending at entry1
    let summary = store.generate_branch_summary(id1);
    assert!(summary.is_some());
    let summary = summary.unwrap();
    assert!(summary.goal.contains("Abandoned branch"));
    assert!(summary.goal.contains("1 entries"));

    // Clean up
    let _ = fs::remove_file(&test_file);
    let _ = fs::remove_dir(&temp_dir);
}

#[test]
fn test_in_memory_session_round_trip() {
    let mut store = SessionStore::from_data(serde_json::json!({
        "header": {
            "session_id": uuid::Uuid::new_v4(),
            "created_at": chrono::Utc::now(),
            "model": "gpt-4o",
            "system_prompt": null
        },
        "entries": [],
        "compactions": []
    }))
    .unwrap();

    let user_id = store
        .append(Entry::new_user("你好".to_string(), None))
        .unwrap();
    let assistant_id = store
        .append(Entry::new_assistant("你好，有什么可以帮你？".to_string(), None))
        .unwrap();
    let data = store.to_data();

    assert_eq!(data["leaf"], assistant_id.to_string());
    assert_eq!(data["entries"].as_array().unwrap().len(), 2);
    assert!(!store.path.exists());

    let loaded = SessionStore::from_data(data).unwrap();
    assert_eq!(loaded.leaf().unwrap().id, assistant_id);
    assert_eq!(loaded.branch().first().unwrap().id, user_id);
    assert_eq!(loaded.branch().len(), 2);
}
