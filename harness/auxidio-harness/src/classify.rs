use rusqlite::{params, Connection};
use serde_json::json;

use crate::model::ModelClient;

#[derive(Clone, PartialEq, Debug)]
pub struct ClassResult {
    pub class_id: String,
    pub confidence: f64,
    pub via_fast_path: bool,
}

pub fn seed_default_classes(conn: &Connection) -> rusqlite::Result<()> {
    let defaults = [
        (
            "crisis",
            "Immediate danger or emergency involving the user or people present",
            &["right now", "emergency", "danger", "hurt", "bleeding", "fire", "attack", "breaking in"][..],
            0.6,
        ),
        (
            "distress",
            "User is worried, stressed, or struggling but not in immediate danger",
            &["worried", "anxious", "stressed", "struggling", "upset", "frustrated", "stuck"][..],
            0.6,
        ),
        (
            "planning",
            "User is weighing options or organizing a future action",
            &["plan", "schedule", "organize", "decide", "choose", "option"][..],
            0.5,
        ),
        (
            "routine",
            "Everyday reminder or habitual task",
            &["reminder", "daily", "usual", "routine", "every day"][..],
            0.5,
        ),
    ];
    for (class_id, description, keywords, base_confidence) in defaults {
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM situation_classes WHERE class_id = ?",
            params![class_id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            conn.execute(
                "INSERT INTO situation_classes (class_id, name, description, fast_keywords_json, base_confidence) VALUES (?, ?, ?, ?, ?)",
                params![
                    class_id,
                    class_id,
                    description,
                    auxidio_core::py_dumps(&json!(keywords)),
                    base_confidence
                ],
            )?;
        }
    }
    Ok(())
}

pub fn known_class_ids(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT class_id FROM situation_classes ORDER BY class_id ASC")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    rows.collect()
}

pub fn classify(conn: &Connection, model: Option<&dyn ModelClient>, text: &str) -> ClassResult {
    let lowered = text.to_lowercase();
    if let Ok(classes) = load_fast_paths(conn) {
        for (class_id, keywords, base_confidence) in &classes {
            if keywords.iter().any(|keyword| lowered.contains(keyword.as_str())) {
                return ClassResult {
                    class_id: class_id.clone(),
                    confidence: *base_confidence,
                    via_fast_path: true,
                };
            }
        }
    }

    match model {
        Some(model) => model_fallback(conn, model, text),
        None => ClassResult { class_id: "unknown".to_string(), confidence: 0.0, via_fast_path: false },
    }
}

fn load_fast_paths(conn: &Connection) -> rusqlite::Result<Vec<(String, Vec<String>, f64)>> {
    let mut stmt = conn.prepare(
        "SELECT class_id, fast_keywords_json, base_confidence FROM situation_classes",
    )?;
    let rows = stmt.query_map([], |row| {
        let class_id: String = row.get(0)?;
        let keywords_json: String = row.get(1)?;
        let base_confidence: f64 = row.get(2)?;
        Ok((class_id, keywords_json, base_confidence))
    })?;
    let mut out = vec![];
    for row in rows {
        let (class_id, keywords_json, base_confidence) = row?;
        let keywords: Vec<String> =
            serde_json::from_str(&keywords_json).unwrap_or_default();
        if !keywords.is_empty() {
            out.push((class_id, keywords, base_confidence));
        }
    }
    Ok(out)
}

fn model_fallback(conn: &Connection, model: &dyn ModelClient, text: &str) -> ClassResult {
    let ids = known_class_ids(conn).unwrap_or_default();
    let system = format!(
        "You are a situation classifier. Reply with ONLY JSON: {{\"class_id\": string|null, \"label\": string|null, \"confidence\": number}}. class_id must be one of [{}] or null. If none fits, set class_id null and provide a short lowercase label. confidence is 0..1.",
        ids.join(", ")
    );
    let response = match model.chat(&system, &[harness_core::ChatMessage::user(text)]) {
        Ok(response) => response,
        Err(_) => return ClassResult { class_id: "unknown".to_string(), confidence: 0.0, via_fast_path: false },
    };
    let parsed: serde_json::Value = match serde_json::from_str(response.trim()) {
        Ok(value) => value,
        Err(_) => return ClassResult { class_id: "unknown".to_string(), confidence: 0.0, via_fast_path: false },
    };
    let confidence = parsed["confidence"].as_f64().unwrap_or(0.0).clamp(0.0, 1.0);

    if let Some(class_id) = parsed["class_id"].as_str() {
        if ids.iter().any(|known| known == class_id) {
            return ClassResult { class_id: class_id.to_string(), confidence, via_fast_path: false };
        }
    }
    if let Some(label) = parsed["label"].as_str() {
        let slug: String = label
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("_");
        if !slug.is_empty() {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO situation_classes (class_id, name, description, fast_keywords_json, base_confidence) VALUES (?, ?, ?, '[]', 0.5)",
                params![slug, slug, label],
            );
            return ClassResult { class_id: slug, confidence, via_fast_path: false };
        }
    }
    ClassResult { class_id: "unknown".to_string(), confidence: 0.0, via_fast_path: false }
}
