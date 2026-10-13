use auxidio_harness::memory;
use auxidio_harness::selfmon;

#[test]
fn sample_writes_telemetry_without_panicking() {
    let conn = memory::open_memory_in_memory().unwrap();
    selfmon::sample_once(&conn, |_| {});

    let memory_sample = memory::latest_telemetry(&conn, "memory_pct").unwrap();
    assert!(memory_sample.is_some(), "memory telemetry should always be recorded");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM self_telemetry", [], |row| row.get(0))
        .unwrap();
    assert!(count >= 1);
}

#[test]
fn threshold_breach_raises_maintenance_request() {
    let conn = memory::open_memory_in_memory().unwrap();
    // Inject a hot reading through the same code path the sampler uses.
    let (immediate, _) = auxidio_core::emotion::run_inward_channel_from(&[91.0], 50.0, &[]);
    assert_eq!(immediate.len(), 1);
    for warning in &immediate {
        memory::add_maintenance_request(
            &conn,
            &warning.system,
            "thermal",
            &warning.priority,
            &warning.warning,
            &serde_json::json!({ "current_value": warning.current_value }),
        )
        .unwrap();
    }
    let open = memory::open_maintenance_requests(&conn).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].2, "immediate");
    assert!(open[0].3.contains("overheating"));
}
