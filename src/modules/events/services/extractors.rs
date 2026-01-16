use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ExtractedData {
    pub deal_amount: Option<i32>,
    pub company_name: Option<String>,
    pub contact_email: Option<String>,
    pub stage: Option<String>,
    pub description: Option<String>,
}

/// Extract business data from text using pattern matching
pub fn extract_business_data(text: &str) -> ExtractedData {
    ExtractedData {
        deal_amount: extract_deal_amount(text),
        company_name: extract_company_name(text),
        contact_email: extract_email(text),
        stage: detect_deal_stage(text),
        description: Some(text.to_string()),
    }
}

/// Extract deal amounts like "$50k", "$50,000", "$50K"
fn extract_deal_amount(text: &str) -> Option<i32> {
    // Pattern 1: $50k or $50K
    let re_k = Regex::new(r"\$(\d+(?:,\d{3})*)[kK]").ok()?;
    if let Some(cap) = re_k.captures(text) {
        let num_str = cap.get(1)?.as_str().replace(",", "");
        return num_str.parse::<i32>().ok().map(|n| n * 1000);
    }
    
    // Pattern 2: $50,000 or $50000
    let re_full = Regex::new(r"\$(\d+(?:,\d{3})*)(?:[^\d]|$)").ok()?;
    if let Some(cap) = re_full.captures(text) {
        let num_str = cap.get(1)?.as_str().replace(",", "");
        return num_str.parse::<i32>().ok();
    }
    
    None
}

/// Extract company names - looks for patterns like "with Acme Corp"
fn extract_company_name(text: &str) -> Option<String> {
    // Pattern: "with [Company Name]" where name is capitalized
    let patterns = vec![
        r"\b(?:with|from|at)\s+([A-Z][a-zA-Z]+(?:\s+(?:Corp|Inc|LLC|Ltd|Co|Company|Corporation|Technologies|Tech|Solutions|Group|Partners|Ventures))?)",
        r"\b([A-Z][a-zA-Z]+(?:\s+(?:Corp|Inc|LLC|Ltd|Co|Company|Corporation|Technologies|Tech|Solutions|Group|Partners|Ventures)))",
    ];
    
    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(cap) = re.captures(text) {
                return Some(cap.get(1)?.as_str().to_string());
            }
        }
    }
    
    None
}

/// Extract email addresses
fn extract_email(text: &str) -> Option<String> {
    let re = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").ok()?;
    re.find(text).map(|m| m.as_str().to_string())
}

/// Detect deal stage from keywords
fn detect_deal_stage(text: &str) -> Option<String> {
    let text_lower = text.to_lowercase();
    
    if text_lower.contains("closed") || text_lower.contains("won") || text_lower.contains("signed") {
        Some("Closed Won".to_string())
    } else if text_lower.contains("lost") || text_lower.contains("rejected") {
        Some("Closed Lost".to_string())
    } else if text_lower.contains("negotiat") || text_lower.contains("proposal") {
        Some("Negotiation".to_string())
    } else if text_lower.contains("qualify") || text_lower.contains("discovery") {
        Some("Qualification".to_string())
    } else if text_lower.contains("demo") || text_lower.contains("presentation") {
        Some("Demo".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_deal_amount_with_k() {
        assert_eq!(extract_deal_amount("Just closed a $50k deal!"), Some(50000));
        assert_eq!(extract_deal_amount("Signed $100K contract"), Some(100000));
    }

    #[test]
    fn test_extract_deal_amount_full() {
        assert_eq!(extract_deal_amount("Deal worth $50,000"), Some(50000));
        assert_eq!(extract_deal_amount("Won $100000 deal"), Some(100000));
    }

    #[test]
    fn test_extract_company_name() {
        assert_eq!(
            extract_company_name("Closed deal with Acme Corp"),
            Some("Acme Corp".to_string())
        );
        assert_eq!(
            extract_company_name("Meeting at Tech Solutions"),
            Some("Tech Solutions".to_string())
        );
    }

    #[test]
    fn test_extract_email() {
        assert_eq!(
            extract_email("Contact john@acme.com for details"),
            Some("john@acme.com".to_string())
        );
    }

    #[test]
    fn test_detect_deal_stage() {
        assert_eq!(
            detect_deal_stage("Just closed the deal!"),
            Some("Closed Won".to_string())
        );
        assert_eq!(
            detect_deal_stage("In negotiation phase"),
            Some("Negotiation".to_string())
        );
    }

    #[test]
    fn test_full_extraction() {
        let text = "Just closed a $50k deal with Acme Corp! Contact john@acme.com";
        let data = extract_business_data(text);
        
        assert_eq!(data.deal_amount, Some(50000));
        assert_eq!(data.company_name, Some("Acme Corp".to_string()));
        assert_eq!(data.contact_email, Some("john@acme.com".to_string()));
        assert_eq!(data.stage, Some("Closed Won".to_string()));
    }
}
