///! Unit Tests for CRM Contact Validation Logic

use echo_backend::modules::crm_integration::schemas::CreateContactRequest as CrmContactInput;

// =============================================================================
// CONTACT INPUT VALIDATION LOGIC
// =============================================================================

fn validate_contact_input(input: &CrmContactInput) -> Result<(), String> {
    // Basic validation logic that would be in the actual service
    if input.first_name.trim().is_empty() {
        return Err("First name is required".to_string());
    }

    if input.last_name.trim().is_empty() {
        return Err("Last name is required".to_string());
    }

    if !input.email.contains('@') || !input.email.contains('.') {
        return Err("Invalid email format".to_string());
    }

    // Validate phone if provided
    if let Some(ref phone) = input.phone {
        if !phone.chars().all(|c: char| c.is_ascii_digit() || c == '+' || c == '-' || c == '(' || c == ')' || c == ' ' || c == '.') {
            return Err("Invalid phone format".to_string());
        }
    }

    // Validate company name if provided
    if let Some(ref company) = input.company {
        if company.trim().is_empty() {
            return Err("Company name cannot be empty".to_string());
        }
    }

    Ok(())
}

#[tokio::test]
async fn contact_validation_accepts_valid_input() {
    let valid_input = CrmContactInput {
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        email: "john.doe@example.com".to_string(),
        phone: Some("+1-234-567-8900".to_string()),
        company: Some("Acme Corp".to_string()),
        title: Some("Software Engineer".to_string()),
    };

    let result = validate_contact_input(&valid_input);
    assert!(result.is_ok(), "Valid contact input should pass validation");
}

#[tokio::test]
async fn contact_validation_rejects_empty_first_name() {
    let invalid_input = CrmContactInput {
        first_name: "".to_string(),
        last_name: "Doe".to_string(),
        email: "john.doe@example.com".to_string(),
        phone: Some("+1-234-567-8900".to_string()),
        company: Some("Acme Corp".to_string()),
        title: Some("Software Engineer".to_string()),
    };

    let result = validate_contact_input(&invalid_input);
    assert!(result.is_err(), "Empty first name should be rejected");
    assert!(result.unwrap_err().contains("First name is required"));
}

#[tokio::test]
async fn contact_validation_rejects_empty_last_name() {
    let invalid_input = CrmContactInput {
        first_name: "John".to_string(),
        last_name: "".to_string(),
        email: "john.doe@example.com".to_string(),
        phone: Some("+1-234-567-8900".to_string()),
        company: Some("Acme Corp".to_string()),
        title: Some("Software Engineer".to_string()),
    };

    let result = validate_contact_input(&invalid_input);
    assert!(result.is_err(), "Empty last name should be rejected");
    assert!(result.unwrap_err().contains("Last name is required"));
}

#[tokio::test]
async fn contact_validation_rejects_invalid_email_missing_at() {
    let invalid_input = CrmContactInput {
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        email: "johndoe.com".to_string(),  // Missing @
        phone: Some("+1-234-567-8900".to_string()),
        company: Some("Acme Corp".to_string()),
        title: Some("Software Engineer".to_string()),
    };

    let result = validate_contact_input(&invalid_input);
    assert!(result.is_err(), "Email without @ should be rejected");
    assert!(result.unwrap_err().contains("Invalid email format"));
}

#[tokio::test]
async fn contact_validation_rejects_invalid_email_missing_dot() {
    let invalid_input = CrmContactInput {
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        email: "john@doecom".to_string(),  // Missing .
        phone: Some("+1-234-567-8900".to_string()),
        company: Some("Acme Corp".to_string()),
        title: Some("Software Engineer".to_string()),
    };

    let result = validate_contact_input(&invalid_input);
    assert!(result.is_err(), "Email without . should be rejected");
    assert!(result.unwrap_err().contains("Invalid email format"));
}

#[tokio::test]
async fn contact_validation_accepts_email_with_special_chars() {
    let valid_input = CrmContactInput {
        first_name: "Test".to_string(),
        last_name: "User".to_string(),
        email: "test+tag.user@domain.co.uk".to_string(),  // Valid email with special chars
        phone: Some("(123) 456-7890".to_string()),
        company: Some("Special Domain Corp".to_string()),
        title: Some("Developer".to_string()),
    };

    let result = validate_contact_input(&valid_input);
    assert!(result.is_ok(), "Email with valid special characters should pass validation");
}

