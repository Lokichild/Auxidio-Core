use auxidio_core::cases;
use auxidio_core::decision;
use auxidio_core::emotion;
use serde_json::Value;

fn load(name: &str) -> Value {
    let path = format!("{}/tests/vectors/{}", env!("CARGO_MANIFEST_DIR"), name);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("missing vector file {} : {}", path, e));
    serde_json::from_str(&content).expect("vector file must parse")
}

fn is_iso_timestamp(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 19 {
        return false;
    }
    let pat = |i: usize, c: u8| b[i] == c;
    let digit = |i: usize| b[i].is_ascii_digit();
    digit(0) && digit(1) && digit(2) && digit(3)
        && pat(4, b'-')
        && digit(5) && digit(6)
        && pat(7, b'-')
        && digit(8) && digit(9)
        && pat(10, b'T')
        && digit(11) && digit(12)
        && pat(13, b':')
        && digit(14) && digit(15)
        && pat(16, b':')
        && digit(17) && digit(18)
}

fn mask_value(value: Value) -> Value {
    match value {
        Value::String(s) => Value::String(mask_string(&s)),
        Value::Array(items) => Value::Array(items.into_iter().map(mask_value).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| {
                    let masked = if k == "timestamp" {
                        Value::String("<TS>".to_string())
                    } else {
                        mask_value(v)
                    };
                    (k, masked)
                })
                .collect(),
        ),
        other => other,
    }
}

