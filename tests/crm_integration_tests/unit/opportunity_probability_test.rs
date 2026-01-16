///! Unit Tests for CRM Opportunity Probability Logic

// =============================================================================
// OPPORTUNITY PROBABILITY CALCULATION LOGIC
// =============================================================================

fn calculate_probability_for_stage(stage: &str) -> i32 {
    match stage.to_lowercase().as_str() {
        "new" => 10,
        "qualified" => 50,
        "proposal" => 75,
        "negotiation" => 85,
        "closed won" => 100,
        "closed lost" => 0,
        _ => 25, // Default probability for unknown stages
    }
}

fn calculate_probability_adjustment(
    days_since_creation: i64,
    days_until_close_date: Option<i64>,
    deal_size_multiplier: f64,
    contact_influence_score: i32,
) -> i32 {
    let mut adjustment = 0;

    // Adjust based on age of opportunity
    if days_since_creation > 180 {
        // Opportunity is quite old, probability might decrease
        adjustment -= 10;
    } else if days_since_creation < 30 {
        // New opportunity, slight boost for freshness
        adjustment += 5;
    }

    // Adjust based on time to close date
    if let Some(days) = days_until_close_date {
        if days < 30 {
            // Approaching close date, adjust based on urgency
            if days < 7 {
                adjustment += 10; // High urgency
            } else {
                adjustment += 5; // Moderate urgency
            }
        } else if days > 365 {
            // Very distant close date, might indicate uncertainty
            adjustment -= 5;
        }
    }

    // Adjust based on deal size (larger deals might be harder to close)
    if deal_size_multiplier > 2.0 {
        adjustment -= 10; // Large deal, harder to close
    } else if deal_size_multiplier < 0.5 {
        adjustment += 5; // Small deal, easier to close
    }

    // Adjust based on contact influence
    if contact_influence_score > 80 {
        adjustment += 15; // High influence contact
    } else if contact_influence_score > 50 {
        adjustment += 5;  // Medium influence contact
    } else if contact_influence_score < 20 {
        adjustment -= 10; // Low influence contact
    }

    adjustment
}

fn calculate_adjusted_probability(
    base_stage_probability: i32,
    days_since_creation: i64,
    days_until_close_date: Option<i64>,
    deal_size_multiplier: f64,
    contact_influence_score: i32,
) -> i32 {
    let base_prob = base_stage_probability.max(0).min(100);
    let adjustment = calculate_probability_adjustment(
        days_since_creation,
        days_until_close_date,
        deal_size_multiplier,
        contact_influence_score,
    );
    
    let adjusted = base_prob + adjustment;
    adjusted.max(0).min(100) // Clamp between 0 and 100
}

// =============================================================================
// STAGE-BASED PROBABILITY TESTS
// =============================================================================

#[tokio::test]
async fn probability_calculation_new_stage() {
    let probability = calculate_probability_for_stage("new");
    assert_eq!(probability, 10, "New stage should have 10% probability");
}

#[tokio::test]
async fn probability_calculation_qualified_stage() {
    let probability = calculate_probability_for_stage("qualified");
    assert_eq!(probability, 50, "Qualified stage should have 50% probability");
}

#[tokio::test]
async fn probability_calculation_proposal_stage() {
    let probability = calculate_probability_for_stage("proposal");
    assert_eq!(probability, 75, "Proposal stage should have 75% probability");
}

#[tokio::test]
async fn probability_calculation_negotiation_stage() {
    let probability = calculate_probability_for_stage("negotiation");
    assert_eq!(probability, 85, "Negotiation stage should have 85% probability");
}

#[tokio::test]
async fn probability_calculation_closed_won_stage() {
    let probability = calculate_probability_for_stage("closed won");
    assert_eq!(probability, 100, "Closed won stage should have 100% probability");
}

