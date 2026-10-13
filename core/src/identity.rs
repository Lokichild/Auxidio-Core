use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Identity {
    pub user_name: String,
    pub device_id: String,
    pub created_at: String,
}

pub fn generate_identity(user_name: &str) -> Identity {
    Identity {
        user_name: user_name.to_string(),
        device_id: uuid::Uuid::new_v4().to_string(),
        created_at: now_str(),
    }
}

pub fn save_identity(identity: &Identity, path: &str) -> bool {
    match serde_json::to_string_pretty(identity) {
        Ok(content) => std::fs::write(path, content).is_ok(),
        Err(_) => false,
    }
}

pub fn load_identity(path: &str) -> Option<Identity> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn identity_value(identity: &Identity) -> serde_json::Value {
    json!({
        "user_name": identity.user_name,
        "device_id": identity.device_id,
        "created_at": identity.created_at
    })
}

fn now_str() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs() as i64;
    let (y, mo, d, h, mi, s) = crate::cases::civil_from_secs(secs);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
        y,
        mo,
        d,
        h,
        mi,
        s,
        dur.subsec_micros()
    )
}
