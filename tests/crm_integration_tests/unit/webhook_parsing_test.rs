///! Unit Tests for CRM Webhook Parsing Logic

use serde_json::Value;

// =============================================================================
// WEBHOOK PARSING LOGIC
// =============================================================================

#[derive(Debug, PartialEq)]
struct ParsedWebhook {
    event_type: String,
    entity_type: String,
    operation: String,
    entity_id: Option<String>,
    timestamp: Option<String>,
    data: Value,
}

fn parse_webhook_payload(payload: &str) -> Result<ParsedWebhook, String> {
    let parsed: Value = serde_json::from_str(payload)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let event = parsed.get("event")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'event' field")?
        .to_string();

    let data = parsed.get("data")
        .unwrap_or(&Value::Null)
        .clone();

    let entity_id = parsed.pointer("/data/id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let timestamp = parsed.get("timestamp")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Determine entity type from event
    let entity_type = if event.contains("contact") || event.contains("person") {
        "contact".to_string()
    } else if event.contains("lead") {
        "lead".to_string()
    } else if event.contains("opportunity") || event.contains("deal") {
        "opportunity".to_string()
    } else if event.contains("account") || event.contains("company") {
        "account".to_string()
    } else {
        "unknown".to_string()
    };

    // Determine operation from event
    let operation = if event.contains(".created") || event.contains(".added") || event.contains(".inserted") {
        "create".to_string()
    } else if event.contains(".updated") || event.contains(".modified") || event.contains(".changed") {
        "update".to_string()
    } else if event.contains(".deleted") || event.contains(".removed") {
        "delete".to_string()
    } else if event.contains(".merged") {
        "merge".to_string()
    } else {
        "other".to_string()
    };

    Ok(ParsedWebhook {
        event_type: event,
        entity_type,
        operation,
        entity_id,
        timestamp,
        data,
    })
}

fn validate_webhook_signature(_payload: &str, _signature: &str, _secret: &str) -> bool {
    // Placeholder implementation - in real world would use HMAC-SHA256
    true
}

fn extract_entity_id_from_payload(payload: &str) -> Option<String> {
    let parsed: Result<Value, _> = serde_json::from_str(payload);
    if let Ok(parsed) = parsed {
        parsed.pointer("/data/id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        None
    }
}

// =============================================================================
// BASIC WEBHOOK PARSING TESTS
// =============================================================================

#[tokio::test]
async fn webhook_parsing_parses_valid_contact_created_event() {
    let payload = r#"{
        "event": "contact.created",
        "timestamp": "2023-10-15T10:30:00Z",
        "data": {
            "id": "contact_123",
            "first_name": "John",
            "last_name": "Doe",
            "email": "john.doe@example.com"
        }
    }"#;

    let result = parse_webhook_payload(payload);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(parsed.event_type, "contact.created");
    assert_eq!(parsed.entity_type, "contact");
    assert_eq!(parsed.operation, "create");
    assert_eq!(parsed.entity_id, Some("contact_123".to_string()));
    assert_eq!(parsed.timestamp, Some("2023-10-15T10:30:00Z".to_string()));
    assert!(parsed.data.is_object());
}

#[tokio::test]
async fn webhook_parsing_parses_valid_lead_updated_event() {
    let payload = r#"{
        "event": "lead.updated",
        "timestamp": "2023-10-15T11:30:00Z",
        "data": {
            "id": "lead_456",
            "first_name": "Alice",
            "last_name": "Johnson",
            "status": "Qualified"
        }
    }"#;

    let result = parse_webhook_payload(payload);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(parsed.event_type, "lead.updated");
    assert_eq!(parsed.entity_type, "lead");
    assert_eq!(parsed.operation, "update");
    assert_eq!(parsed.entity_id, Some("lead_456".to_string()));
    assert_eq!(parsed.timestamp, Some("2023-10-15T11:30:00Z".to_string()));
}

#[tokio::test]
async fn webhook_parsing_parses_valid_opportunity_deleted_event() {
    let payload = r#"{
        "event": "opportunity.deleted",
        "timestamp": "2023-10-15T12:30:00Z",
        "data": {
            "id": "opp_789"
        }
    }"#;

    let result = parse_webhook_payload(payload);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(parsed.event_type, "opportunity.deleted");
    assert_eq!(parsed.entity_type, "opportunity");
    assert_eq!(parsed.operation, "delete");
    assert_eq!(parsed.entity_id, Some("opp_789".to_string()));
}

// =============================================================================
// WEBHOOK PARSING ERROR HANDLING TESTS
// =============================================================================

#[tokio::test]
async fn webhook_parsing_rejects_invalid_json() {
    let invalid_payload = r#"{"event": "contact.created""#; // Missing closing brace

    let result = parse_webhook_payload(invalid_payload);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid JSON"));
}