fn mask_string(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if i + 19 <= bytes.len() && is_iso_timestamp(&s[i..]) {
            let mut end = i + 19;
            if end < bytes.len() && bytes[end] == b'.' {
                end += 1;
                while end < bytes.len() && bytes[end].is_ascii_digit() {
                    end += 1;
                }
            }
            out.push_str("<TS>");
            i = end;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

#[test]
fn decision_tiers_match_reference() {
    let vectors = load("decision.json");
    for case in vectors["tier"].as_array().unwrap() {
        let input = case["input"].as_f64().unwrap();
        let expected = case["expected"].as_str().unwrap();
        assert_eq!(
            decision::get_confidence_tier(input),
            expected,
            "tier mismatch at input {}",
            input
        );
    }
}

#[test]
fn decision_evaluate_matches_reference() {
    let vectors = load("decision.json");
    for case in vectors["evaluate"].as_array().unwrap() {
        let c = case["confidence"].as_f64().unwrap();
        let col = case["collective"].as_f64().unwrap();
        let ind = case["individual"].as_f64().unwrap();
        let actual = serde_json::to_value(decision::evaluate_decision(c, col, ind)).unwrap();
        assert_eq!(actual, case["expected"], "evaluate mismatch at {:?}", case);
    }
}

#[test]
fn decision_coefficients_match_reference() {
    let vectors = load("decision.json");
    for case in vectors["tier_coefficient"].as_array().unwrap() {
        let tier = case["tier"].as_f64().unwrap();
        let exponent = case["exponent"].as_f64().unwrap();
        let actual = decision::tier_coefficient(tier, exponent);
        let expected = case["expected"].as_f64().unwrap();
        assert!(
            (actual - expected).abs() < 1e-12,
            "tier_coefficient mismatch at {:?}: {} vs {}",
            case,
            actual,
            expected
        );
    }
    for case in vectors["group_impact"].as_array().unwrap() {
        let group: decision::Group = serde_json::from_value(case["group"].clone()).unwrap();
        let exponent = case["exponent"].as_f64().unwrap();
        let actual = decision::calculate_group_impact(&group, exponent);
        let expected = case["expected"].as_f64().unwrap();
        assert!(
            (actual - expected).abs() < 1e-12,
            "group_impact mismatch at {:?}: {} vs {}",
            case,
            actual,
            expected
        );
    }
}

#[test]
fn decision_score_matches_reference() {
    let vectors = load("decision.json");
    for case in vectors["score_decision"].as_array().unwrap() {
        let confidence = case["confidence"].as_f64().unwrap();
        let groups: Vec<decision::Group> =
            serde_json::from_value(case["groups"].clone()).unwrap();
        let threshold = case["threshold"].as_f64().unwrap();
        let exponent = case["exponent"].as_f64().unwrap();
        let actual = serde_json::to_value(decision::score_decision(
            confidence, &groups, threshold, exponent,
        ))
        .unwrap();
        assert_eq!(actual, case["expected"], "score_decision mismatch at {:?}", case);
    }
}

#[test]
fn emotion_analyze_matches_reference() {
    let vectors = load("emotion.json");
    for case in vectors["analyze"].as_array().unwrap() {
        let current = case["current"].as_str().unwrap();
        let previous = case["previous"].as_str();
        let actual = serde_json::to_value(emotion::analyze_user_state(current, previous)).unwrap();
        assert_eq!(actual, case["expected"], "analyze mismatch at {:?}", case);
    }
}

#[test]
fn emotion_outward_matches_reference() {
    let vectors = load("emotion.json");
    for case in vectors["outward"].as_array().unwrap() {
        let current = case["current"].as_str().unwrap();
        let previous = case["previous"].as_str();
        let actual = emotion::run_outward_channel(current, previous);
        assert_eq!(actual, case["expected"], "outward mismatch at {:?}", case);
    }
    for case in vectors["style"].as_array().unwrap() {
        let state = case["state"].as_str().unwrap();
        let actual = emotion::get_communication_style(state);
        assert_eq!(actual, case["expected"], "style mismatch at {:?}", case);
    }
}

#[test]
fn cases_similarity_matches_reference() {
    let vectors = load("cases.json");
    for case in vectors["similarity"].as_array().unwrap() {
        let stored: serde_json::Map<String, Value> =
            serde_json::from_value(case["stored"].clone()).unwrap();
        let query: serde_json::Map<String, Value> =
            serde_json::from_value(case["query"].clone()).unwrap();
        let actual = cases::calculate_similarity(&stored, &query);
        let expected = case["expected"].as_f64().unwrap();
        assert!(
            (actual - expected).abs() < 1e-12,
            "similarity mismatch at {:?}: {} vs {}",
            case,
            actual,
            expected
        );
    }
}

#[test]
fn cases_db_script_matches_reference() {
    let vectors = load("cases.json");
    let conn = cases::open_in_memory().unwrap();

    for step in vectors["db_script"].as_array().unwrap() {
        let op = step["op"].as_str().unwrap();
        let args = &step["args"];
        let expected = &step["expected"];

        let actual: Value = match op {
            "store_case" => {
                let description = args["description"].as_str().unwrap();
                let factors = args["factors"].clone();
                let solution = args["solution"].as_str().unwrap();
                let outcome = args["outcome_score"].as_f64().unwrap();
                cases::store_case(&conn, description, &factors, solution, outcome).unwrap()
            }
            "find_similar_cases" => {
                let query: serde_json::Map<String, Value> =
                    serde_json::from_value(args["query"].clone()).unwrap();
                let min_sim = args["min_similarity"].as_f64().unwrap();
                let max_results = args["max_results"].as_u64().unwrap() as usize;
                let rows = cases::find_similar_cases(&conn, &query, min_sim, max_results).unwrap();
                Value::Array(rows)
            }
            "update_outcome" => {
                let case_id = args["case_id"].as_i64().unwrap();
                let new_result = args["new_result"].as_f64().unwrap();
                cases::update_outcome(&conn, case_id, new_result).unwrap()
            }
            "check_injection_history" => {
                let window = args["window_minutes"].as_i64().unwrap();
                Value::from(cases::check_injection_history(&conn, window).unwrap())
            }
            "get_security_log" => {
                let unreviewed = args["unreviewed_only"].as_bool().unwrap();
                let rows = cases::get_security_log(&conn, unreviewed).unwrap();
                Value::Array(rows)
            }
            "display_security_report" => {
                Value::from(cases::display_security_report(&conn).unwrap())
            }
            "mark_security_log_reviewed" => {
                let ids: Vec<i64> = serde_json::from_value(args["entry_ids"].clone()).unwrap();
                cases::mark_security_log_reviewed(&conn, &ids).unwrap()
            }
            other => panic!("unknown op {}", other),
        };

        let masked = mask_value(actual);
        assert_eq!(&masked, expected, "db script mismatch at step {:?}", step);
    }
}
