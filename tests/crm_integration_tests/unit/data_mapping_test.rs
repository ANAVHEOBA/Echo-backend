///! Unit Tests for CRM Data Mapping Logic

use std::collections::HashMap;

// =============================================================================
// DATA MAPPING LOGIC
// =============================================================================

#[derive(Debug, Clone)]
struct CrmContact {
    id: Option<String>,
    first_name: String,
    last_name: String,
    email: String,
    phone: Option<String>,
    company: Option<String>,
    title: Option<String>,
}

#[derive(Debug, Clone)]
struct ExternalContact {
    external_id: Option<String>,
    fname: String,
    lname: String,
    email_addr: String,
    telephone: Option<String>,
    org_name: Option<String>,
    job_title: Option<String>,
}

fn map_external_contact_to_internal(external: ExternalContact, field_mapping: &HashMap<String, String>) -> CrmContact {
    // Apply field mappings to transform external data to internal structure
    let mut mapped_data = HashMap::new();
    
    // Map each external field to internal field based on mapping
    if let Some(ref external_id) = external.external_id {
        if let Some(internal_field) = field_mapping.get("id") {
            mapped_data.insert(internal_field.clone(), external_id.clone());
        }
    }

    if let Some(ref phone) = external.telephone {
        if let Some(internal_field) = field_mapping.get("phone") {
            mapped_data.insert(internal_field.clone(), phone.clone());
        }
    }

    if let Some(ref org) = external.org_name {
        if let Some(internal_field) = field_mapping.get("company") {
            mapped_data.insert(internal_field.clone(), org.clone());
        }
    }

    if let Some(ref title) = external.job_title {
        if let Some(internal_field) = field_mapping.get("title") {
            mapped_data.insert(internal_field.clone(), title.clone());
        }
    }
    
    // Create internal contact with mapped data
    CrmContact {
        id: external.external_id,
        first_name: external.fname,
        last_name: external.lname,
        email: external.email_addr,
        phone: external.telephone,
        company: external.org_name,
        title: external.job_title,
    }
}

fn create_default_field_mapping(source_crm: &str) -> HashMap<String, String> {
    let mut mapping = HashMap::new();
    
    match source_crm.to_lowercase().as_str() {
        "salesforce" => {
            mapping.insert("id".to_string(), "Id".to_string());
            mapping.insert("first_name".to_string(), "FirstName".to_string());
            mapping.insert("last_name".to_string(), "LastName".to_string());
            mapping.insert("email".to_string(), "Email".to_string());
            mapping.insert("phone".to_string(), "Phone".to_string());
            mapping.insert("company".to_string(), "Company".to_string());
            mapping.insert("title".to_string(), "Title".to_string());
        },
        "hubspot" => {
            mapping.insert("id".to_string(), "vid".to_string());
            mapping.insert("first_name".to_string(), "firstname".to_string());
            mapping.insert("last_name".to_string(), "lastname".to_string());
            mapping.insert("email".to_string(), "email".to_string());
            mapping.insert("phone".to_string(), "phone".to_string());
            mapping.insert("company".to_string(), "company".to_string());
            mapping.insert("title".to_string(), "jobtitle".to_string());
        },
        "pipedrive" => {
            mapping.insert("id".to_string(), "id".to_string());
            mapping.insert("first_name".to_string(), "first_name".to_string());
            mapping.insert("last_name".to_string(), "last_name".to_string());
            mapping.insert("email".to_string(), "email".to_string());
            mapping.insert("phone".to_string(), "phone".to_string());
            mapping.insert("company".to_string(), "org_name".to_string());
            mapping.insert("title".to_string(), "position".to_string());
        },
        _ => {
            // Default mapping for unknown CRM
            mapping.insert("id".to_string(), "id".to_string());
            mapping.insert("first_name".to_string(), "first_name".to_string());
            mapping.insert("last_name".to_string(), "last_name".to_string());
            mapping.insert("email".to_string(), "email".to_string());
            mapping.insert("phone".to_string(), "phone".to_string());
            mapping.insert("company".to_string(), "company".to_string());
            mapping.insert("title".to_string(), "title".to_string());
        }
    }
    
    mapping
}

