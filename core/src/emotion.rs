use serde::Serialize;
use serde_json::{json, Value};

use crate::py_round;
use crate::py_str;

pub const THRESHOLD_CPU_TEMP: f64 = 80.0;
pub const THRESHOLD_MEMORY: f64 = 85.0;
pub const THRESHOLD_STORAGE: f64 = 90.0;

pub const IMMEDIATE: &str = "immediate";
pub const CONVENIENT: &str = "convenient";

pub const STABLE: &str = "stable";
pub const STRESSED: &str = "stressed";
pub const CRISIS: &str = "crisis";

pub const STRESSED_INDICATORS: [&str; 15] = [
    "worried", "concerned", "anxious", "nervous", "scared", "problem", "trouble", "difficult",
    "struggling", "help", "upset", "frustrated", "confused", "lost", "stuck",
];

pub const CRISIS_INDICATORS: [&str; 17] = [
    "right now", "happening now", "currently", "emergency", "urgent", "immediately",
    "someone is", "they are", "breaking in", "attack", "danger", "hurt", "bleeding", "fire",
    "threatening", "following me", "inside my",
];

pub const TONE_SHIFT_INDICATORS: [&str; 10] = [
    "wait", "stop", "hold on", "never mind", "forget that", "actually", "quickly", "hurry",
    "fast", "now",
];

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct Warning {
    pub priority: String,
    pub system: String,
    pub current_value: f64,
    pub threshold: f64,
    pub warning: String,
    pub suggestion: String,
}

pub fn warning_value(w: &Warning) -> Value {
    json!({
        "priority": w.priority,
        "system": w.system,
        "current_value": w.current_value,
        "threshold": w.threshold,
        "warning": w.warning,
        "suggestion": w.suggestion
    })
}

pub fn check_cpu_temperature_from(readings: &[f64]) -> Option<Warning> {
    let mut cpu_temp: Option<f64> = None;
    for entry in readings {
        if cpu_temp.is_none() || *entry > cpu_temp.unwrap() {
            cpu_temp = Some(*entry);
        }
    }
    if let Some(t) = cpu_temp {
        if t != 0.0 && t >= THRESHOLD_CPU_TEMP {
            let rounded = py_round(t, 1);
            return Some(Warning {
                priority: IMMEDIATE.to_string(),
                system: "cpu_temperature".to_string(),
                current_value: rounded,
                threshold: THRESHOLD_CPU_TEMP,
                warning: format!(
                    "I am overheating. My current temperature is {}°C, which is above my safe operating limit of {}°C.",
                    py_str(rounded),
                    py_str(THRESHOLD_CPU_TEMP)
                ),
                suggestion: "Please move me to a cooler location, ensure my fan is unobstructed, and reduce the number of active tasks. I may need to rest for a few minutes.".to_string(),
            });
        }
    }
    None
}

pub fn check_memory_from(usage_percent: f64) -> Option<Warning> {
    if usage_percent >= THRESHOLD_MEMORY {
        let rounded = py_round(usage_percent, 1);
        return Some(Warning {
            priority: CONVENIENT.to_string(),
            system: "memory".to_string(),
            current_value: rounded,
            threshold: THRESHOLD_MEMORY,
            warning: format!(
                "My memory is running low. I am currently using {}% of available memory.",
                py_str(rounded)
            ),
            suggestion: "I would benefit from closing any unused applications or processes. If this continues, consider restarting me when convenient.".to_string(),
        });
    }
    None
}

pub fn check_storage_from(drives: &[(String, u64, u64)]) -> Option<Vec<Warning>> {
    let mut warnings = vec![];
    for (drive, used, total) in drives {
        if *total == 0 {
            continue;
        }
        let percent_used = (*used as f64 / *total as f64) * 100.0;
        if percent_used >= THRESHOLD_STORAGE {
            let rounded = py_round(percent_used, 1);
            warnings.push(Warning {
                priority: CONVENIENT.to_string(),
                system: format!("storage_{}", drive),
                current_value: rounded,
                threshold: THRESHOLD_STORAGE,
                warning: format!(
                    "My storage at {} is running low. I am currently using {}% of available space.",
                    drive,
                    py_str(rounded)
                ),
                suggestion: "Consider clearing older case library entries or expanding my storage capacity. I may have difficulty learning new solutions if this continues.".to_string(),
            });
        }
    }
    if warnings.is_empty() {
        None
    } else {
        Some(warnings)
    }
}

