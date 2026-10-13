use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};

use crate::py_dumps;
use crate::py_round;

pub const MIN_OUTCOME_SCORE: f64 = 0.3;
pub const MIN_USE_COUNT_BEFORE_REMOVAL: i64 = 5;

pub const INJECTION_PATTERNS: [&str; 10] = [
    "--", ";", "DROP", "DELETE", "INSERT", "UPDATE", "SELECT", "/*", "*/", "xp_",
];

pub fn check_for_injection(user_input: &str) -> Vec<String> {
    let input_upper = user_input.to_uppercase();
    INJECTION_PATTERNS
        .iter()
        .filter(|pattern| input_upper.contains(&pattern.to_uppercase()))
        .map(|pattern| pattern.to_string())
        .collect()
}

pub fn log_injection_attempt(
    conn: &Connection,
    attempted_input: &str,
    patterns_found: &[String],
) -> rusqlite::Result<()> {
    let patterns_json = py_dumps(&Value::Array(
        patterns_found.iter().map(|p| json!(p)).collect(),
    ));
    conn.execute(
        "INSERT INTO security_log (attempted_input, patterns_found, timestamp) VALUES (?, ?, ?)",
        params![attempted_input, patterns_json, now_iso()],
    )?;
    Ok(())
}

pub fn check_injection_history(conn: &Connection, time_window_minutes: i64) -> rusqlite::Result<i64> {
    let modifier = format!("-{} minutes", time_window_minutes);
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM security_log WHERE timestamp > datetime('now', ?)",
        params![modifier],
        |row| row.get(0),
    )?;
    Ok(count)
}

pub fn handle_injection_attempt(
    conn: &Connection,
    attempted_input: &str,
    patterns_found: &[String],
) -> rusqlite::Result<Value> {
    log_injection_attempt(conn, attempted_input, patterns_found)?;
    let recent_count = check_injection_history(conn, 30)?;
    let patterns_value = Value::Array(patterns_found.iter().map(|p| json!(p)).collect());
    if recent_count >= 3 {
        Ok(json!({
            "success": false,
            "severity": "escalated",
            "reason": "Multiple unusual inputs detected. The registered user will be informed.",
            "patterns_found": patterns_value
        }))
    } else {
        Ok(json!({
            "success": false,
            "severity": "logged",
            "reason": "Input flagged for suspicious patterns. Attempt logged.",
            "patterns_found": patterns_value
        }))
    }
}

pub fn get_security_log(conn: &Connection, unreviewed_only: bool) -> rusqlite::Result<Vec<Value>> {
    let _ = conn.execute(
        "ALTER TABLE security_log ADD COLUMN reviewed INTEGER DEFAULT 0",
        [],
    );

    let sql = if unreviewed_only {
        "SELECT id, attempted_input, patterns_found, timestamp FROM security_log WHERE reviewed = 0 ORDER BY timestamp DESC"
    } else {
        "SELECT id, attempted_input, patterns_found, timestamp FROM security_log ORDER BY timestamp DESC"
    };

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let attempted_input: String = row.get(1)?;
        let patterns_found: String = row.get(2)?;
        let timestamp: String = row.get(3)?;
        Ok((id, attempted_input, patterns_found, timestamp))
    })?;

    let mut entries = vec![];
    for row in rows {
        let (id, attempted_input, patterns_found, timestamp) = row?;
        let patterns: Value = serde_json::from_str(&patterns_found).unwrap_or(Value::Null);
        let mut entry = Map::new();
        entry.insert("id".to_string(), json!(id));
        entry.insert("attempted_input".to_string(), json!(attempted_input));
        entry.insert("patterns_found".to_string(), patterns);
        entry.insert("timestamp".to_string(), json!(timestamp));
        entries.push(Value::Object(entry));
    }
    Ok(entries)
}

pub fn mark_security_log_reviewed(conn: &Connection, entry_ids: &[i64]) -> rusqlite::Result<Value> {
    for entry_id in entry_ids {
        conn.execute(
            "UPDATE security_log SET reviewed = 1 WHERE id = ?",
            params![entry_id],
        )?;
    }
    Ok(json!({ "success": true, "marked_reviewed": entry_ids.len() }))
}

pub fn display_security_report(conn: &Connection) -> rusqlite::Result<String> {
    let entries = get_security_log(conn, true)?;
    if entries.is_empty() {
        return Ok("No new security events to report.".to_string());
    }

    let mut report_lines = vec![format!(
        "Security Report: {} unreviewed event(s) detected.",
        entries.len()
    )];
    report_lines.push(String::new());

    let mut entry_ids = vec![];
    for entry in &entries {
        let timestamp = entry.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
        let attempted = entry.get("attempted_input").and_then(|v| v.as_str()).unwrap_or("");
        let patterns: Vec<String> = entry
            .get("patterns_found")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| p.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        report_lines.push(format!("Time: {}", timestamp));
        report_lines.push(format!("Suspicious input detected: {}", attempted));
        report_lines.push(format!("Patterns found: {}", patterns.join(", ")));
        report_lines.push(String::new());
        if let Some(id) = entry.get("id").and_then(|v| v.as_i64()) {
            entry_ids.push(id);
        }
    }

    mark_security_log_reviewed(conn, &entry_ids)?;
    Ok(report_lines.join("\n"))
}

