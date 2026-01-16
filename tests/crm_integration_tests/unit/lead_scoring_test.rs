///! Unit Tests for CRM Lead Scoring Logic

use echo_backend::modules::crm_integration::models::Lead as CrmLead;
use chrono::Utc;

// =============================================================================
// LEAD SCORING LOGIC
// =============================================================================

fn calculate_lead_score(lead: &CrmLead) -> i32 {
    let mut score = 50; // Base score

    // Business email increases score
    if is_business_email(&lead.email) {
        score += 20;
    } else {
        score -= 10; // Personal email decreases score
    }

    // Source affects score
    match lead.source.as_str() {
        "Direct" | "Referral" | "Event" => score += 15,
        "Website" | "Advertisement" => score += 5,
        "Cold Call" | "Trade Show" => score -= 5,
        _ => {} // Neutral sources don't change score
    }

    // Known company increases score
    if !["", "Personal", "Individual", "Self"].contains(&lead.company.as_str()) {
        score += 10;
    }

    // Title affects score
    let high_value_titles = [
        "CEO", "CTO", "CFO", "CMO", "VP", "Director", "Manager", 
        "Head of", "President", "Founder", "Owner"
    ];
    
    if let Some(title) = &lead.title {
        if high_value_titles.iter().any(|t: &&str| title.to_uppercase().contains(t)) {
            score += 15;
        }
    }

    // Clamp score between 0 and 100
    score.clamp(0, 100)
}

fn is_business_email(email: &str) -> bool {
    let domain = extract_domain_from_email(email);
    let personal_domains = [
        "gmail.com", "yahoo.com", "hotmail.com", "outlook.com", 
        "aol.com", "icloud.com", "protonmail.com", "mail.com",
        "live.com", "msn.com", "ymail.com", "rocketmail.com"
    ];
    
    !personal_domains.contains(&domain.as_str())
}

fn extract_domain_from_email(email: &str) -> String {
    email.split('@').nth(1).unwrap_or("").to_lowercase()
}

// =============================================================================
// LEAD SCORING TESTS
// =============================================================================