pub fn run_inward_channel_from(
    temp_readings: &[f64],
    memory_percent: f64,
    drives: &[(String, u64, u64)],
) -> (Vec<Warning>, Vec<Warning>) {
    let mut immediate_warnings = vec![];
    let mut convenient_warnings = vec![];

    if let Some(w) = check_cpu_temperature_from(temp_readings) {
        immediate_warnings.push(w);
    }
    if let Some(w) = check_memory_from(memory_percent) {
        convenient_warnings.push(w);
    }
    if let Some(ws) = check_storage_from(drives) {
        convenient_warnings.extend(ws);
    }

    (immediate_warnings, convenient_warnings)
}

pub fn read_cpu_temperatures() -> Vec<f64> {
    let mut temps = vec![];
    if let Ok(entries) = std::fs::read_dir("/sys/class/thermal") {
        for entry in entries.flatten() {
            let path = entry.path().join("temp");
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(millideg) = content.trim().parse::<f64>() {
                    temps.push(millideg / 1000.0);
                }
            }
        }
    }
    temps
}

pub fn read_memory_percent() -> f64 {
    let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total: Option<f64> = None;
    let mut available: Option<f64> = None;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total = rest.split_whitespace().next().and_then(|v| v.parse::<f64>().ok());
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            available = rest.split_whitespace().next().and_then(|v| v.parse::<f64>().ok());
        }
    }
    match (total, available) {
        (Some(t), Some(a)) if t > 0.0 => ((t - a) / t) * 100.0,
        _ => 0.0,
    }
}

pub fn read_storage(drives: &[&str]) -> Vec<(String, u64, u64)> {
    let mut out = vec![];
    for drive in drives {
        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            let c_path = std::ffi::CString::new(*drive).unwrap();
            if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
                let total = stat.f_blocks as u64 * stat.f_frsize as u64;
                let free = stat.f_bfree as u64 * stat.f_frsize as u64;
                out.push((drive.to_string(), total.saturating_sub(free), total));
            }
        }
    }
    out
}

pub fn run_inward_channel() -> (Vec<Warning>, Vec<Warning>) {
    run_inward_channel_from(
        &read_cpu_temperatures(),
        read_memory_percent(),
        &read_storage(&["/", "/mnt/ssd2"]),
    )
}

