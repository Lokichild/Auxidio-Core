use std::sync::Arc;

use auxidio_harness::context::{compact, ContextManager};
use auxidio_harness::model::{ModelError, ScriptedClient};
use harness_core::ChatMessage;

fn conversation(n: usize) -> Vec<ChatMessage> {
    (0..n)
        .flat_map(|i| {
            vec![
                ChatMessage::user(format!("question {}. What should I do about the keys?", i)),
                ChatMessage::assistant(format!("answer {}. Remind yourself at the door.", i)),
            ]
        })
        .collect()
}

#[test]
fn compact_summarizes_old_and_keeps_recent() {
    let context = conversation(6);
    let summarizer = ScriptedClient::new(vec!["User repeatedly misplaced keys; agreed on a door reminder.".to_string()]);
    let compacted = compact(&context, &summarizer, 4);

    assert_eq!(compacted.len(), 5);
    assert_eq!(compacted[0].role, "system");
    assert!(compacted[0].content.contains("[memory summary]"));
    assert!(compacted[0].content.contains("door reminder"));
    assert_eq!(compacted[1..], context[8..]);
}

#[test]
fn compact_falls_back_to_extractive_when_summarizer_is_down() {
    struct Down;
    impl auxidio_harness::model::ModelClient for Down {
        fn chat(&self, _system: &str, _history: &[ChatMessage]) -> Result<String, ModelError> {
            Err(ModelError::Unreachable("no summarizer".into()))
        }
    }

    let context = conversation(3);
    let compacted = compact(&context, &Down, 2);
    assert_eq!(compacted.len(), 3);
    assert!(compacted[0].content.contains("[memory summary]"));
    assert!(compacted[0].content.contains("question 0"));
}

#[test]
fn manager_compacts_in_background_and_swaps_at_take() {
    let manager = ContextManager::new(4, 2);
    let summarizer: Arc<dyn auxidio_harness::model::ModelClient + Send + Sync> =
        Arc::new(ScriptedClient::new(vec!["keys discussion summarized.".to_string()]));

    // Under budget: nothing happens.
    manager.maybe_compact(conversation(1), summarizer.clone());
    assert!(manager.take_compacted().is_none());

    // Over budget: background compaction produces a swap-ready copy.
    manager.maybe_compact(conversation(6), summarizer.clone());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let compacted = loop {
        if let Some(compacted) = manager.take_compacted() {
            break compacted;
        }
        assert!(std::time::Instant::now() < deadline, "compaction never finished");
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    assert_eq!(compacted.len(), 3);
    assert!(compacted[0].content.contains("keys discussion summarized"));
    assert!(manager.take_compacted().is_none());
}