#[tokio::test]
async fn lead_scoring_business_email_gets_bonus() {
    let lead = CrmLead {
        id: "lead_1".to_string(),
        first_name: "Business".to_string(),
        last_name: "User".to_string(),
        email: "business.user@company.com".to_string(), // Business email
        phone: None,
        company: "Company Inc".to_string(),
        status: "New".to_string(),
        source: "Website".to_string(),
        title: Some("Manager".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let score = calculate_lead_score(&lead);
    assert!(score > 50, "Business email should increase score");
}

#[tokio::test]
async fn lead_scoring_personal_email_gets_penalty() {
    let lead = CrmLead {
        id: "lead_2".to_string(),
        first_name: "Personal".to_string(),
        last_name: "User".to_string(),
        email: "personal.user@gmail.com".to_string(), // Personal email
        phone: None,
        company: "Personal".to_string(),
        status: "New".to_string(),
        source: "Website".to_string(),
        title: Some("Engineer".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let score = calculate_lead_score(&lead);
    assert!(score < 50, "Personal email should decrease score");
}

#[tokio::test]
async fn lead_scoring_direct_source_gets_high_bonus() {
    let lead = CrmLead {
        id: "lead_3".to_string(),
        first_name: "Direct".to_string(),
        last_name: "Source".to_string(),
        email: "direct.source@business.com".to_string(),
        phone: None,
        company: "Enterprise Corp".to_string(),
        status: "New".to_string(),
        source: "Direct".to_string(), // High-value source
        title: Some("Director".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let score = calculate_lead_score(&lead);
    assert!(score >= 80, "Direct source should result in high score");
}

#[tokio::test]
async fn lead_scoring_referral_source_gets_bonus() {
    let lead = CrmLead {
        id: "lead_4".to_string(),
        first_name: "Referral".to_string(),
        last_name: "Source".to_string(),
        email: "referral.source@business.com".to_string(),
        phone: None,
        company: "Midsize Ltd".to_string(),
        status: "New".to_string(),
        source: "Referral".to_string(), // High-value source
        title: Some("Manager".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let score = calculate_lead_score(&lead);
    assert!(score >= 75, "Referral source should result in high score");
}

#[tokio::test]
async fn lead_scoring_cold_call_source_gets_penalty() {
    let lead = CrmLead {
        id: "lead_5".to_string(),
        first_name: "Cold".to_string(),
        last_name: "Caller".to_string(),
        email: "cold.caller@business.com".to_string(),
        phone: None,
        company: "Small Biz".to_string(),
        status: "New".to_string(),
        source: "Cold Call".to_string(), // Low-value source
        title: Some("Employee".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let score = calculate_lead_score(&lead);
    assert!(score < 50, "Cold call source should decrease score");
}

#[tokio::test]
async fn lead_scoring_known_company_gets_bonus() {
    let lead = CrmLead {
        id: "lead_6".to_string(),
        first_name: "Known".to_string(),
        last_name: "Company".to_string(),
        email: "known.company@business.com".to_string(),
        phone: None,
        company: "Fortune 500 Corp".to_string(), // Known company
        status: "New".to_string(),
        source: "Website".to_string(),
        title: Some("Executive".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let score = calculate_lead_score(&lead);
    assert!(score > 50, "Known company should increase score");
}

#[tokio::test]
async fn lead_scoring_executive_title_gets_bonus() {
    let lead = CrmLead {
        id: "lead_7".to_string(),
        first_name: "Executive".to_string(),
        last_name: "Title".to_string(),
        email: "executive.title@business.com".to_string(),
        phone: None,
        company: "Large Corp".to_string(),
        status: "New".to_string(),
        source: "Event".to_string(),
        title: Some("Chief Technology Officer".to_string()), // Executive title
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let score = calculate_lead_score(&lead);
    assert!(score >= 85, "Executive title should result in high score");
}

#[tokio::test]
async fn lead_scoring_manager_title_gets_bonus() {
    let lead = CrmLead {
        id: "lead_8".to_string(),
        first_name: "Manager".to_string(),
        last_name: "Title".to_string(),
        email: "manager.title@business.com".to_string(),
        phone: None,
        company: "Mid Corp".to_string(),
        status: "New".to_string(),
        source: "Website".to_string(),
        title: Some("Product Manager".to_string()), // Manager title
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let score = calculate_lead_score(&lead);
    assert!(score >= 70, "Manager title should result in increased score");
}

#[tokio::test]
async fn lead_scoring_engineer_title_gets_neutral_score() {
    let lead = CrmLead {
        id: "lead_9".to_string(),
        first_name: "Engineer".to_string(),
        last_name: "Title".to_string(),
        email: "engineer.title@business.com".to_string(),
        phone: None,
        company: "Tech Co".to_string(),
        status: "New".to_string(),
        source: "Website".to_string(),
        title: Some("Software Engineer".to_string()), // Technical title
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let score = calculate_lead_score(&lead);
    assert!(score >= 50 && score < 70, "Technical title should result in moderate score");
}

#[tokio::test]
async fn lead_scoring_combined_factors_result_in_appropriate_score() {
    let lead = CrmLead {
        id: "lead_10".to_string(),
        first_name: "Perfect".to_string(),
        last_name: "Lead".to_string(),
        email: "perfect.lead@enterprise.com".to_string(), // Business email
        phone: None,
        company: "Fortune 500 Enterprise".to_string(), // Known company
        status: "New".to_string(),
        source: "Referral".to_string(), // High-value source
        title: Some("Chief Executive Officer".to_string()), // Executive title
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let score = calculate_lead_score(&lead);
    assert!(score >= 95, "Perfect combination should result in very high score");
}

#[tokio::test]
async fn lead_scoring_low_value_combination_results_in_low_score() {
    let lead = CrmLead {
        id: "lead_11".to_string(),
        first_name: "Low".to_string(),
        last_name: "Value".to_string(),
        email: "low.value@gmail.com".to_string(), // Personal email
        phone: None,
        company: "Personal".to_string(), // Personal company
        status: "New".to_string(),
        source: "Cold Call".to_string(), // Low-value source
        title: Some("Student".to_string()), // Non-decision maker title
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let score = calculate_lead_score(&lead);
    assert!(score <= 30, "Low-value combination should result in low score");
}

// =============================================================================
// EMAIL CATEGORIZATION LOGIC TESTS
// =============================================================================

#[tokio::test]
async fn business_email_detection_identifies_business_emails() {
    let business_emails = vec![
        "user@company.com",
        "contact@business.org",
        "info@enterprise.net",
        "support@organization.co.uk",
        "admin@corporation.biz",
    ];

    for email in business_emails {
        assert!(is_business_email(email), "Email '{}' should be detected as business", email);
    }
}

#[tokio::test]
async fn business_email_detection_identifies_personal_emails() {
    let personal_emails = vec![
        "user@gmail.com",
        "contact@yahoo.com",
        "info@hotmail.com",
        "support@outlook.com",
        "admin@icloud.com",
    ];

    for email in personal_emails {
        assert!(!is_business_email(email), "Email '{}' should be detected as personal", email);
    }

    // Edge cases
    assert!(!is_business_email("user@"));
    assert!(!is_business_email("@gmail.com"));
    assert!(!is_business_email("user"));
    assert!(!is_business_email(""));
}

#[tokio::test]
async fn domain_extraction_works_correctly() {
    assert_eq!(extract_domain_from_email("user@company.com"), "company.com");
    assert_eq!(extract_domain_from_email("contact@business.org"), "business.org");
    assert_eq!(extract_domain_from_email("info@subdomain.example.net"), "subdomain.example.net");
    assert_eq!(extract_domain_from_email("user@COMPANY.COM"), "company.com"); // Case insensitive
    assert_eq!(extract_domain_from_email(""), "");
    assert_eq!(extract_domain_from_email("user@"), "");
    assert_eq!(extract_domain_from_email("@domain.com"), "domain.com");
}

// =============================================================================
// SCORE BOUNDARY TESTS
// =============================================================================

#[tokio::test]
async fn lead_score_clamped_at_maximum() {
    // Create a lead that would score very high
    let lead = CrmLead {
        id: "high_lead".to_string(),
        first_name: "High".to_string(),
        last_name: "Scorer".to_string(),
        email: "high.scorer@enterprise.com".to_string(),
        phone: None,
        company: "Fortune 500 Enterprise".to_string(),
        status: "New".to_string(),
        source: "Direct".to_string(),
        title: Some("Chief Executive Officer".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let score = calculate_lead_score(&lead);
    assert!(score <= 100, "Score should be clamped at maximum of 100");
    assert!(score > 90, "High-value lead should have high score");
}

#[tokio::test]
async fn lead_score_clamped_at_minimum() {
    // Create a lead that would score very low
    let lead = CrmLead {
        id: "low_lead".to_string(),
        first_name: "Low".to_string(),
        last_name: "Scorer".to_string(),
        email: "low.scorer@gmail.com".to_string(),
        phone: None,
        company: "Personal".to_string(),
        status: "New".to_string(),
        source: "Cold Call".to_string(),
        title: Some("Unemployed".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let score = calculate_lead_score(&lead);
    assert!(score >= 0, "Score should be clamped at minimum of 0");
    assert!(score < 20, "Low-value lead should have low score");
}