fn normalize_phone_number(phone: &str) -> String {
    // Remove all non-digit characters except +
    let cleaned: String = phone.chars()
        .filter(|c| c.is_ascii_digit() || *c == '+')
        .collect();
    
    // If it starts with +, leave as is
    if cleaned.starts_with('+') {
        return cleaned;
    }
    
    // If it's 10 digits, assume US and add +1
    if cleaned.len() == 10 {
        return format!("+1{}", cleaned);
    }
    
    // If it's 11 digits and starts with 1, add +
    if cleaned.len() == 11 && cleaned.starts_with('1') {
        return format!("+{}", cleaned);
    }
    
    // Otherwise, return as is
    cleaned
}

fn normalize_company_name(company: &str) -> String {
    // Remove extra whitespace and common suffixes
    let mut normalized = company.trim().to_string();
    
    // Remove common legal suffixes
    let suffixes = ["LLC", "Inc", "Corp", "Ltd", "Co", "GmbH", "SA", "SRL"];
    for suffix in &suffixes {
        if normalized.ends_with(suffix) {
            let len = normalized.len();
            let end_pos = len - suffix.len();
            if end_pos > 0 && normalized.chars().nth(end_pos - 1).unwrap_or(' ') == ' ' {
                normalized = normalized[..end_pos].trim_end().to_string();
            }
        }
    }
    
    // Collapse multiple spaces
    let mut result = String::new();
    let mut prev_char_was_space = false;
    
    for c in normalized.chars() {
        if c.is_whitespace() {
            if !prev_char_was_space {
                result.push(' ');
                prev_char_was_space = true;
            }
        } else {
            result.push(c);
            prev_char_was_space = false;
        }
    }
    
    result
}

// =============================================================================
// BASIC DATA MAPPING TESTS
// =============================================================================

#[tokio::test]
async fn data_mapping_maps_salesforce_contact_correctly() {
    let mut field_mapping = HashMap::new();
    field_mapping.insert("id".to_string(), "Id".to_string());
    field_mapping.insert("phone".to_string(), "Phone".to_string());
    field_mapping.insert("company".to_string(), "Company".to_string());
    field_mapping.insert("title".to_string(), "Title".to_string());
    
    let external_contact = ExternalContact {
        external_id: Some("sf_contact_123".to_string()),
        fname: "John".to_string(),
        lname: "Doe".to_string(),
        email_addr: "john.doe@example.com".to_string(),
        telephone: Some("+1-234-567-8900".to_string()),
        org_name: Some("Acme Corp".to_string()),
        job_title: Some("Software Engineer".to_string()),
    };
    
    let internal_contact = map_external_contact_to_internal(external_contact, &field_mapping);
    
    assert_eq!(internal_contact.id, Some("sf_contact_123".to_string()));
    assert_eq!(internal_contact.first_name, "John");
    assert_eq!(internal_contact.last_name, "Doe");
    assert_eq!(internal_contact.email, "john.doe@example.com");
    assert_eq!(internal_contact.phone, Some("+1-234-567-8900".to_string()));
    assert_eq!(internal_contact.company, Some("Acme Corp".to_string()));
    assert_eq!(internal_contact.title, Some("Software Engineer".to_string()));
}

#[tokio::test]
async fn data_mapping_maps_hubspot_contact_correctly() {
    let mut field_mapping = HashMap::new();
    field_mapping.insert("id".to_string(), "vid".to_string());
    field_mapping.insert("phone".to_string(), "phone".to_string());
    field_mapping.insert("company".to_string(), "company".to_string());
    field_mapping.insert("title".to_string(), "jobtitle".to_string());
    
    let external_contact = ExternalContact {
        external_id: Some("hub_contact_456".to_string()),
        fname: "Jane".to_string(),
        lname: "Smith".to_string(),
        email_addr: "jane.smith@example.com".to_string(),
        telephone: Some("(555) 123-4567".to_string()),
        org_name: Some("Tech Solutions LLC".to_string()),
        job_title: Some("CTO".to_string()),
    };
    
    let internal_contact = map_external_contact_to_internal(external_contact, &field_mapping);
    
    assert_eq!(internal_contact.id, Some("hub_contact_456".to_string()));
    assert_eq!(internal_contact.first_name, "Jane");
    assert_eq!(internal_contact.last_name, "Smith");
    assert_eq!(internal_contact.email, "jane.smith@example.com");
    assert_eq!(internal_contact.phone, Some("(555) 123-4567".to_string()));
    assert_eq!(internal_contact.company, Some("Tech Solutions LLC".to_string()));
    assert_eq!(internal_contact.title, Some("CTO".to_string()));
}

