use serde::{Deserialize, Serialize};

use crate::py_list_repr;
use crate::py_round;

pub const WEIGHT_ACCURACY: f64 = 0.5;
pub const WEIGHT_COLLECTIVE: f64 = 0.3;
pub const WEIGHT_INDIVIDUAL: f64 = 0.2;
pub const EXPONENT: f64 = 1.5;
pub const DEFAULT_PROPORTIONALITY_THRESHOLD: f64 = 2.0;

pub fn get_confidence_tier(confidence_rating: f64) -> &'static str {
    if confidence_rating >= 0.86 {
        "confident_decision"
    } else if confidence_rating >= 0.56 {
        "wary_but_secure"
    } else if confidence_rating >= 0.31 {
        "proceed_only_if_necessary"
    } else {
        "gather_more_data"
    }
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct DecisionResult {
    pub score: f64,
    pub tier: String,
}

pub fn evaluate_decision(
    confidence_rating: f64,
    net_collective_impact: f64,
    net_individual_impact: f64,
) -> DecisionResult {
    let tier = get_confidence_tier(confidence_rating);
    let weighted_score = confidence_rating * WEIGHT_ACCURACY
        + net_collective_impact * WEIGHT_COLLECTIVE
        + net_individual_impact * WEIGHT_INDIVIDUAL;
    DecisionResult {
        score: py_round(weighted_score, 2),
        tier: tier.to_string(),
    }
}

pub fn tier_coefficient(tier: f64, exponent: f64) -> f64 {
    tier.powf(exponent)
}

// Group numeric fields keep their original JSON number type (int stays int),
// matching the Python reference, which echoes whatever type it was given.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Group {
    pub name: String,
    pub size: serde_json::Number,
    pub harm_tier: serde_json::Number,
    pub benefit_tier: serde_json::Number,
}

impl Group {
    pub fn size_f64(&self) -> f64 {
        self.size.as_f64().unwrap_or(0.0)
    }
    pub fn harm_f64(&self) -> f64 {
        self.harm_tier.as_f64().unwrap_or(0.0)
    }
    pub fn benefit_f64(&self) -> f64 {
        self.benefit_tier.as_f64().unwrap_or(0.0)
    }
}

pub fn calculate_group_impact(group: &Group, exponent: f64) -> f64 {
    let benefit = if group.benefit_f64() > 0.0 {
        tier_coefficient(group.benefit_f64(), exponent)
    } else {
        0.0
    };
    let harm = if group.harm_f64() > 0.0 {
        tier_coefficient(group.harm_f64(), exponent)
    } else {
        0.0
    };
    (benefit - harm) * group.size_f64()
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct GroupResult {
    pub name: String,
    pub impact: f64,
    pub harm_tier: serde_json::Number,
    pub benefit_tier: serde_json::Number,
    pub size: serde_json::Number,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct ScoreResult {
    pub recommendation: String,
    pub reason: String,
    pub confidence_tier: String,
    pub flagged_groups: Vec<String>,
    pub group_results: Vec<GroupResult>,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct HaltResult {
    pub recommendation: String,
    pub reason: String,
    pub confidence_tier: String,
    pub flagged_groups: Vec<String>,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
#[serde(untagged)]
pub enum ScoreOutcome {
    Halt(HaltResult),
    Scored(ScoreResult),
}

pub fn score_decision(
    confidence_rating: f64,
    groups: &[Group],
    proportionality_threshold: f64,
    exponent: f64,
) -> ScoreOutcome {
    let confidence_tier = get_confidence_tier(confidence_rating);
    if confidence_tier == "gather_more_data" {
        return ScoreOutcome::Halt(HaltResult {
            recommendation: "halt".to_string(),
            reason: "Insufficient confidence in situational accuracy".to_string(),
            confidence_tier: confidence_tier.to_string(),
            flagged_groups: vec![],
        });
    }

    let group_results: Vec<GroupResult> = groups
        .iter()
        .map(|group| GroupResult {
            name: group.name.clone(),
            impact: calculate_group_impact(group, exponent),
            harm_tier: group.harm_tier.clone(),
            benefit_tier: group.benefit_tier.clone(),
            size: group.size.clone(),
        })
        .collect();

    let harmed_groups: Vec<&GroupResult> = group_results.iter().filter(|g| g.impact < 0.0).collect();
    let benefited_groups: Vec<&GroupResult> = group_results.iter().filter(|g| g.impact > 0.0).collect();

    let total_benefit: f64 = benefited_groups.iter().map(|g| g.impact).sum();

    let mut disproportionate_groups: Vec<String> = vec![];
    let mut requires_mitigation_groups: Vec<String> = vec![];

    for group in harmed_groups {
        let harm_magnitude = group.impact.abs();
        if total_benefit <= 0.0 {
            disproportionate_groups.push(group.name.clone());
        } else if harm_magnitude > total_benefit * proportionality_threshold {
            disproportionate_groups.push(group.name.clone());
        } else {
            requires_mitigation_groups.push(group.name.clone());
        }
    }

    let (recommendation, reason) = if !disproportionate_groups.is_empty() {
        (
            "reject".to_string(),
            format!(
                "Harm to {} is disproportionate to collective benefit",
                py_list_repr(&disproportionate_groups)
            ),
        )
    } else if !requires_mitigation_groups.is_empty() {
        (
            "conditional_proceed".to_string(),
            format!(
                "Proceed only with adequate mitigation for {}",
                py_list_repr(&requires_mitigation_groups)
            ),
        )
    } else {
        (
            "full_proceed".to_string(),
            "Net benefit is proportionate across all groups".to_string(),
        )
    };

    let mut flagged_groups = disproportionate_groups;
    flagged_groups.extend(requires_mitigation_groups);

    ScoreOutcome::Scored(ScoreResult {
        recommendation,
        reason,
        confidence_tier: confidence_tier.to_string(),
        flagged_groups,
        group_results,
    })
}