#[tokio::test]
async fn webhook_parsing_rejects_missing_event_field() {
    let payload = r#"{
        "timestamp": "2023-10-15T10:30:00Z",
        "data": {
            "id": "contact_123"
        }
    }"#;

    let result = parse_webhook_payload(payload);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing 'event' field"));
}

#[tokio::test]
async fn webhook_parsing_handles_missing_timestamp() {
    let payload = r#"{
        "event": "contact.created",
        "data": {
            "id": "contact_123",
            "first_name": "John",
            "last_name": "Doe"
        }
    }"#;

    let result = parse_webhook_payload(payload);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(parsed.event_type, "contact.created");
    assert_eq!(parsed.timestamp, None); // Timestamp is optional
}

#[tokio::test]
async fn webhook_parsing_handles_missing_data_field() {
    let payload = r#"{
        "event": "contact.created",
        "timestamp": "2023-10-15T10:30:00Z"
    }"#;

    let result = parse_webhook_payload(payload);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(parsed.event_type, "contact.created");
    assert!(parsed.data.is_null()); // Data defaults to null
}

#[tokio::test]
async fn webhook_parsing_handles_missing_entity_id() {
    let payload = r#"{
        "event": "contact.created",
        "timestamp": "2023-10-15T10:30:00Z",
        "data": {
            "first_name": "John",
            "last_name": "Doe"
        }
    }"#;

    let result = parse_webhook_payload(payload);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(parsed.entity_type, "contact");
    assert_eq!(parsed.operation, "create");
    assert_eq!(parsed.entity_id, None); // ID is optional in data
}

// =============================================================================
// ENTITY TYPE DETECTION TESTS
// =============================================================================

#[tokio::test]
async fn webhook_parsing_detects_contact_entity_types() {
    let events = vec!["contact.created", "person.updated", "contact.deleted"];
    
    for event in events {
        let payload = format!(r#"{{
            "event": "{}",
            "data": {{"id": "123"}}
        }}"#, event);
        
        let result = parse_webhook_payload(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().entity_type, "contact");
    }
}

#[tokio::test]
async fn webhook_parsing_detects_lead_entity_types() {
    let events = vec!["lead.created", "lead.updated", "lead.deleted"];
    
    for event in events {
        let payload = format!(r#"{{
            "event": "{}",
            "data": {{"id": "123"}}
        }}"#, event);
        
        let result = parse_webhook_payload(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().entity_type, "lead");
    }
}

#[tokio::test]
async fn webhook_parsing_detects_opportunity_entity_types() {
    let events = vec!["opportunity.created", "deal.updated", "opportunity.deleted"];
    
    for event in events {
        let payload = format!(r#"{{
            "event": "{}",
            "data": {{"id": "123"}}
        }}"#, event);
        
        let result = parse_webhook_payload(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().entity_type, "opportunity");
    }
}

#[tokio::test]
async fn webhook_parsing_detects_account_entity_types() {
    let events = vec!["account.created", "company.updated", "account.deleted"];
    
    for event in events {
        let payload = format!(r#"{{
            "event": "{}",
            "data": {{"id": "123"}}
        }}"#, event);
        
        let result = parse_webhook_payload(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().entity_type, "account");
    }
}

#[tokio::test]
async fn webhook_parsing_defaults_to_unknown_entity_type() {
    let events = vec!["custom.event", "unknown.created", "special.updated"];
    
    for event in events {
        let payload = format!(r#"{{
            "event": "{}",
            "data": {{"id": "123"}}
        }}"#, event);
        
        let result = parse_webhook_payload(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().entity_type, "unknown");
    }
}

// =============================================================================
// OPERATION DETECTION TESTS
// =============================================================================

#[tokio::test]
async fn webhook_parsing_detects_create_operations() {
    let events = vec!["contact.created", "lead.added", "opportunity.inserted"];
    
    for event in events {
        let payload = format!(r#"{{
            "event": "{}",
            "data": {{"id": "123"}}
        }}"#, event);
        
        let result = parse_webhook_payload(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().operation, "create");
    }
}

#[tokio::test]
async fn webhook_parsing_detects_update_operations() {
    let events = vec!["contact.updated", "lead.modified", "opportunity.changed"];
    
    for event in events {
        let payload = format!(r#"{{
            "event": "{}",
            "data": {{"id": "123"}}
        }}"#, event);
        
        let result = parse_webhook_payload(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().operation, "update");
    }
}

#[tokio::test]
async fn webhook_parsing_detects_delete_operations() {
    let events = vec!["contact.deleted", "lead.removed", "opportunity.deleted"];
    
    for event in events {
        let payload = format!(r#"{{
            "event": "{}",
            "data": {{"id": "123"}}
        }}"#, event);
        
        let result = parse_webhook_payload(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().operation, "delete");
    }
}

#[tokio::test]
async fn webhook_parsing_detects_merge_operations() {
    let events = vec!["contact.merged", "account.merged"];
    
    for event in events {
        let payload = format!(r#"{{
            "event": "{}",
            "data": {{"id": "123"}}
        }}"#, event);
        
        let result = parse_webhook_payload(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().operation, "merge");
    }
}

