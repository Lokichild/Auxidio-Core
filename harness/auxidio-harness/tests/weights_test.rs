use auxidio_harness::model::{ModelError, ScriptedClient};
use auxidio_harness::weights;
use harness_core::ChatMessage;

#[test]
fn extracts_and_validates() {
    let model = ScriptedClient::new(vec![
        r#"{"confidence_rating": 0.8, "net_collective_impact": 0.4, "net_individual_impact": -0.2}"#.to_string(),
    ]);
    let inputs = weights::extract_decision_inputs(&model, "should we move the appointment", "stable", "").unwrap();
    assert_eq!(inputs.confidence_rating, 0.8);
    assert_eq!(inputs.net_collective_impact, 0.4);
    assert_eq!(inputs.net_individual_impact, -0.2);
}

#[test]
fn out_of_range_is_rejected() {
    let model = ScriptedClient::new(vec![
        r#"{"confidence_rating": 1.5, "net_collective_impact": 0.4, "net_individual_impact": 0.0}"#.to_string(),
    ]);
    assert!(weights::extract_decision_inputs(&model, "x", "stable", "").is_err());
}

#[test]
fn model_failure_is_rejected() {
    struct Down;
    impl auxidio_harness::model::ModelClient for Down {
        fn chat(&self, _system: &str, _history: &[ChatMessage]) -> Result<String, ModelError> {
            Err(ModelError::Unreachable("down".into()))
        }
    }
    assert!(weights::extract_decision_inputs(&Down, "x", "stable", "").is_err());
    let fallback = weights::conservative_inputs();
    assert!(weights::valid(&fallback));
    assert_eq!(fallback.confidence_rating, 0.3);
}
