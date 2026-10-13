use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SCHEMA_VERSION: i64 = 1;

pub fn open_memory(path: &str) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    configure(&conn)?;
    auxidio_core::cases::initialize_tables(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn open_memory_in_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    configure(&conn)?;
    auxidio_core::cases::initialize_tables(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "FULL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_millis(2000))?;
    Ok(())
}

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version >= SCHEMA_VERSION {
        return Ok(());
    }
    conn.execute_batch(
        "BEGIN IMMEDIATE;
        CREATE TABLE IF NOT EXISTS sessions (
            session_id TEXT PRIMARY KEY,
            started_at TEXT NOT NULL,
            ended_at TEXT,
            resume_state TEXT
        );
        CREATE TABLE IF NOT EXISTS episodes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            situation_class TEXT,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS situation_classes (
            class_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            fast_keywords_json TEXT NOT NULL,
            base_confidence REAL NOT NULL DEFAULT 0.5
        );
        CREATE TABLE IF NOT EXISTS factor_weights (
            class_id TEXT NOT NULL,
            factor_key TEXT NOT NULL,
            weight REAL NOT NULL,
            observations INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (class_id, factor_key)
        );
        CREATE TABLE IF NOT EXISTS case_factors (
            case_id INTEGER NOT NULL,
            factor_key TEXT NOT NULL,
            factor_value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS outcomes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            case_id INTEGER NOT NULL,
            session_id TEXT,
            score REAL NOT NULL,
            source TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS problem_records (
            problem_id INTEGER PRIMARY KEY AUTOINCREMENT,
            description TEXT NOT NULL,
            class_id TEXT,
            original_value REAL NOT NULL,
            current_value REAL NOT NULL,
            maslow_level INTEGER NOT NULL,
            m_impact INTEGER NOT NULL DEFAULT 1,
            recurrence_count INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'open',
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS user_profile (
            user_id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL,
            notes TEXT
        );
        CREATE TABLE IF NOT EXISTS adaptations (
            user_id TEXT NOT NULL,
            param_key TEXT NOT NULL,
            value_json TEXT NOT NULL,
            source TEXT NOT NULL,
            pin TEXT,
            confidence REAL NOT NULL DEFAULT 0.5,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (user_id, param_key)
        );
        CREATE TABLE IF NOT EXISTS adaptation_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL,
            param_key TEXT NOT NULL,
            old_value TEXT,
            new_value TEXT NOT NULL,
            reason TEXT NOT NULL,
            outcome TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS session_state (
            session_id TEXT NOT NULL,
            checkpoint_at TEXT NOT NULL,
            last_episode_id INTEGER NOT NULL,
            context_json TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS self_telemetry (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            metric TEXT NOT NULL,
            value REAL NOT NULL,
            context TEXT,
            created_at TEXT NOT NULL
        );
        COMMIT;",
    )?;
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Episode {
    pub id: i64,
    pub session_id: String,
    pub kind: String,
    pub payload: Value,
    pub situation_class: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Checkpoint {
    pub session_id: String,
    pub checkpoint_at: String,
    pub last_episode_id: i64,
    pub context: Value,
}

pub fn start_session(conn: &Connection) -> rusqlite::Result<String> {
    let session_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO sessions (session_id, started_at) VALUES (?, ?)",
        params![session_id, now_iso()],
    )?;
    Ok(session_id)
}

pub fn open_session_id(conn: &Connection) -> rusqlite::Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT session_id FROM sessions WHERE ended_at IS NULL ORDER BY started_at DESC LIMIT 1",
    )?;
    let mut rows = stmt.query([])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn end_session(conn: &Connection, session_id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sessions SET ended_at = ? WHERE session_id = ?",
        params![now_iso(), session_id],
    )?;
    Ok(())
}

pub fn append_episode(
    conn: &Connection,
    session_id: &str,
    kind: &str,
    payload: &Value,
    situation_class: Option<&str>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO episodes (session_id, kind, payload_json, situation_class, created_at) VALUES (?, ?, ?, ?, ?)",
        params![
            session_id,
            kind,
            auxidio_core::py_dumps(payload),
            situation_class,
            now_iso()
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn episodes_after(conn: &Connection, session_id: &str, last_id: i64) -> rusqlite::Result<Vec<Episode>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, kind, payload_json, situation_class, created_at FROM episodes WHERE session_id = ? AND id > ? ORDER BY id ASC",
    )?;
    let rows = stmt.query_map(params![session_id, last_id], |row| {
        let payload_json: String = row.get(3)?;
        Ok(EpisodeRow {
            id: row.get(0)?,
            session_id: row.get(1)?,
            kind: row.get(2)?,
            payload_json,
            situation_class: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    let mut out = vec![];
    for row in rows {
        let row = row?;
        let payload: Value = serde_json::from_str(&row.payload_json).unwrap_or(Value::Null);
        out.push(Episode {
            id: row.id,
            session_id: row.session_id,
            kind: row.kind,
            payload,
            situation_class: row.situation_class,
            created_at: row.created_at,
        });
    }
    Ok(out)
}

struct EpisodeRow {
    id: i64,
    session_id: String,
    kind: String,
    payload_json: String,
    situation_class: Option<String>,
    created_at: String,
}

pub fn save_checkpoint(
    conn: &Connection,
    session_id: &str,
    last_episode_id: i64,
    context: &Value,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO session_state (session_id, checkpoint_at, last_episode_id, context_json) VALUES (?, ?, ?, ?)",
        params![session_id, now_iso(), last_episode_id, auxidio_core::py_dumps(context)],
    )?;
    Ok(())
}

pub fn load_checkpoint(conn: &Connection, session_id: &str) -> rusqlite::Result<Option<Checkpoint>> {
    let mut stmt = conn.prepare(
        "SELECT session_id, checkpoint_at, last_episode_id, context_json FROM session_state WHERE session_id = ? ORDER BY rowid DESC LIMIT 1",
    )?;
    let mut rows = stmt.query(params![session_id])?;
    match rows.next()? {
        Some(row) => {
            let context_json: String = row.get(3)?;
            Ok(Some(Checkpoint {
                session_id: row.get(0)?,
                checkpoint_at: row.get(1)?,
                last_episode_id: row.get(2)?,
                context: serde_json::from_str(&context_json).unwrap_or(Value::Null),
            }))
        }
        None => Ok(None),
    }
}

pub fn record_telemetry(
    conn: &Connection,
    metric: &str,
    value: f64,
    context: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO self_telemetry (metric, value, context, created_at) VALUES (?, ?, ?, ?)",
        params![metric, value, context, now_iso()],
    )?;
    Ok(())
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs() as i64;
    let (y, mo, d, h, mi, s) = auxidio_core::cases::civil_from_secs(secs);
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
