use harness_core::{ChatMessage, VerifyResult, VerifyStatus};

use crate::model::ModelClient;
use crate::verifier::VerifierClient;

pub struct ExtractedBlock {
    pub claim: String,
    pub script: String,
    pub range: std::ops::Range<usize>,
}

pub fn extract_verify_block(text: &str) -> Option<ExtractedBlock> {
    const FENCE: &str = "```verify";
    let start = text.find(FENCE)?;
    let inner_start = start + FENCE.len();
    let rest = &text[inner_start..];
    let close_rel = rest.find("```")?;
    let inner = &rest[..close_rel];
    let value: serde_json::Value = serde_json::from_str(inner.trim()).ok()?;
    let claim = value.get("claim")?.as_str()?.to_string();
    let script = value.get("script")?.as_str()?.to_string();
    let end = inner_start + close_rel + 3;
    Some(ExtractedBlock { claim, script, range: start..end })
}

pub fn strip_verify_block(text: &str) -> String {
    match extract_verify_block(text) {
        Some(block) => {
            let mut cleaned = String::new();
            cleaned.push_str(&text[..block.range.start]);
            cleaned.push_str(&text[block.range.end..]);
            cleaned.trim().to_string()
        }
        None => text.to_string(),
    }
}

pub fn format_verification(result: &VerifyResult) -> String {
    match result.status {
        VerifyStatus::Verified => format!(
            "Verification: the executed script returned {}. The arithmetic checks out; use it.",
            render_value(result)
        ),
        VerifyStatus::Mismatch => format!(
            "Verification: the executed script returned {}, which contradicts the claimed value. Correct the calculation and restate the answer.",
            render_value(result)
        ),
        VerifyStatus::Timeout => format!(
            "Verification failed: the script exceeded its time limit. Treat the calculation as unverified. Script output: {}",
            result.stdout
        ),
        VerifyStatus::SyntaxError => format!(
            "Verification failed: the script had a syntax error. Treat the calculation as unverified. Detail: {}",
            result.stdout
        ),
        VerifyStatus::RuntimeError => format!(
            "Verification failed: the script raised a runtime error. Treat the calculation as unverified. Detail: {}",
            result.stdout
        ),
        VerifyStatus::NonNumeric => format!(
            "Verification failed: the script did not print a single numeric result. Treat the calculation as unverified. Output: {}",
            result.stdout
        ),
    }
}

fn render_value(result: &VerifyResult) -> String {
    match result.value {
        Some(value) => auxidio_core::py_str(value),
        None => result.stdout.trim().to_string(),
    }
}

pub fn generate_with_verification(
    model: &dyn ModelClient,
    verifier: Option<&dyn VerifierClient>,
    system: &str,
    history: &[ChatMessage],
    max_rounds: u32,
) -> Result<(String, u32), crate::model::ModelError> {
    let mut history = history.to_vec();
    let mut rounds = 0u32;
    loop {
        let response = model.chat(system, &history)?;
        match extract_verify_block(&response) {
            Some(block) if verifier.is_some() && rounds < max_rounds => {
                let verifier = verifier.expect("checked above");
                let result = verifier.verify(&block.claim, &block.script);
                let note = match &result {
                    Ok(result) => format_verification(result),
                    Err(err) => format!(
                        "Verification unavailable ({}). Treat the calculation as unverified.",
                        err
                    ),
                };
                history.push(ChatMessage::assistant(response));
                history.push(ChatMessage::user(note));
                rounds += 1;
            }
            Some(_) => {
                let cleaned = strip_verify_block(&response);
                return Ok((
                    format!(
                        "{} (I could not verify this calculation, so treat the numbers as unconfirmed.)",
                        cleaned
                    ),
                    rounds,
                ));
            }
            None => return Ok((response, rounds)),
        }
    }
}
