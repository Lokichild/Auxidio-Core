use rusqlite::Connection;

use crate::memory;

// Optimal bands and failure ceilings per metric family, used to fold telemetry
// into C_self per the provisional glossary (Vision Review 4.7). Open testing
// parameters: config-level once the config layer grows selfmon keys.
pub struct MetricBand {
    pub metric: &'static str,
    pub optimal_max: f64,
    pub failure_ceiling: f64,
}

pub const DEFAULT_BANDS: [MetricBand; 2] = [
    MetricBand { metric: "temp_c", optimal_max: 60.0, failure_ceiling: 95.0 },
    MetricBand { metric: "memory_pct", optimal_max: 70.0, failure_ceiling: 100.0 },
];

pub fn c_self(conn: &Connection, bands: &[MetricBand]) -> f64 {
    let mut deviations = vec![];
    for band in bands {
        if let Ok(Some(value)) = memory::latest_telemetry(conn, band.metric) {
            let span = (band.failure_ceiling - band.optimal_max).max(1e-9);
            let deviation = ((value - band.optimal_max) / span).clamp(0.0, 1.0);
            deviations.push(deviation);
        }
    }
    if deviations.is_empty() {
        return 100.0;
    }
    let mean = deviations.iter().sum::<f64>() / deviations.len() as f64;
    100.0 - 100.0 * mean
}

pub fn stressor_weight(problem: &memory::ProblemRecord) -> f64 {
    let level = problem.maslow_level.max(1) as f64;
    problem.m_impact as f64 / level
}

pub fn c_others(conn: &Connection, target_stress_sum: f64, local_mvt: f64) -> f64 {
    let problems = memory::open_problem_records(conn).unwrap_or_default();
    let observed: f64 = problems.iter().map(stressor_weight).sum();
    let span = local_mvt.max(1e-9);
    ((observed - target_stress_sum).abs() / span * 100.0).clamp(0.0, 100.0)
}

pub fn priority_score(problem: &memory::ProblemRecord, c_self: f64, c_others: f64) -> f64 {
    let level = problem.maslow_level.max(1) as f64;
    (c_self + c_others * problem.m_impact as f64) / level
}

pub fn ranked_open_problems(conn: &Connection, target_stress_sum: f64, local_mvt: f64) -> Vec<(memory::ProblemRecord, f64)> {
    let self_score = c_self(conn, &DEFAULT_BANDS);
    let others_score = c_others(conn, target_stress_sum, local_mvt);
    let mut problems = memory::open_problem_records(conn).unwrap_or_default();
    let mut scored: Vec<(memory::ProblemRecord, f64)> = problems
        .drain(..)
        .map(|problem| {
            let score = priority_score(&problem, self_score, others_score);
            (problem, score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored
}
