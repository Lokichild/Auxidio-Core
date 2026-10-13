use harness_core::ChatMessage;

#[derive(Debug)]
pub enum ModelError {
    Unreachable(String),
    BadResponse(String),
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelError::Unreachable(msg) => write!(f, "model unreachable: {}", msg),
            ModelError::BadResponse(msg) => write!(f, "bad model response: {}", msg),
        }
    }
}

pub trait ModelClient {
    fn chat(&self, system: &str, history: &[ChatMessage]) -> Result<String, ModelError>;
}

pub struct OllamaClient {
    pub url: String,
    pub model: String,
    pub timeout: std::time::Duration,
}

impl OllamaClient {
    pub fn new(url: &str, model: &str, timeout_secs: u64) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            timeout: std::time::Duration::from_secs(timeout_secs),
        }
    }
}

impl ModelClient for OllamaClient {
    fn chat(&self, system: &str, history: &[ChatMessage]) -> Result<String, ModelError> {
        let mut messages = vec![ChatMessage::system(system)];
        messages.extend(history.iter().cloned());
        let body = serde_json::json!({
            "model": self.model,
            "stream": false,
            "messages": messages,
        });
        let agent = ureq::AgentBuilder::new().timeout(self.timeout).build();
        let response = agent
            .post(&format!("{}/api/chat", self.url))
            .send_string(&body.to_string())
            .map_err(|err| ModelError::Unreachable(err.to_string()))?;
        let payload: serde_json::Value = response
            .into_json()
            .map_err(|err| ModelError::BadResponse(err.to_string()))?;
        payload["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| ModelError::BadResponse("missing message.content".to_string()))
    }
}

pub struct ScriptedClient {
    responses: std::cell::RefCell<Vec<String>>,
    pub calls: std::cell::RefCell<Vec<Vec<ChatMessage>>>,
}

impl ScriptedClient {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses: std::cell::RefCell::new(responses),
            calls: std::cell::RefCell::new(vec![]),
        }
    }
}

impl ModelClient for ScriptedClient {
    fn chat(&self, system: &str, history: &[ChatMessage]) -> Result<String, ModelError> {
        let mut full = vec![ChatMessage::system(system)];
        full.extend(history.iter().cloned());
        self.calls.borrow_mut().push(full);
        let mut queue = self.responses.borrow_mut();
        if queue.is_empty() {
            Ok("(scripted client exhausted)".to_string())
        } else {
            Ok(queue.remove(0))
        }
    }
}