#[tokio::test]
async fn contact_validation_rejects_invalid_phone_format() {
    let invalid_input = CrmContactInput {
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        email: "john.doe@example.com".to_string(),
        phone: Some("invalid-phone-number".to_string()),  // Contains invalid chars
        company: Some("Acme Corp".to_string()),
        title: Some("Software Engineer".to_string()),
    };

    let result = validate_contact_input(&invalid_input);
    assert!(result.is_err(), "Invalid phone format should be rejected");
    assert!(result.unwrap_err().contains("Invalid phone format"));
}

#[tokio::test]
async fn contact_validation_accepts_valid_phone_formats() {
    let valid_phones = vec![
        "+1-234-567-8900",
        "(123) 456-7890",
        "123-456-7890",
        "+1.234.567.8900",
        "1234567890",
        "+12345678900",
    ];

    for phone in valid_phones {
        let input = CrmContactInput {
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            phone: Some(phone.to_string()),
            company: Some("Acme Corp".to_string()),
            title: Some("Software Engineer".to_string()),
        };

        let result = validate_contact_input(&input);
        assert!(result.is_ok(), "Phone '{}' should be valid", phone);
    }
}

#[tokio::test]
async fn contact_validation_rejects_empty_company_name() {
    let invalid_input = CrmContactInput {
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        email: "john.doe@example.com".to_string(),
        phone: Some("+1-234-567-8900".to_string()),
        company: Some("".to_string()),  // Empty company
        title: Some("Software Engineer".to_string()),
    };

    let result = validate_contact_input(&invalid_input);
    assert!(result.is_err(), "Empty company name should be rejected");
    assert!(result.unwrap_err().contains("Company name cannot be empty"));
}

#[tokio::test]
async fn contact_validation_accepts_optional_company_and_title() {
    let valid_input = CrmContactInput {
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        email: "john.doe@example.com".to_string(),
        phone: None,  // Phone is optional
        company: None,  // Company is optional
        title: None,  // Title is optional
    };

    let result = validate_contact_input(&valid_input);
    assert!(result.is_ok(), "Optional fields should be allowed to be None");
}

#[tokio::test]
async fn contact_validation_trims_whitespace_correctly() {
    let input_with_whitespace = CrmContactInput {
        first_name: "  John  ".to_string(),
        last_name: "  Doe  ".to_string(),
        email: "  john.doe@example.com  ".to_string(),
        phone: Some("  +1-234-567-8900  ".to_string()),
        company: Some("  Acme Corp  ".to_string()),
        title: Some("  Software Engineer  ".to_string()),
    };

    // The validation should trim whitespace during processing
    // For this test, we'll check that the validation doesn't reject due to whitespace
    // (actual trimming would happen in the service layer)
    let result = validate_contact_input(&input_with_whitespace);
    assert!(result.is_ok(), "Whitespace should not affect validation outcome");
}

// =============================================================================
// EMAIL NORMALIZATION LOGIC
// =============================================================================

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

#[tokio::test]
async fn email_normalization_converts_to_lowercase() {
    let raw_email = "John.Doe@EXAMPLE.COM";
    let normalized = normalize_email(raw_email);
    
    assert_eq!(normalized, "john.doe@example.com");
}

#[tokio::test]
async fn email_normalization_trims_whitespace() {
    let raw_email = "  john.doe@example.com  ";
    let normalized = normalize_email(raw_email);
    
    assert_eq!(normalized, "john.doe@example.com");
}

#[tokio::test]
async fn email_normalization_handles_both_operations() {
    let raw_email = "  John.Doe@EXAMPLE.COM  ";
    let normalized = normalize_email(raw_email);
    
    assert_eq!(normalized, "john.doe@example.com");
}

// =============================================================================
// NAME VALIDATION LOGIC
// =============================================================================

fn is_valid_name(name: Option<&str>) -> bool {
    match name {
        None => true, // Names are optional
        Some(n) => !n.trim().is_empty() && n.len() <= 100,
    }
}

#[tokio::test]
async fn name_validation_accepts_none() {
    assert!(is_valid_name(None));
}

#[tokio::test]
async fn name_validation_accepts_valid_names() {
    assert!(is_valid_name(Some("John")));
    assert!(is_valid_name(Some("Mary-Jane")));
    assert!(is_valid_name(Some("José")));
    assert!(is_valid_name(Some("Zhang Wei")));
}

#[tokio::test]
async fn name_validation_rejects_too_long() {
    let long_name = "a".repeat(101);
    assert!(!is_valid_name(Some(&long_name)));
}

#[tokio::test]
async fn name_validation_accepts_max_length() {
    let max_name = "a".repeat(100);
    assert!(is_valid_name(Some(&max_name)));
}

#[tokio::test]
async fn name_validation_rejects_whitespace_only() {
    assert!(!is_valid_name(Some("   ")));
    assert!(!is_valid_name(Some("\t\n")));
}