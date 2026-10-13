use auxidio_harness::memory;
use serde_json::json;

#[test]
fn migrations_are_idempotent() {
    let conn = memory::open_memory_in_memory().unwrap();
    memory::migrate(&conn).unwrap();
    memory::migrate(&conn).unwrap();
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, memory::SCHEMA_VERSION);
}

#[test]
fn sessions_episodes_and_checkpoints_roundtrip() {
    let conn = memory::open_memory_in_memory().unwrap();

    assert!(memory::open_session_id(&conn).unwrap().is_none());
    let session_id = memory::start_session(&conn).unwrap();
    assert_eq!(memory::open_session_id(&conn).unwrap(), Some(session_id.clone()));

    let first = memory::append_episode(&conn, &session_id, "turn_input", &json!({"text": "hello"}), None).unwrap();
    let second = memory::append_episode(&conn, &session_id, "turn_output", &json!({"text": "hi there"}), Some("greeting")).unwrap();
    assert_eq!(second, first + 1);

    memory::save_checkpoint(&conn, &session_id, first, &json!([{"role": "user", "content": "hello"}])).unwrap();
    let checkpoint = memory::load_checkpoint(&conn, &session_id).unwrap().unwrap();
    assert_eq!(checkpoint.last_episode_id, first);
    assert_eq!(checkpoint.context[0]["content"], "hello");

    let replayed = memory::episodes_after(&conn, &session_id, first).unwrap();
    assert_eq!(replayed.len(), 1);
    assert_eq!(replayed[0].kind, "turn_output");
    assert_eq!(replayed[0].situation_class.as_deref(), Some("greeting"));

    memory::end_session(&conn, &session_id).unwrap();
    assert!(memory::open_session_id(&conn).unwrap().is_none());
}

#[test]
fn telemetry_records_append() {
    let conn = memory::open_memory_in_memory().unwrap();
    memory::record_telemetry(&conn, "temp_c", 61.5, Some("idle")).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM self_telemetry", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}
