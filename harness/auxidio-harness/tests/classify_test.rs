use auxidio_harness::classify;
use auxidio_harness::memory;
use auxidio_harness::model::ScriptedClient;

#[test]
fn seeds_and_fast_path() {
    let conn = memory::open_memory_in_memory().unwrap();
    classify::seed_default_classes(&conn).unwrap();
    classify::seed_default_classes(&conn).unwrap(); // idempotent

    let ids = classify::known_class_ids(&conn).unwrap();
    assert!(ids.contains(&"crisis".to_string()));
    assert!(ids.contains(&"planning".to_string()));

    let crisis = classify::classify(&conn, None, "there is a fire in the kitchen");
    assert_eq!(crisis.class_id, "crisis");
    assert!(crisis.via_fast_path);

    let planning = classify::classify(&conn, None, "I want to plan the week ahead");
    assert_eq!(planning.class_id, "planning");
    assert!(planning.via_fast_path);

    let unknown = classify::classify(&conn, None, "the wallpaper is beige");
    assert_eq!(unknown.class_id, "unknown");
    assert_eq!(unknown.confidence, 0.0);
}

#[test]
fn model_fallback_picks_known_class() {
    let conn = memory::open_memory_in_memory().unwrap();
    classify::seed_default_classes(&conn).unwrap();
    let model = ScriptedClient::new(vec![r#"{"class_id": "routine", "label": null, "confidence": 0.7}"#.to_string()]);
    let result = classify::classify(&conn, Some(&model), "the wallpaper is beige");
    assert_eq!(result.class_id, "routine");
    assert!(!result.via_fast_path);
    assert_eq!(result.confidence, 0.7);
}

#[test]
fn model_fallback_creates_new_class_from_label() {
    let conn = memory::open_memory_in_memory().unwrap();
    classify::seed_default_classes(&conn).unwrap();
    let model = ScriptedClient::new(vec![
        r#"{"class_id": null, "label": "medication check", "confidence": 0.6}"#.to_string(),
    ]);
    let result = classify::classify(&conn, Some(&model), "the wallpaper is beige");
    assert_eq!(result.class_id, "medication_check");
    assert!(classify::known_class_ids(&conn).unwrap().contains(&"medication_check".to_string()));
}

#[test]
fn malformed_model_output_degrades_to_unknown() {
    let conn = memory::open_memory_in_memory().unwrap();
    classify::seed_default_classes(&conn).unwrap();
    let model = ScriptedClient::new(vec!["sure, class is crisis maybe".to_string()]);
    let result = classify::classify(&conn, Some(&model), "the wallpaper is beige");
    assert_eq!(result.class_id, "unknown");
    assert_eq!(result.confidence, 0.0);
}
