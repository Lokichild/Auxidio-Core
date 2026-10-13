use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HarnessConfig {
    #[serde(default = "default_db_path")]
    pub db_path: String,
    #[serde(default = "default_user_name")]
    pub user_name: String,
    #[serde(default)]
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub session: SessionConfig,
    #[serde(default)]
    pub verify: VerifyConfig,
    #[serde(default)]
    pub selfmon: SelfmonConfig,
    #[serde(default)]
    pub queue: QueueConfig,
    #[serde(default)]
    pub context: ContextConfig,
    #[serde(default)]
    pub audio: AudioSettings,
    #[serde(default)]
    pub tts: TtsConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TtsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "crate::output::default_piper_cmd")]
    pub piper_cmd: Vec<String>,
    #[serde(default = "crate::output::default_play_cmd")]
    pub play_cmd: Vec<String>,
    #[serde(default = "default_voice")]
    pub voice: String,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            piper_cmd: crate::output::default_piper_cmd(),
            play_cmd: crate::output::default_play_cmd(),
            voice: default_voice(),
        }
    }
}

fn default_voice() -> String {
    "~/piper_voices/en_US-lessac-medium.onnx".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SelfmonConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_selfmon_interval")]
    pub interval_secs: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QueueConfig {
    #[serde(default = "default_target_stress_sum")]
    pub target_stress_sum: f64,
    #[serde(default = "default_local_mvt")]
    pub local_mvt: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContextConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_budget_messages")]
    pub budget_messages: usize,
    #[serde(default = "default_keep_recent")]
    pub keep_recent_messages: usize,
    #[serde(default = "default_summarizer_model")]
    pub summarizer_model: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "crate::sensors::audio::default_record_cmd")]
    pub record_cmd: Vec<String>,
    #[serde(default = "crate::sensors::audio::default_whisper_cmd")]
    pub whisper_cmd: Vec<String>,
    #[serde(default = "default_model")]
    pub whisper_model: String,
}

impl Default for SelfmonConfig {
    fn default() -> Self {
        Self { enabled: default_true(), interval_secs: default_selfmon_interval() }
    }
}
impl Default for QueueConfig {
    fn default() -> Self {
        Self { target_stress_sum: default_target_stress_sum(), local_mvt: default_local_mvt() }
    }
}
impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            budget_messages: default_budget_messages(),
            keep_recent_messages: default_keep_recent(),
            summarizer_model: default_summarizer_model(),
        }
    }
}
impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            record_cmd: crate::sensors::audio::default_record_cmd(),
            whisper_cmd: crate::sensors::audio::default_whisper_cmd(),
            whisper_model: default_model(),
        }
    }
}

fn default_selfmon_interval() -> u64 {
    30
}
fn default_target_stress_sum() -> f64 {
    1.0
}
fn default_local_mvt() -> f64 {
    10.0
}
fn default_budget_messages() -> usize {
    12
}
fn default_keep_recent() -> usize {
    4
}
fn default_summarizer_model() -> String {
    "gemma3:1b".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OllamaConfig {
    #[serde(default = "default_ollama_url")]
    pub url: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionConfig {
    #[serde(default = "default_context_turns")]
    pub context_turns: usize,
    #[serde(default = "default_checkpoint_interval")]
    pub checkpoint_interval: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VerifyConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_rounds")]
    pub max_rounds: u32,
    #[serde(default = "default_python_cmd")]
    pub python_cmd: String,
    #[serde(default = "default_service_path")]
    pub service_path: String,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
            user_name: default_user_name(),
            ollama: OllamaConfig::default(),
            session: SessionConfig::default(),
            verify: VerifyConfig::default(),
            selfmon: SelfmonConfig::default(),
            queue: QueueConfig::default(),
            context: ContextConfig::default(),
            audio: AudioSettings::default(),
            tts: TtsConfig::default(),
        }
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            url: default_ollama_url(),
            model: default_model(),
            timeout_secs: default_timeout_secs(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            context_turns: default_context_turns(),
            checkpoint_interval: default_checkpoint_interval(),
        }
    }
}

impl Default for VerifyConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            max_rounds: default_max_rounds(),
            python_cmd: default_python_cmd(),
            service_path: default_service_path(),
        }
    }
}

fn default_db_path() -> String {
    "auxidio_memory.db".to_string()
}
fn default_user_name() -> String {
    "Auxidio".to_string()
}
fn default_ollama_url() -> String {
    "http://127.0.0.1:11434".to_string()
}
fn default_model() -> String {
    "gemma3:1b".to_string()
}
fn default_timeout_secs() -> u64 {
    120
}
fn default_context_turns() -> usize {
    8
}
fn default_checkpoint_interval() -> u32 {
    4
}
fn default_true() -> bool {
    true
}
fn default_max_rounds() -> u32 {
    harness_core::MAX_VERIFY_ROUNDS
}
fn default_python_cmd() -> String {
    "python3".to_string()
}
fn default_service_path() -> String {
    "python/math_verifier/math_verifier.py".to_string()
}

pub fn load(path: &str) -> HarnessConfig {
    match std::fs::read_to_string(path) {
        Ok(content) => match toml::from_str::<HarnessConfig>(&content) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("config parse error in {}: {} — using defaults", path, err);
                HarnessConfig::default()
            }
        },
        Err(_) => HarnessConfig::default(),
    }
}