#[tokio::test]
async fn probability_calculation_closed_lost_stage() {
    let probability = calculate_probability_for_stage("closed lost");
    assert_eq!(probability, 0, "Closed lost stage should have 0% probability");
}

#[tokio::test]
async fn probability_calculation_case_insensitive() {
    assert_eq!(calculate_probability_for_stage("NEW"), 10);
    assert_eq!(calculate_probability_for_stage("New"), 10);
    assert_eq!(calculate_probability_for_stage("qualified"), 50);
    assert_eq!(calculate_probability_for_stage("QUALIFIED"), 50);
    assert_eq!(calculate_probability_for_stage("Qualified"), 50);
}

#[tokio::test]
async fn probability_calculation_unknown_stage_defaults() {
    let probability = calculate_probability_for_stage("unknown_stage");
    assert_eq!(probability, 25, "Unknown stage should default to 25%");
    
    let probability = calculate_probability_for_stage("invalid");
    assert_eq!(probability, 25, "Invalid stage should default to 25%");
    
    let probability = calculate_probability_for_stage("");
    assert_eq!(probability, 25, "Empty stage should default to 25%");
}

// =============================================================================
// PROBABILITY ADJUSTMENT LOGIC TESTS
// =============================================================================

#[tokio::test]
async fn probability_adjustment_based_on_age() {
    // Old opportunity (>180 days) gets negative adjustment
    let adjustment = calculate_probability_adjustment(200, None, 1.0, 50);
    assert_eq!(adjustment, -10, "Old opportunity should get -10 adjustment");

    // New opportunity (<30 days) gets positive adjustment
    let adjustment = calculate_probability_adjustment(15, None, 1.0, 50);
    assert_eq!(adjustment, 5, "New opportunity should get +5 adjustment");

    // Medium-aged opportunity gets no adjustment
    let adjustment = calculate_probability_adjustment(100, None, 1.0, 50);
    assert_eq!(adjustment, 0, "Medium-aged opportunity should get no adjustment");
}

#[tokio::test]
async fn probability_adjustment_based_on_close_date_urgency() {
    // High urgency (closing in <7 days)
    let adjustment = calculate_probability_adjustment(90, Some(5), 1.0, 50);
    assert_eq!(adjustment, 10, "High urgency should get +10 adjustment");

    // Moderate urgency (closing in 7-30 days)
    let adjustment = calculate_probability_adjustment(90, Some(20), 1.0, 50);
    assert_eq!(adjustment, 5, "Moderate urgency should get +5 adjustment");

    // Distant close date (>365 days)
    let adjustment = calculate_probability_adjustment(90, Some(400), 1.0, 50);
    assert_eq!(adjustment, -5, "Distant close date should get -5 adjustment");

    // Normal close date (30-365 days)
    let adjustment = calculate_probability_adjustment(90, Some(100), 1.0, 50);
    assert_eq!(adjustment, 0, "Normal close date should get no adjustment");
}

#[tokio::test]
async fn probability_adjustment_based_on_deal_size() {
    // Large deal gets negative adjustment
    let adjustment = calculate_probability_adjustment(90, Some(100), 3.0, 50);
    assert_eq!(adjustment, -10, "Large deal should get -10 adjustment");

    // Small deal gets positive adjustment
    let adjustment = calculate_probability_adjustment(90, Some(100), 0.3, 50);
    assert_eq!(adjustment, 5, "Small deal should get +5 adjustment");

    // Medium deal gets no adjustment
    let adjustment = calculate_probability_adjustment(90, Some(100), 1.0, 50);
    assert_eq!(adjustment, 0, "Medium deal should get no adjustment");
}

