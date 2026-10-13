use serde::{Deserialize, Serialize};

pub const MAX_VERIFY_ROUNDS: u32 = 3;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".to_string(), content: content.into() }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: "assistant".to_string(), content: content.into() }
    }
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".to_string(), content: content.into() }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(tag = "kind")]
pub enum SensorEvent {
    AudioChunk {
        sample_rate: u32,
        samples: usize,
    },
    UtteranceComplete {
        transcript: String,
    },
    Frame {
        width: u32,
        height: u32,
        trigger: FrameTrigger,
    },
    Text {
        content: String,
        source: TextSource,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum FrameTrigger {
    Ambient,
    SpeechOnset,
    Motion,
    ActiveSituationWindow,
    ExplicitRequest,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TextSource {
    Stdin,
    LocalSocket,
    Peripheral,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct VerifyBlock {
    pub claim: String,
    pub script: String,
}

impl VerifyBlock {
    pub fn parse(fenced: &str) -> Option<VerifyBlock> {
        let body = fenced.trim();
        if !body.starts_with("```verify") {
            return None;
        }
        let inner = body.strip_prefix("```verify")?;
        let inner = inner.strip_suffix("```").unwrap_or(inner);
        serde_json::from_str::<VerifyBlock>(inner.trim()).ok()
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct VerifyResult {
    pub status: VerifyStatus,
    pub stdout: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub duration_ms: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum VerifyStatus {
    Verified,
    Mismatch,
    Timeout,
    SyntaxError,
    RuntimeError,
    NonNumeric,
}
