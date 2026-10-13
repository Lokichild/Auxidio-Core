use auxidio_harness::memory;
use auxidio_harness::queue;

#[test]
fn life_safety_outranks_growth() {
    let conn = memory::open_memory_in_memory().unwrap();
    memory::record_telemetry(&conn, "temp_c", 95.0, Some("test")).unwrap();

    let urgent = memory::add_problem_record(&conn, "user is injured", 8.0, 1, 1).unwrap();
    let trivial = memory::add_problem_record(&conn, "choose a hobby", 2.0, 5, 1).unwrap();

    let ranked = queue::ranked_open_problems(&conn, 1.0, 10.0);
    assert_eq!(ranked.len(), 2);
    assert_eq!(ranked[0].0.problem_id, urgent);
    assert_eq!(ranked[1].0.problem_id, trivial);
    assert!(ranked[0].1 > ranked[1].1);
}

#[test]
fn impact_scales_within_level() {
    let conn = memory::open_memory_in_memory().unwrap();
    let solo = memory::add_problem_record(&conn, "one person cold", 5.0, 2, 1).unwrap();
    let many = memory::add_problem_record(&conn, "household cold", 5.0, 2, 4).unwrap();

    let ranked = queue::ranked_open_problems(&conn, 1.0, 10.0);
    assert_eq!(ranked[0].0.problem_id, many);
    assert_eq!(ranked[1].0.problem_id, solo);
}

#[test]
fn c_self_falls_as_heat_rises() {
    let conn = memory::open_memory_in_memory().unwrap();
    let cool = queue::c_self(&conn, &queue::DEFAULT_BANDS);
    assert_eq!(cool, 100.0);

    memory::record_telemetry(&conn, "temp_c", 60.0, Some("test")).unwrap();
    let at_optimal = queue::c_self(&conn, &queue::DEFAULT_BANDS);
    assert_eq!(at_optimal, 100.0);

    memory::record_telemetry(&conn, "temp_c", 95.0, Some("test")).unwrap();
    let failing = queue::c_self(&conn, &queue::DEFAULT_BANDS);
    assert_eq!(failing, 0.0);
}

#[test]
fn c_others_measures_deviation_from_target_stress() {
    let conn = memory::open_memory_in_memory().unwrap();
    // No open problems: observed 0, target 1, span 10 -> |0-1|/10*100 = 10
    assert_eq!(queue::c_others(&conn, 1.0, 10.0), 10.0);

    memory::add_problem_record(&conn, "level 1 solo", 5.0, 1, 1).unwrap();
    // observed 1.0 == target 1.0 -> 0
    assert_eq!(queue::c_others(&conn, 1.0, 10.0), 0.0);
}
