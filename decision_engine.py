# Auxidio Core Decision Engine
# Phase 2 - Tiered confidence gating

WEIGHTS = {
    "accuracy": 0.5,
    "net_collective_impact": 0.3,
    "net_individual_impact": 0.2
}

exponent=1.5

def get_confidence_tier(confidence_rating):
    """
    Determines the action tier based on confidence_rating alone,
    since truth/accuracy functions as a gate on the whole decision.
    """
    if confidence_rating >= 0.86:
        return "confident_decision"
    elif confidence_rating >= 0.56:
        return "wary_but_secure"
    elif confidence_rating >= 0.31:
        return "proceed_only_if_necessary"
    else:
        return "gather_more_data"

def evaluate_decision(confidence_rating, net_collective_impact, net_individual_impact):
    """
    Evaluates a single decision based on three weighted criteria.
    
    confidence_rating: float 0-1, how accurate/reliable is the information
    net_collective_impact: float -1 to 1, net benefit or harm to the collective
    net_individual_impact: float -1 to 1, net benefit or harm to the individual
    
    Returns weighted score and confidence tier.
    """
    tier = get_confidence_tier(confidence_rating)
    
    weighted_score = (
        confidence_rating * WEIGHTS["accuracy"] +
        net_collective_impact * WEIGHTS["net_collective_impact"] +
        net_individual_impact * WEIGHTS["net_individual_impact"]
    )

    return {
        "score": round(weighted_score, 2),
        "tier": tier
    }

def tier_coefficient(tier, exponent):
    """
    Converts a severity tier into a weighted coefficient.
    exponent is an open testing parameter, default 1.5 as a starting midpoint.
    """
    return tier ** exponent

def calculate_group_impact(group, exponent):
    """
    Calculates net impact for a single group.
    Positive result means net benefit, negative means net harm.
    """
    benefit = tier_coefficient(group["benefit_tier"], exponent) if group["benefit_tier"] > 0 else 0
    harm = tier_coefficient(group["harm_tier"], exponent) if group["harm_tier"] > 0 else 0
    net_impact = (benefit - harm) * group["size"]
    return net_impact

def score_decision(confidence_rating, groups, proportionality_threshold=2.0, exponent=exponent):

    """
    Scores a proposed decision across any number of affected groups.
    
    confidence_rating: float 0-1, gates the entire comparison
    groups: list of dicts, each with name, size, harm_tier, benefit_tier
    proportionality_threshold: open testing parameter, default 2.0
    
    Returns a recommendation tier and reasoning.
    """
    # Gate on confidence first
    confidence_tier = get_confidence_tier(confidence_rating)
    if confidence_tier == "gather_more_data":
        return {
            "recommendation": "halt",
            "reason": "Insufficient confidence in situational accuracy",
            "confidence_tier": confidence_tier,
            "flagged_groups": []
        }
    
    # Calculate net impact for each group
    group_results = []
    for group in groups:
        impact = calculate_group_impact(group, exponent)
        group_results.append({
            "name": group["name"],
            "impact": impact,
            "harm_tier": group["harm_tier"],
            "benefit_tier": group["benefit_tier"],
            "size": group["size"]
        })
    
    # Separate harmed and benefited groups
    harmed_groups = [g for g in group_results if g["impact"] < 0]
    benefited_groups = [g for g in group_results if g["impact"] > 0]
    
    # Calculate total collective benefit
    total_benefit = sum(g["impact"] for g in benefited_groups)
    
    # Check proportionality for each harmed group
    disproportionate_groups = []
    requires_mitigation_groups = []
    
    for group in harmed_groups:
        harm_magnitude = abs(group["impact"])
        if total_benefit <= 0:
            disproportionate_groups.append(group["name"])
        elif harm_magnitude > (total_benefit * proportionality_threshold):
            disproportionate_groups.append(group["name"])
        else:
            requires_mitigation_groups.append(group["name"])
    
    # Assign recommendation tier
    if disproportionate_groups:
        recommendation = "reject"
        reason = f"Harm to {disproportionate_groups} is disproportionate to collective benefit"
    elif requires_mitigation_groups:
        recommendation = "conditional_proceed"
        reason = f"Proceed only with adequate mitigation for {requires_mitigation_groups}"
    else:
        recommendation = "full_proceed"
        reason = "Net benefit is proportionate across all groups"
    
    return {
        "recommendation": recommendation,
        "reason": reason,
        "confidence_tier": confidence_tier,
        "flagged_groups": disproportionate_groups + requires_mitigation_groups,
        "group_results": group_results
    }

if __name__ == "__main__":
    result = evaluate_decision(
        confidence_rating=0.9,
        net_collective_impact=0.9,
        net_individual_impact=-0.9
    )
    print(f"Decision score: {result['score']}")
    print(f"Confidence tier: {result['tier']}")