pub fn report_inward_status() -> Value {
    let (immediate, convenient) = run_inward_channel();
    let all_clear = immediate.is_empty() && convenient.is_empty();
    json!({
        "all_clear": all_clear,
        "immediate_warnings": immediate.iter().map(warning_value).collect::<Vec<_>>(),
        "convenient_warnings": convenient.iter().map(warning_value).collect::<Vec<_>>()
    })
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct StateAnalysis {
    pub state: String,
    pub reasoning: String,
    pub clarification_needed: bool,
    pub clarification_question: Option<String>,
}

pub fn analyze_user_state(current_input: &str, previous_input: Option<&str>) -> StateAnalysis {
    let current_lower = current_input.to_lowercase();

    if let Some(previous) = previous_input {
        if !previous.is_empty() {
            let tone_shift_detected = TONE_SHIFT_INDICATORS
                .iter()
                .any(|indicator| current_lower.contains(indicator));
            let length_shift = previous.split_whitespace().count() > 10
                && current_input.split_whitespace().count() < 5;

            if tone_shift_detected && length_shift {
                return StateAnalysis {
                    state: CRISIS.to_string(),
                    reasoning: "Multiple tone shift indicators detected simultaneously.".to_string(),
                    clarification_needed: true,
                    clarification_question: Some("What is going on? Are you okay?".to_string()),
                };
            } else if tone_shift_detected || length_shift {
                return StateAnalysis {
                    state: CRISIS.to_string(),
                    reasoning: "Sudden change in communication pattern detected.".to_string(),
                    clarification_needed: true,
                    clarification_question: Some("Are you dealing with something right now?".to_string()),
                };
            }
        }
    }

    let crisis_matches: Vec<&str> = CRISIS_INDICATORS
        .iter()
        .copied()
        .filter(|indicator| current_lower.contains(indicator))
        .collect();
    if !crisis_matches.is_empty() {
        return StateAnalysis {
            state: CRISIS.to_string(),
            reasoning: "Immediate situation language detected.".to_string(),
            clarification_needed: false,
            clarification_question: None,
        };
    }

    let stressed_matches: Vec<&str> = STRESSED_INDICATORS
        .iter()
        .copied()
        .filter(|indicator| current_lower.contains(indicator))
        .collect();
    if !stressed_matches.is_empty() {
        return StateAnalysis {
            state: STRESSED.to_string(),
            reasoning: "Elevated concern language detected.".to_string(),
            clarification_needed: false,
            clarification_question: None,
        };
    }

    StateAnalysis {
        state: STABLE.to_string(),
        reasoning: "No elevated state indicators detected.".to_string(),
        clarification_needed: false,
        clarification_question: None,
    }
}

pub fn get_communication_style(state: &str) -> Value {
    if state == CRISIS {
        json!({
            "state": CRISIS,
            "tone": "Direct and decisive.",
            "response_length": "Brief. Essential information only.",
            "question_style": "Single, plain, immediate. Example: 'Are you safe right now?'",
            "elaboration": "None unless explicitly requested.",
            "pacing": "Immediate response priority."
        })
    } else if state == STRESSED {
        json!({
            "state": STRESSED,
            "tone": "Calm and confident.",
            "response_length": "Concise. Clear and complete but not lengthy.",
            "question_style": "Direct and purposeful. Brief explanation of why if needed.",
            "elaboration": "Available if requested, not offered automatically.",
            "pacing": "Steady and unhurried."
        })
    } else {
        json!({
            "state": STABLE,
            "tone": "Conversational and thorough.",
            "response_length": "As needed. Full explanations offered naturally.",
            "question_style": "Open and exploratory.",
            "elaboration": "Offered freely.",
            "pacing": "Natural conversation flow."
        })
    }
}

pub fn run_outward_channel(current_input: &str, previous_input: Option<&str>) -> Value {
    let state_analysis = analyze_user_state(current_input, previous_input);
    let communication_style = get_communication_style(&state_analysis.state);
    json!({
        "user_state": state_analysis.state,
        "reasoning": state_analysis.reasoning,
        "clarification_needed": state_analysis.clarification_needed,
        "clarification_question": state_analysis.clarification_question,
        "communication_style": communication_style
    })
}

// Manual pins for the inward channel. The hardware readers cannot run inside the
// vector generator, so these expected strings are transcribed verbatim from the
// frozen Python reference (emotional_framework.py) and pinned here.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inward_temperature_warning_matches_reference_strings() {
        let warning = check_cpu_temperature_from(&[72.0, 85.26]).unwrap();
        assert_eq!(warning.priority, "immediate");
        assert_eq!(warning.system, "cpu_temperature");
        assert_eq!(warning.current_value, 85.3);
        assert_eq!(warning.threshold, 80.0);
        assert_eq!(
            warning.warning,
            "I am overheating. My current temperature is 85.3°C, which is above my safe operating limit of 80.0°C."
        );
        assert_eq!(
            warning.suggestion,
            "Please move me to a cooler location, ensure my fan is unobstructed, and reduce the number of active tasks. I may need to rest for a few minutes."
        );
    }

    #[test]
    fn inward_temperature_below_threshold_is_silent() {
        assert!(check_cpu_temperature_from(&[79.9]).is_none());
        assert!(check_cpu_temperature_from(&[]).is_none());
    }

    #[test]
    fn inward_memory_warning_matches_reference_strings() {
        let warning = check_memory_from(91.234).unwrap();
        assert_eq!(warning.priority, "convenient");
        assert_eq!(warning.system, "memory");
        assert_eq!(warning.current_value, 91.2);
        assert_eq!(
            warning.warning,
            "My memory is running low. I am currently using 91.2% of available memory."
        );
        assert_eq!(
            warning.suggestion,
            "I would benefit from closing any unused applications or processes. If this continues, consider restarting me when convenient."
        );
    }

    #[test]
    fn inward_storage_warning_matches_reference_strings() {
        let drives = vec![("/".to_string(), 950u64, 1000u64), ("/mnt/ssd2".to_string(), 100, 1000)];
        let warnings = check_storage_from(&drives).unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].system, "storage_/");
        assert_eq!(warnings[0].current_value, 95.0);
        assert_eq!(
            warnings[0].warning,
            "My storage at / is running low. I am currently using 95.0% of available space."
        );
        assert_eq!(
            warnings[0].suggestion,
            "Consider clearing older case library entries or expanding my storage capacity. I may have difficulty learning new solutions if this continues."
        );
    }
}