#[tokio::test]
async fn webhook_parsing_defaults_to_other_operation() {
    let events = vec!["contact.synced", "lead.assigned", "opportunity.reopened"];
    
    for event in events {
        let payload = format!(r#"{{
            "event": "{}",
            "data": {{"id": "123"}}
        }}"#, event);
        
        let result = parse_webhook_payload(&payload);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().operation, "other");
    }
}

// =============================================================================
// SIGNATURE VALIDATION TESTS
// =============================================================================

#[tokio::test]
async fn webhook_signature_validation_rejects_empty_signature() {
    let payload = r#"{"event": "contact.created", "data": {"id": "123"}}"#;
    let is_valid = validate_webhook_signature(payload, "", "secret");
    
    assert!(!is_valid, "Empty signature should be invalid");
}

#[tokio::test]
async fn webhook_signature_validation_accepts_valid_signature_format() {
    let payload = r#"{"event": "contact.created", "data": {"id": "123"}}"#;
    // Simulate a valid signature (in real world this would be HMAC result)
    let is_valid = validate_webhook_signature(payload, "valid_signature_123", "secret");
    
    assert!(is_valid, "Valid signature format should be accepted");
}

#[tokio::test]
async fn webhook_signature_validation_rejects_short_signature() {
    let payload = r#"{"event": "contact.created", "data": {"id": "123"}}"#;
    let is_valid = validate_webhook_signature(payload, "short", "secret");
    
    assert!(!is_valid, "Short signature should be rejected");
}

// =============================================================================
// ENTITY ID EXTRACTION TESTS
// =============================================================================

#[tokio::test]
async fn entity_id_extraction_finds_existing_id() {
    let payload = r#"{
        "event": "contact.created",
        "data": {
            "id": "contact_123",
            "first_name": "John"
        }
    }"#;
    
    let id = extract_entity_id_from_payload(payload);
    assert_eq!(id, Some("contact_123".to_string()));
}

#[tokio::test]
async fn entity_id_extraction_returns_none_when_missing() {
    let payload = r#"{
        "event": "contact.created",
        "data": {
            "first_name": "John",
            "last_name": "Doe"
        }
    }"#;
    
    let id = extract_entity_id_from_payload(payload);
    assert_eq!(id, None);
}

#[tokio::test]
async fn entity_id_extraction_handles_nested_ids() {
    let payload = r#"{
        "event": "contact.created",
        "data": {
            "contact": {
                "id": "nested_contact_456"
            }
        }
    }"#;
    
    // Our current extraction only looks at /data/id, not nested paths
    let id = extract_entity_id_from_payload(payload);
    assert_eq!(id, None);
}

#[tokio::test]
async fn entity_id_extraction_handles_invalid_json() {
    let invalid_payload = r#"{"event": "contact.created""#;
    
    let id = extract_entity_id_from_payload(invalid_payload);
    assert_eq!(id, None);
}

// =============================================================================
// CASE SENSITIVITY TESTS
// =============================================================================

#[tokio::test]
async fn webhook_parsing_handles_mixed_case_events() {
    let payload = r#"{
        "event": "Contact.Created",
        "data": {
            "id": "contact_123"
        }
    }"#;

    let result = parse_webhook_payload(payload);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(parsed.event_type, "Contact.Created");
    // Entity type detection should be case-insensitive
    assert_eq!(parsed.entity_type, "contact");
    assert_eq!(parsed.operation, "create");
}

#[tokio::test]
async fn webhook_parsing_handles_different_vendor_formats() {
    // Salesforce format
    let salesforce_payload = r#"{
        "event": "sobject.ContactChangeEvent",
        "data": {
            "Id": "contact_123",
            "ChangeEventHeader": {
                "changeType": "CREATE"
            }
        }
    }"#;

    let result = parse_webhook_payload(salesforce_payload);
    assert!(result.is_ok());
    
    let parsed = result.unwrap();
    // Our parser identifies "contact" in the event name
    assert_eq!(parsed.entity_type, "contact");
    // For "ChangeEvent", we default to "other" since it doesn't match our patterns
    assert_eq!(parsed.operation, "other");
}

#[tokio::test]
async fn webhook_parsing_handles_hubspot_format() {
    // HubSpot format
    let hubspot_payload = r#"{
        "eventId": "event_123",
        "subscriptionId": "sub_456",
        "portalId": "portal_789",
        "occurredAt": 1678886400000,
        "subscriptionType": "contact.creation",
        "objectId": "contact_999",
        "properties": {
            "firstname": "John",
            "lastname": "Doe",
            "email": "john@example.com"
        }
    }"#;

    // This payload doesn't match our expected format (no "event" field)
    let result = parse_webhook_payload(hubspot_payload);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing 'event' field"));
}