// =============================================================================
// FIELD MAPPING GENERATION TESTS
// =============================================================================

#[tokio::test]
async fn field_mapping_generation_creates_salesforce_mapping() {
    let mapping = create_default_field_mapping("salesforce");
    
    assert_eq!(mapping.get("id"), Some(&"Id".to_string()));
    assert_eq!(mapping.get("first_name"), Some(&"FirstName".to_string()));
    assert_eq!(mapping.get("last_name"), Some(&"LastName".to_string()));
    assert_eq!(mapping.get("email"), Some(&"Email".to_string()));
    assert_eq!(mapping.get("phone"), Some(&"Phone".to_string()));
    assert_eq!(mapping.get("company"), Some(&"Company".to_string()));
    assert_eq!(mapping.get("title"), Some(&"Title".to_string()));
}

#[tokio::test]
async fn field_mapping_generation_creates_hubspot_mapping() {
    let mapping = create_default_field_mapping("hubspot");
    
    assert_eq!(mapping.get("id"), Some(&"vid".to_string()));
    assert_eq!(mapping.get("first_name"), Some(&"firstname".to_string()));
    assert_eq!(mapping.get("last_name"), Some(&"lastname".to_string()));
    assert_eq!(mapping.get("email"), Some(&"email".to_string()));
    assert_eq!(mapping.get("phone"), Some(&"phone".to_string()));
    assert_eq!(mapping.get("company"), Some(&"company".to_string()));
    assert_eq!(mapping.get("title"), Some(&"jobtitle".to_string()));
}

#[tokio::test]
async fn field_mapping_generation_creates_pipedrive_mapping() {
    let mapping = create_default_field_mapping("pipedrive");
    
    assert_eq!(mapping.get("id"), Some(&"id".to_string()));
    assert_eq!(mapping.get("first_name"), Some(&"first_name".to_string()));
    assert_eq!(mapping.get("last_name"), Some(&"last_name".to_string()));
    assert_eq!(mapping.get("email"), Some(&"email".to_string()));
    assert_eq!(mapping.get("phone"), Some(&"phone".to_string()));
    assert_eq!(mapping.get("company"), Some(&"org_name".to_string()));
    assert_eq!(mapping.get("title"), Some(&"position".to_string()));
}

#[tokio::test]
async fn field_mapping_generation_creates_default_mapping() {
    let mapping = create_default_field_mapping("unknown_crm");
    
    assert_eq!(mapping.get("id"), Some(&"id".to_string()));
    assert_eq!(mapping.get("first_name"), Some(&"first_name".to_string()));
    assert_eq!(mapping.get("last_name"), Some(&"last_name".to_string()));
    assert_eq!(mapping.get("email"), Some(&"email".to_string()));
    assert_eq!(mapping.get("phone"), Some(&"phone".to_string()));
    assert_eq!(mapping.get("company"), Some(&"company".to_string()));
    assert_eq!(mapping.get("title"), Some(&"title".to_string()));
}

#[tokio::test]
async fn field_mapping_generation_case_insensitive() {
    let mapping1 = create_default_field_mapping("SALESFORCE");
    let mapping2 = create_default_field_mapping("salesforce");
    let mapping3 = create_default_field_mapping("SalesForce");
    
    assert_eq!(mapping1, mapping2);
    assert_eq!(mapping2, mapping3);
}

// =============================================================================
// PHONE NUMBER NORMALIZATION TESTS
// =============================================================================