pub fn initialize_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS cases (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            description TEXT NOT NULL,
            factors TEXT NOT NULL,
            solution TEXT NOT NULL,
            outcome_score REAL NOT NULL,
            use_count INTEGER DEFAULT 1,
            created_at TEXT NOT NULL,
            last_used TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS security_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            attempted_input TEXT NOT NULL,
            patterns_found TEXT NOT NULL,
            timestamp TEXT NOT NULL
        );",
    )
}

pub fn open(path: &str) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    initialize_tables(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    initialize_tables(&conn)?;
    Ok(conn)
}

pub fn store_case(
    conn: &Connection,
    description: &str,
    factors: &Value,
    solution: &str,
    outcome_score: f64,
) -> rusqlite::Result<Value> {
    let injection_check = check_for_injection(description);
    if !injection_check.is_empty() {
        return handle_injection_attempt(conn, description, &injection_check);
    }

    let now = now_iso();
    conn.execute(
        "INSERT INTO cases (description, factors, solution, outcome_score, use_count, created_at, last_used) VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            description,
            py_dumps(factors),
            solution,
            outcome_score,
            1,
            now,
            now
        ],
    )?;
    let case_id = conn.last_insert_rowid();
    Ok(json!({ "success": true, "case_id": case_id }))
}

pub fn calculate_similarity(stored_factors: &Map<String, Value>, query_factors: &Map<String, Value>) -> f64 {
    if stored_factors.is_empty() || query_factors.is_empty() {
        return 0.0;
    }

    let mut matching_factors = 0i64;
    let total_factors = query_factors.len() as f64;

    for (key, value) in query_factors.iter() {
        if let Some(stored) = stored_factors.get(key) {
            if stored == value {
                matching_factors += 1;
            }
        }
    }

    if total_factors == 0.0 {
        return 0.0;
    }

    matching_factors as f64 / total_factors
}

pub fn find_similar_cases(
    conn: &Connection,
    query_factors: &Map<String, Value>,
    min_similarity: f64,
    max_results: usize,
) -> rusqlite::Result<Vec<Value>> {
    let mut stmt = conn.prepare(
        "SELECT id, description, factors, solution, outcome_score, use_count FROM cases ORDER BY outcome_score DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        let case_id: i64 = row.get(0)?;
        let description: String = row.get(1)?;
        let factors_json: String = row.get(2)?;
        let solution: String = row.get(3)?;
        let outcome_score: f64 = row.get(4)?;
        let use_count: i64 = row.get(5)?;
        Ok((case_id, description, factors_json, solution, outcome_score, use_count))
    })?;

    let mut scored_cases = vec![];
    for row in rows {
        let (case_id, description, factors_json, solution, outcome_score, use_count) = row?;
        let stored_factors: Map<String, Value> = serde_json::from_str(&factors_json)
            .ok()
            .and_then(|v: Value| v.as_object().cloned())
            .unwrap_or_default();
        let similarity = calculate_similarity(&stored_factors, query_factors);

        if similarity >= min_similarity {
            let combined_score = (similarity * 0.6) + (outcome_score * 0.4);
            let mut entry = Map::new();
            entry.insert("case_id".to_string(), json!(case_id));
            entry.insert("description".to_string(), json!(description));
            entry.insert("solution".to_string(), json!(solution));
            entry.insert("outcome_score".to_string(), json!(outcome_score));
            entry.insert("similarity".to_string(), json!(py_round(similarity, 2)));
            entry.insert("combined_score".to_string(), json!(py_round(combined_score, 2)));
            entry.insert("use_count".to_string(), json!(use_count));
            scored_cases.push(Value::Object(entry));
        }
    }

    scored_cases.sort_by(|a, b| {
        let av = a.get("combined_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let bv = b.get("combined_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
    });
    scored_cases.truncate(max_results);
    Ok(scored_cases)
}

pub fn update_outcome(conn: &Connection, case_id: i64, new_result: f64) -> rusqlite::Result<Value> {
    let row: Option<(f64, i64)> = conn
        .query_row(
            "SELECT outcome_score, use_count FROM cases WHERE id = ?",
            params![case_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    let (current_score, use_count) = match row {
        Some(r) => r,
        None => return Ok(json!({ "success": false, "reason": "Case not found." })),
    };

    let new_score = (current_score * use_count as f64 + new_result) / (use_count as f64 + 1.0);
    let new_use_count = use_count + 1;

    if new_score < MIN_OUTCOME_SCORE && new_use_count >= MIN_USE_COUNT_BEFORE_REMOVAL {
        conn.execute("DELETE FROM cases WHERE id = ?", params![case_id])?;
        return Ok(json!({
            "success": true,
            "action": "removed",
            "reason": format!(
                "Outcome score {} fell below threshold after {} uses.",
                py_round(new_score, 2),
                new_use_count
            )
        }));
    }

    let now = now_iso();
    conn.execute(
        "UPDATE cases SET outcome_score = ?, use_count = ?, last_used = ? WHERE id = ?",
        params![py_round(new_score, 2), new_use_count, now, case_id],
    )?;
    Ok(json!({
        "success": true,
        "action": "updated",
        "new_score": py_round(new_score, 2),
        "use_count": new_use_count
    }))
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs() as i64;
    let (y, mo, d, h, mi, s) = civil_from_secs(secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}",
        y,
        mo,
        d,
        h,
        mi,
        s,
        dur.subsec_micros()
    )
}

pub fn civil_from_secs(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let h = (rem / 3600) as u32;
    let mi = ((rem % 3600) / 60) as u32;
    let s = (rem % 60) as u32;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d, h, mi, s)
}
