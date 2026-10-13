use auxidio_harness::config::HarnessConfig;
use auxidio_harness::memory;
use auxidio_harness::model::ScriptedClient;
use auxidio_harness::session::SessionManager;
use auxidio_harness::verifier::ScriptedVerifier;
use harness_core::{VerifyResult, VerifyStatus};
use serde_json::json;

fn test_config() -> HarnessConfig {
    let mut config = HarnessConfig::default();
    config.session.context_turns = 2;
    config.session.checkpoint_interval = 2;
    config
}

#[test]
fn turn_stores_episodes_and_context() {
    let conn = memory::open_memory_in_memory().unwrap();
    let config = test_config();
    let mut manager = SessionManager::start_or_resume(conn, &config).unwrap();

    let model = ScriptedClient::new(vec!["You could remind yourself at the door.".to_string()]);
    let response = manager.process_turn("I keep forgetting my keys", &model, None).unwrap();
    assert_eq!(response, "You could remind yourself at the door.");
    assert_eq!(manager.context_len(), 2);

    let conn = manager.finish_into_conn().unwrap();
    let session_id = memory::open_session_id(&conn).unwrap();
    assert!(session_id.is_none());
}

#[test]
fn clarification_short_circuits_the_model() {
    let conn = memory::open_memory_in_memory().unwrap();
    let config = test_config();
    let mut manager = SessionManager::start_or_resume(conn, &config).unwrap();

    let long_previous = "I was thinking about setting up some security cameras around my home and wanted to discuss the best placement options for covering all entry points.";
    manager.process_turn(long_previous, &ScriptedClient::new(vec!["ok".into()]), None).unwrap();

    let model = ScriptedClient::new(vec!["should never be used".to_string()]);
    let response = manager.process_turn("wait", &model, None).unwrap();
    assert_eq!(response, "What is going on? Are you okay?");
    assert!(model.calls.borrow().is_empty());
}

#[test]
fn verify_loop_feeds_results_back() {
    let conn = memory::open_memory_in_memory().unwrap();
    let config = test_config();
    let mut manager = SessionManager::start_or_resume(conn, &config).unwrap();

    let fenced = "Let me check.\n```verify\n{\"claim\": \"3 * 14.99 + 6.50 = 49.99\", \"script\": \"print(round(3 * 14.99 + 6.50, 2))\"}\n```\n";
    let model = ScriptedClient::new(vec![
        fenced.to_string(),
        "The total is 51.47, not 49.99.".to_string(),
    ]);
    let verifier = ScriptedVerifier::new(vec![VerifyResult {
        status: VerifyStatus::Mismatch,
        stdout: "51.47".to_string(),
        value: Some(51.47),
        note: Some("claim states 49.99, script computed 51.47".to_string()),
        duration_ms: 1,
    }]);

    let response = manager
        .process_turn("what does three times 14.99 plus 6.50 come to", &model, Some(&verifier))
        .unwrap();
    assert_eq!(response, "The total is 51.47, not 49.99.");
    assert_eq!(verifier.calls.borrow().len(), 1);
    assert_eq!(model.calls.borrow().len(), 2);
    let second_call = &model.calls.borrow()[1];
    let injected = second_call.last().unwrap().content.clone();
    assert!(injected.contains("51.47"), "injection was: {}", injected);
}

fn temp_db_path(tag: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "auxidio_test_{}_{}.db",
        tag,
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    path.to_string_lossy().to_string()
}

#[test]
fn context_trims_and_resume_replays() {
    let path = temp_db_path("resume");
    let conn = memory::open_memory(&path).unwrap();
    let config = test_config();
    let mut manager = SessionManager::start_or_resume(conn, &config).unwrap();
    let session_id = manager.session_id().to_string();

    let model = ScriptedClient::new(vec![
        "one".into(), "two".into(), "three".into(), "four".into(), "five".into(),
    ]);
    for i in 1..=5 {
        manager.process_turn(&format!("turn {}", i), &model, None).unwrap();
    }
    assert!(manager.context_len() <= 4);
    manager.checkpoint().unwrap();
    let conn = manager.finish_into_conn().unwrap();

    let episodes = memory::episodes_after(&conn, &session_id, 0).unwrap();
    assert_eq!(episodes.len(), 10);
    drop(conn);

    // Resume path: reopen with an open session.
    let conn = memory::open_memory(&path).unwrap();
    conn.execute(
        "UPDATE sessions SET ended_at = NULL WHERE session_id = ?1",
        rusqlite::params![session_id],
    )
    .unwrap();
    let resumed = SessionManager::start_or_resume(conn, &config).unwrap();
    assert!(resumed.resumed());
    assert!(resumed.context_len() > 0 && resumed.context_len() <= 4);
    let _ = resumed.finish();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn model_failure_degrades_honestly() {
    use auxidio_harness::model::{ModelError, ModelClient};
    use harness_core::ChatMessage;

    struct FailingClient;
    impl ModelClient for FailingClient {
        fn chat(&self, _system: &str, _history: &[ChatMessage]) -> Result<String, ModelError> {
            Err(ModelError::Unreachable("connection refused".into()))
        }
    }

    let conn = memory::open_memory_in_memory().unwrap();
    let config = test_config();
    let mut manager = SessionManager::start_or_resume(conn, &config).unwrap();
    let response = manager.process_turn("hello there", &FailingClient, None).unwrap();
    assert!(response.contains("could not reach my reasoning model"));
    assert!(response.contains("connection refused"));
    let _ = manager.finish();
}