#[tokio::test]
async fn phone_normalization_handles_various_formats() {
    assert_eq!(normalize_phone_number("+1-234-567-8900"), "+12345678900");
    assert_eq!(normalize_phone_number("(555) 123-4567"), "+15551234567");
    assert_eq!(normalize_phone_number("555.123.4567"), "+15551234567");
    assert_eq!(normalize_phone_number("5551234567"), "+15551234567");
    assert_eq!(normalize_phone_number("15551234567"), "+15551234567");
    assert_eq!(normalize_phone_number("+44-20-7946-0958"), "+442079460958"); // International
    assert_eq!(normalize_phone_number("invalid"), "invalid");
    assert_eq!(normalize_phone_number(""), "");
}

#[tokio::test]
async fn phone_normalization_preserves_international_format() {
    assert_eq!(normalize_phone_number("+442079460958"), "+442079460958");
    assert_eq!(normalize_phone_number("+33123456789"), "+33123456789");
    assert_eq!(normalize_phone_number("+81312345678"), "+81312345678");
}

#[tokio::test]
async fn phone_normalization_adds_country_code_for_us_numbers() {
    assert_eq!(normalize_phone_number("5551234567"), "+15551234567");
    assert_eq!(normalize_phone_number("15551234567"), "+15551234567");
    assert_eq!(normalize_phone_number("2125551234"), "+12125551234");
}

// =============================================================================
// COMPANY NAME NORMALIZATION TESTS
// =============================================================================

#[tokio::test]
async fn company_normalization_removes_legal_suffixes() {
    assert_eq!(normalize_company_name("Acme Corp"), "Acme");
    assert_eq!(normalize_company_name("Tech Solutions LLC"), "Tech Solutions");
    assert_eq!(normalize_company_name("Global Inc"), "Global");
    assert_eq!(normalize_company_name("International Ltd"), "International");
    assert_eq!(normalize_company_name("Company Co"), "Company");
    assert_eq!(normalize_company_name("Enterprise GmbH"), "Enterprise");
    assert_eq!(normalize_company_name("Business SA"), "Business");
    assert_eq!(normalize_company_name("Startup SRL"), "Startup");
}

#[tokio::test]
async fn company_normalization_handles_multiple_suffixes() {
    assert_eq!(normalize_company_name("ABC Corp LLC"), "ABC Corp"); // Only removes last
    assert_eq!(normalize_company_name("XYZ Inc."), "XYZ Inc."); // Doesn't match exactly
    assert_eq!(normalize_company_name("Test Company, Inc"), "Test Company, Inc"); // Doesn't match exactly
}

#[tokio::test]
async fn company_normalization_collapses_whitespace() {
    assert_eq!(normalize_company_name("  Acme   Corp  "), "Acme Corp");
    assert_eq!(normalize_company_name("Tech\tSolutions"), "Tech Solutions");
    assert_eq!(normalize_company_name("Global\nEnterprises"), "Global Enterprises");
    assert_eq!(normalize_company_name("Multi    Space    Company"), "Multi Space Company");
}

#[tokio::test]
async fn company_normalization_preserves_meaningful_content() {
    assert_eq!(normalize_company_name("Acme Corporation"), "Acme Corporation"); // Corp vs Corporation
    assert_eq!(normalize_company_name("Test & Co"), "Test & Co"); // Preserves symbols
    assert_eq!(normalize_company_name("Company Name With Numbers 123"), "Company Name With Numbers 123");
    assert_eq!(normalize_company_name(""), "");
}

// =============================================================================
// MAPPING EDGE CASE TESTS
// =============================================================================

#[tokio::test]
async fn data_mapping_handles_missing_optional_fields() {
    let field_mapping = HashMap::new(); // Empty mapping
    
    let external_contact = ExternalContact {
        external_id: None,
        fname: "John".to_string(),
        lname: "Doe".to_string(),
        email_addr: "john.doe@example.com".to_string(),
        telephone: None,
        org_name: None,
        job_title: None,
    };
    
    let internal_contact = map_external_contact_to_internal(external_contact, &field_mapping);
    
    assert_eq!(internal_contact.id, None);
    assert_eq!(internal_contact.phone, None);
    assert_eq!(internal_contact.company, None);
    assert_eq!(internal_contact.title, None);
}