#[tokio::test]
async fn probability_adjustment_based_on_contact_influence() {
    // High influence contact gets positive adjustment
    let adjustment = calculate_probability_adjustment(90, Some(100), 1.0, 90);
    assert_eq!(adjustment, 15, "High influence contact should get +15 adjustment");

    // Medium influence contact gets small positive adjustment
    let adjustment = calculate_probability_adjustment(90, Some(100), 1.0, 60);
    assert_eq!(adjustment, 5, "Medium influence contact should get +5 adjustment");

    // Low influence contact gets negative adjustment
    let adjustment = calculate_probability_adjustment(90, Some(100), 1.0, 10);
    assert_eq!(adjustment, -10, "Low influence contact should get -10 adjustment");

    // Medium influence contact (boundary)
    let adjustment = calculate_probability_adjustment(90, Some(100), 1.0, 50);
    assert_eq!(adjustment, 5, "Medium influence contact should get +5 adjustment");
}

// =============================================================================
// COMBINED PROBABILITY CALCULATION TESTS
// =============================================================================

#[tokio::test]
async fn combined_probability_calculation_medium_opportunity() {
    let base_prob = calculate_probability_for_stage("qualified"); // 50%
    let adjusted_prob = calculate_adjusted_probability(
        base_prob,    // 50
        60,           // Days since creation (medium age)
        Some(90),     // Days until close (normal urgency)
        1.0,          // Deal size (normal)
        55,           // Contact influence (medium)
    );
    
    // Base: 50, Adjustment: +5 (medium influence) = 55
    assert_eq!(adjusted_prob, 55, "Medium opportunity should have adjusted probability");
}

#[tokio::test]
async fn combined_probability_calculation_high_value_opportunity() {
    let base_prob = calculate_probability_for_stage("negotiation"); // 85%
    let adjusted_prob = calculate_adjusted_probability(
        base_prob,    // 85
        45,           // Days since creation (fresh)
        Some(14),     // Days until close (moderate urgency)
        0.8,          // Deal size (slightly small)
        85,           // Contact influence (high)
    );
    
    // Base: 85, Adjustment: +5 (fresh) +5 (urgency) +5 (small deal) +15 (high influence) = +30
    // But clamped at 100: min(115, 100) = 100
    assert_eq!(adjusted_prob, 100, "High-value opportunity should max out at 100%");
}

#[tokio::test]
async fn combined_probability_calculation_low_value_opportunity() {
    let base_prob = calculate_probability_for_stage("proposal"); // 75%
    let adjusted_prob = calculate_adjusted_probability(
        base_prob,    // 75
        200,          // Days since creation (very old)
        Some(500),    // Days until close (very distant)
        2.5,          // Deal size (large)
        15,           // Contact influence (low)
    );
    
    // Base: 75, Adjustment: -10 (old) -5 (distant) -10 (large deal) -10 (low influence) = -35
    // Result: 75 - 35 = 40
    assert_eq!(adjusted_prob, 40, "Low-value opportunity should have reduced probability");
}

#[tokio::test]
async fn combined_probability_calculation_closed_won_fixed() {
    let base_prob = calculate_probability_for_stage("closed won"); // 100%
    let adjusted_prob = calculate_adjusted_probability(
        base_prob,    // 100
        300,          // Days since creation (old)
        Some(5),      // Days until close (urgent)
        3.0,          // Deal size (large)
        10,           // Contact influence (low)
    );
    
    // Even with adjustments, closed won should remain at 100%
    assert_eq!(adjusted_prob, 100, "Closed won opportunity should remain at 100%");
}

#[tokio::test]
async fn combined_probability_calculation_closed_lost_fixed() {
    let base_prob = calculate_probability_for_stage("closed lost"); // 0%
    let adjusted_prob = calculate_adjusted_probability(
        base_prob,    // 0
        300,          // Days since creation (old)
        Some(5),      // Days until close (urgent)
        3.0,          // Deal size (large)
        10,           // Contact influence (low)
    );
    
    // Even with adjustments, closed lost should remain at 0%
    assert_eq!(adjusted_prob, 0, "Closed lost opportunity should remain at 0%");
}

// =============================================================================
// PROBABILITY BOUNDARY TESTS
// =============================================================================

