use harness_core::ChatMessage;

use crate::model::ModelClient;

#[derive(Clone, PartialEq, Debug)]
pub struct DecisionInputs {
    pub confidence_rating: f64,
    pub net_collective_impact: f64,
    pub net_individual_impact: f64,
}

// Conservative fallback when extraction is unavailable or malformed: low
// confidence lands the turn in gather_more_data, impacts neutral. The system
// asks instead of guessing.
pub fn conservative_inputs() -> DecisionInputs {
    DecisionInputs {
        confidence_rating: 0.3,
        net_collective_impact: 0.0,
        net_individual_impact: 0.0,
    }
}

pub fn valid(inputs: &DecisionInputs) -> bool {
    (0.0..=1.0).contains(&inputs.confidence_rating)
        && (-1.0..=1.0).contains(&inputs.net_collective_impact)
        && (-1.0..=1.0).contains(&inputs.net_individual_impact)
}

pub fn extract_decision_inputs(
    model: &dyn ModelClient,
    user_input: &str,
    user_state: &str,
    case_context: &str,
) -> Result<DecisionInputs, ()> {
    let system = format!(
        "You extract decision inputs for a weighted decision engine. Reply with ONLY JSON: {{\"confidence_rating\": number 0..1, \"net_collective_impact\": number -1..1, \"net_individual_impact\": number -1..1}}. confidence_rating is how reliable the available information is. Impacts are net benefit (positive) or net harm (negative) to the collective and to the individual. User state: {}. {}",
        user_state, case_context
    );
    let response = model
        .chat(&system, &[ChatMessage::user(user_input)])
        .map_err(|_| ())?;
    let parsed: serde_json::Value = serde_json::from_str(response.trim()).map_err(|_| ())?;
    let inputs = DecisionInputs {
        confidence_rating: parsed["confidence_rating"].as_f64().ok_or(())?,
        net_collective_impact: parsed["net_collective_impact"].as_f64().ok_or(())?,
        net_individual_impact: parsed["net_individual_impact"].as_f64().ok_or(())?,
    };
    if valid(&inputs) {
        Ok(inputs)
    } else {
        Err(())
    }
}