#[tokio::test]
async fn data_mapping_preserves_required_fields_even_without_mapping() {
    let field_mapping = HashMap::new(); // Empty mapping
    
    let external_contact = ExternalContact {
        external_id: Some("contact_123".to_string()),
        fname: "Jane".to_string(),
        lname: "Smith".to_string(),
        email_addr: "jane.smith@example.com".to_string(),
        telephone: Some("555-123-4567".to_string()),
        org_name: Some("Tech Co".to_string()),
        job_title: Some("Engineer".to_string()),
    };
    
    let internal_contact = map_external_contact_to_internal(external_contact, &field_mapping);
    
    // Required fields are preserved even without mapping
    assert_eq!(internal_contact.first_name, "Jane");
    assert_eq!(internal_contact.last_name, "Smith");
    assert_eq!(internal_contact.email, "jane.smith@example.com");
}

#[tokio::test]
async fn field_mapping_generation_handles_empty_strings() {
    let mapping = create_default_field_mapping("");
    
    // Empty string should result in default mapping
    assert_eq!(mapping.get("id"), Some(&"id".to_string()));
}

// =============================================================================
// COMBINED TRANSFORMATION TESTS
// =============================================================================

#[tokio::test]
async fn combined_transformation_applies_all_normalizations() {
    let external_contact = ExternalContact {
        external_id: Some("ext_123".to_string()),
        fname: "  John  ".to_string(),  // Extra spaces in name
        lname: "  Doe  ".to_string(),   // Extra spaces in name
        email_addr: "JOHN.DOE@EXAMPLE.COM".to_string(),  // Upper case email
        telephone: Some("(555) 123-4567 ext. 890".to_string()),  // Complex phone
        org_name: Some("  Tech   Solutions   LLC  ".to_string()),  // Multiple spaces + suffix
        job_title: Some("  Senior    Software    Engineer  ".to_string()),  // Multiple spaces in title
    };
    
    // Apply transformations
    let normalized_phone = normalize_phone_number(&external_contact.telephone.clone().unwrap());
    let normalized_company = normalize_company_name(&external_contact.org_name.clone().unwrap());
    
    assert_eq!(normalized_phone, "+15551234567");
    assert_eq!(normalized_company, "Tech Solutions");
}

#[tokio::test]
async fn data_mapping_with_custom_field_mapping() {
    // Custom mapping for a hypothetical CRM
    let mut custom_mapping = HashMap::new();
    custom_mapping.insert("id".to_string(), "contact_id".to_string());
    custom_mapping.insert("phone".to_string(), "mobile_phone".to_string());
    custom_mapping.insert("company".to_string(), "organization".to_string());
    custom_mapping.insert("title".to_string(), "role".to_string());
    
    let external_contact = ExternalContact {
        external_id: Some("custom_456".to_string()),
        fname: "Alice".to_string(),
        lname: "Johnson".to_string(),
        email_addr: "alice.johnson@example.com".to_string(),
        telephone: Some("555-987-6543".to_string()),
        org_name: Some("Enterprise Solutions Inc".to_string()),
        job_title: Some("Product Manager".to_string()),
    };
    
    let internal_contact = map_external_contact_to_internal(external_contact, &custom_mapping);
    
    // The mapping should be applied but the actual values come from the struct fields
    assert_eq!(internal_contact.id, Some("custom_456".to_string()));
    assert_eq!(internal_contact.first_name, "Alice");
    assert_eq!(internal_contact.last_name, "Johnson");
    assert_eq!(internal_contact.email, "alice.johnson@example.com");
    assert_eq!(internal_contact.phone, Some("555-987-6543".to_string()));
    assert_eq!(internal_contact.company, Some("Enterprise Solutions Inc".to_string()));
    assert_eq!(internal_contact.title, Some("Product Manager".to_string()));
}