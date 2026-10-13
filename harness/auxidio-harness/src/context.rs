use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use harness_core::ChatMessage;

use crate::model::ModelClient;

pub const SUMMARY_SYSTEM: &str = "Summarize the conversation excerpt in two or three sentences. Preserve decisions made, commitments, names, and numbers exactly. Do not add interpretation.";

pub struct ContextManager {
    budget_messages: usize,
    keep_recent_messages: usize,
    pending: Mutex<Option<Vec<ChatMessage>>>,
    running: AtomicBool,
}

impl ContextManager {
    pub fn new(budget_messages: usize, keep_recent_messages: usize) -> Arc<Self> {
        Arc::new(Self {
            budget_messages,
            keep_recent_messages,
            pending: Mutex::new(None),
            running: AtomicBool::new(false),
        })
    }

    // Background compaction: works on a snapshot, never the live context. The
    // compacted copy is swapped in by the session at the next turn boundary —
    // the safe point — so an in-flight response is never touched.
    pub fn maybe_compact(
        self: &Arc<Self>,
        context: Vec<ChatMessage>,
        summarizer: Arc<dyn ModelClient + Send + Sync>,
    ) {
        if context.len() <= self.budget_messages {
            return;
        }
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }
        let manager = self.clone();
        let keep = self.keep_recent_messages;
        std::thread::Builder::new()
            .name("auxidio-ctx-gc".to_string())
            .spawn(move || {
                let compacted = compact(&context, summarizer.as_ref(), keep);
                *manager.pending.lock().expect("context pending lock") = Some(compacted);
                manager.running.store(false, Ordering::SeqCst);
            })
            .expect("spawn context collector");
    }

    pub fn take_compacted(&self) -> Option<Vec<ChatMessage>> {
        self.pending.lock().expect("context pending lock").take()
    }
}

pub fn compact(
    context: &[ChatMessage],
    summarizer: &dyn ModelClient,
    keep_recent: usize,
) -> Vec<ChatMessage> {
    if context.len() <= keep_recent {
        return context.to_vec();
    }
    let split = context.len() - keep_recent;
    let (old, recent) = context.split_at(split);
    let transcript: String = old
        .iter()
        .map(|message| format!("{}: {}", message.role, message.content))
        .collect::<Vec<_>>()
        .join("\n");
    let summary = summarizer
        .chat(SUMMARY_SYSTEM, &[ChatMessage::user(transcript)])
        .unwrap_or_else(|_| extractive_summary(old));
    let mut out = vec![ChatMessage::system(format!("[memory summary] {}", summary))];
    out.extend(recent.iter().cloned());
    out
}

fn extractive_summary(old: &[ChatMessage]) -> String {
    old.iter()
        .map(|message| first_sentence(&message.content))
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn first_sentence(text: &str) -> String {
    text.split(|c| c == '.' || c == '\n')
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}
