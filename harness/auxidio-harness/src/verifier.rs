use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use harness_core::{VerifyResult, VerifyStatus};

#[derive(Debug)]
pub enum VerifierError {
    Spawn(String),
    Io(String),
    Protocol(String),
}

impl std::fmt::Display for VerifierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerifierError::Spawn(msg) => write!(f, "verifier spawn failed: {}", msg),
            VerifierError::Io(msg) => write!(f, "verifier io error: {}", msg),
            VerifierError::Protocol(msg) => write!(f, "verifier protocol error: {}", msg),
        }
    }
}

pub trait VerifierClient {
    fn verify(&self, claim: &str, script: &str) -> Result<VerifyResult, VerifierError>;
}

pub struct PythonVerifier {
    child: std::sync::Mutex<Child>,
    stdin: std::sync::Mutex<ChildStdin>,
    stdout: std::sync::Mutex<BufReader<ChildStdout>>,
    next_id: std::cell::Cell<u64>,
}

impl PythonVerifier {
    pub fn spawn(python_cmd: &str, service_path: &str) -> Result<Self, VerifierError> {
        let mut child = Command::new(python_cmd)
            .arg(service_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| VerifierError::Spawn(err.to_string()))?;
        let stdin = child.stdin.take().ok_or_else(|| {
            VerifierError::Spawn("verifier stdin missing".to_string())
        })?;
        let stdout = BufReader::new(child.stdout.take().ok_or_else(|| {
            VerifierError::Spawn("verifier stdout missing".to_string())
        })?);
        Ok(Self {
            child: std::sync::Mutex::new(child),
            stdin: std::sync::Mutex::new(stdin),
            stdout: std::sync::Mutex::new(stdout),
            next_id: std::cell::Cell::new(1),
        })
    }
}

impl VerifierClient for PythonVerifier {
    fn verify(&self, claim: &str, script: &str) -> Result<VerifyResult, VerifierError> {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "verify",
            "params": { "claim": claim, "script": script },
        });
        let mut line = request.to_string();
        line.push('\n');
        {
            let mut stdin = self
                .stdin
                .lock()
                .map_err(|_| VerifierError::Io("stdin lock poisoned".to_string()))?;
            stdin
                .write_all(line.as_bytes())
                .map_err(|err| VerifierError::Io(err.to_string()))?;
            stdin
                .flush()
                .map_err(|err| VerifierError::Io(err.to_string()))?;
        }

        let mut response_line = String::new();
        let read = {
            let mut stdout = self
                .stdout
                .lock()
                .map_err(|_| VerifierError::Io("stdout lock poisoned".to_string()))?;
            stdout
                .read_line(&mut response_line)
                .map_err(|err| VerifierError::Io(err.to_string()))?
        };
        if read == 0 {
            return Err(VerifierError::Protocol("verifier closed".to_string()));
        }
        let payload: serde_json::Value = serde_json::from_str(&response_line)
            .map_err(|err| VerifierError::Protocol(err.to_string()))?;
        let result = payload.get("result").ok_or_else(|| {
            VerifierError::Protocol("missing result".to_string())
        })?;
        serde_json::from_value(result.clone())
            .map_err(|err| VerifierError::Protocol(err.to_string()))
    }
}

impl Drop for PythonVerifier {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub struct ScriptedVerifier {
    pub results: std::cell::RefCell<Vec<VerifyResult>>,
    pub calls: std::cell::RefCell<Vec<(String, String)>>,
}

impl ScriptedVerifier {
    pub fn new(results: Vec<VerifyResult>) -> Self {
        Self {
            results: std::cell::RefCell::new(results),
            calls: std::cell::RefCell::new(vec![]),
        }
    }
    pub fn result(status: VerifyStatus, value: Option<f64>) -> VerifyResult {
        VerifyResult {
            status,
            stdout: String::new(),
            value,
            note: None,
            duration_ms: 0,
        }
    }
}

impl VerifierClient for ScriptedVerifier {
    fn verify(&self, claim: &str, script: &str) -> Result<VerifyResult, VerifierError> {
        self.calls
            .borrow_mut()
            .push((claim.to_string(), script.to_string()));
        let mut queue = self.results.borrow_mut();
        if queue.is_empty() {
            Ok(ScriptedVerifier::result(VerifyStatus::RuntimeError, None))
        } else {
            Ok(queue.remove(0))
        }
    }
}
