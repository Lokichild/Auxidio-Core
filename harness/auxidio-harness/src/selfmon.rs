use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use crate::memory;

pub struct SelfmonHandle {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl SelfmonHandle {
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

pub fn start(db_path: &str, interval: Duration, deliver: fn(&str)) -> Result<SelfmonHandle, rusqlite::Error> {
    let conn = memory::open_memory(db_path)?;
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();

    let handle = std::thread::Builder::new()
        .name("auxidio-selfmon".to_string())
        .spawn(move || {
            while !stop_flag.load(Ordering::SeqCst) {
                sample_once(&conn, deliver);
                std::thread::sleep(interval);
            }
        })
        .map_err(|err| rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
            Some(err.to_string()),
        ))?;

    Ok(SelfmonHandle { stop, handle: Some(handle) })
}

pub fn sample_once(conn: &rusqlite::Connection, deliver: fn(&str)) {
    let temps = auxidio_core::emotion::read_cpu_temperatures();
    let memory_percent = auxidio_core::emotion::read_memory_percent();
    let drives = auxidio_core::emotion::read_storage(&["/", "/mnt/ssd2"]);

    if let Some(hottest) = temps.iter().cloned().fold(None, |acc, t| match acc {
        Some(current) if current >= t => Some(current),
        _ => Some(t),
    }) {
        let _ = memory::record_telemetry(conn, "temp_c", hottest, Some("selfmon"));
    }
    let _ = memory::record_telemetry(conn, "memory_pct", memory_percent, Some("selfmon"));
    for (drive, used, total) in &drives {
        if *total > 0 {
            let percent = (*used as f64 / *total as f64) * 100.0;
            let _ = memory::record_telemetry(
                conn,
                &format!("storage_pct_{}", drive),
                percent,
                Some("selfmon"),
            );
        }
    }

    let (immediate, convenient) =
        auxidio_core::emotion::run_inward_channel_from(&temps, memory_percent, &drives);

    for warning in &immediate {
        let _ = memory::add_maintenance_request(
            conn,
            &warning.system,
            "thermal",
            &warning.priority,
            &warning.warning,
            &json!({
                "current_value": warning.current_value,
                "threshold": warning.threshold,
            }),
        );
        deliver(&warning.warning);
    }
    for warning in &convenient {
        let kind = if warning.system == "memory" { "resource" } else { "storage" };
        let _ = memory::add_maintenance_request(
            conn,
            &warning.system,
            kind,
            &warning.priority,
            &warning.warning,
            &json!({
                "current_value": warning.current_value,
                "threshold": warning.threshold,
            }),
        );
    }
}