#[tokio::test]
async fn probability_calculation_clamped_at_maximum() {
    let base_prob = calculate_probability_for_stage("negotiation"); // 85%
    let adjusted_prob = calculate_adjusted_probability(
        base_prob,    // 85
        15,           // Fresh (adjustment +5)
        Some(5),      // Urgent (adjustment +10)
        0.2,          // Small deal (adjustment +5)
        95,           // High influence (adjustment +15)
    );
    
    // Total: 85 + 5 + 10 + 5 + 15 = 120, but clamped at 100
    assert_eq!(adjusted_prob, 100, "Probability should be clamped at 100%");
}

#[tokio::test]
async fn probability_calculation_clamped_at_minimum() {
    let base_prob = calculate_probability_for_stage("new"); // 10%
    let adjusted_prob = calculate_adjusted_probability(
        base_prob,    // 10
        200,          // Old (adjustment -10)
        Some(400),    // Distant (adjustment -5)
        3.0,          // Large deal (adjustment -10)
        5,            // Low influence (adjustment -10)
    );
    
    // Total: 10 - 10 - 5 - 10 - 10 = -25, but clamped at 0
    assert_eq!(adjusted_prob, 0, "Probability should be clamped at 0%");
}

#[tokio::test]
async fn probability_calculation_handles_extreme_adjustments() {
    // Test with extreme positive adjustment
    let adjusted_prob = calculate_adjusted_probability(10, 10, Some(5), 0.1, 95);
    // Base: 10, Adjustment: +5 (fresh) +10 (urgent) +5 (small) +15 (high influence) = +35
    // Total: 45, within bounds
    assert_eq!(adjusted_prob, 45, "Positive extreme should be calculated correctly");
    
    // Test with extreme negative adjustment
    let adjusted_prob = calculate_adjusted_probability(90, 200, Some(400), 3.0, 5);
    // Base: 90, Adjustment: -10 (old) -5 (distant) -10 (large) -10 (low influence) = -35
    // Total: 55, within bounds
    assert_eq!(adjusted_prob, 55, "Negative extreme should be calculated correctly");
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[tokio::test]
async fn probability_calculation_handles_zero_values() {
    let adjusted_prob = calculate_adjusted_probability(0, 0, None, 0.0, 0);
    assert_eq!(adjusted_prob, 0, "Zero values should result in 0 probability");
}

#[tokio::test]
async fn probability_calculation_handles_maximum_values() {
    let adjusted_prob = calculate_adjusted_probability(100, 1000, Some(1000), 10.0, 100);
    // Base: 100, Adjustment: -10 (old) -5 (distant) -10 (large) +15 (high influence) = -10
    // Total: 90, within bounds
    assert_eq!(adjusted_prob, 90, "Maximum values should be handled correctly");
}

#[tokio::test]
async fn probability_calculation_handles_negative_base_probabilities() {
    // Though this shouldn't happen in practice, test boundary behavior
    let adjusted_prob = calculate_adjusted_probability(-10, 100, Some(100), 1.0, 50);
    // Base gets clamped to 0, then adjustment applied
    // Adjustment: 0 (age) + 0 (close) + 0 (deal) + 5 (influence) = +5
    // Total: 0 + 5 = 5
    assert_eq!(adjusted_prob, 5, "Negative base probabilities should be clamped to 0 first");
}

#[tokio::test]
async fn probability_calculation_handles_over_100_base_probabilities() {
    // Though this shouldn't happen in practice, test boundary behavior
    let adjusted_prob = calculate_adjusted_probability(150, 100, Some(100), 1.0, 50);
    // Base gets clamped to 100, then adjustment applied
    // Adjustment: 0 (age) + 0 (close) + 0 (deal) + 5 (influence) = +5
    // Total: 100 + 5 = 105, but clamped to 100
    assert_eq!(adjusted_prob, 100, "Over-100 base probabilities should be clamped to 100 first");
}