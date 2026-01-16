use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError, ValidationErrors};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateContactRequest {

    pub first_name: String,


    pub last_name: String,


    pub email: String,

    pub phone: Option<String>,

    pub company: Option<String>,
    pub title: Option<String>,
}

impl Validate for CreateContactRequest {
    fn validate(&self) -> Result<(), ValidationErrors> {
        use validator::ValidationErrors;

        let mut errors = ValidationErrors::new();

        // Validate first_name
        if self.first_name.is_empty() {
            let mut err = ValidationError::new("length");
            err.add_param("min".into(), &1usize);
            err.add_param("value".into(), &self.first_name);
            err.message = Some("First name is required".into());
            errors.add("first_name", err);
        }

        // Validate last_name
        if self.last_name.is_empty() {
            let mut err = ValidationError::new("length");
            err.add_param("min".into(), &1usize);
            err.add_param("value".into(), &self.last_name);
            err.message = Some("Last name is required".into());
            errors.add("last_name", err);
        }

        // Validate email
        if !self.email.contains('@') || !self.email.contains('.') {
            let mut err = ValidationError::new("email");
            err.message = Some("Invalid email format".into());
            errors.add("email", err);
        }

        // Validate phone if provided
        if let Some(ref phone) = self.phone {
            if !phone.chars().all(|c| c.is_ascii_digit() || "+-() ".contains(c)) || phone.len() < 7 {
                let mut err = ValidationError::new("custom");
                err.message = Some("Invalid phone format".into());
                errors.add("phone", err);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateContactRequest {

    pub first_name: Option<String>,


    pub last_name: Option<String>,


    pub email: Option<String>,

    pub phone: Option<String>,

    pub company: Option<String>,
    pub title: Option<String>,
}

impl Validate for UpdateContactRequest {
    fn validate(&self) -> Result<(), ValidationErrors> {
        use validator::ValidationErrors;

        let mut errors = ValidationErrors::new();

        // Validate first_name if provided
        if let Some(ref name) = self.first_name {
            if name.is_empty() {
                let mut err = ValidationError::new("length");
                err.add_param("min".into(), &1usize);
                err.add_param("value".into(), name);
                errors.add("first_name", err);
            }
        }

        // Validate last_name if provided
        if let Some(ref name) = self.last_name {
            if name.is_empty() {
                let mut err = ValidationError::new("length");
                err.add_param("min".into(), &1usize);
                err.add_param("value".into(), name);
                errors.add("last_name", err);
            }
        }

        // Validate email if provided
        if let Some(ref email) = self.email {
            if !email.contains('@') || !email.contains('.') {
                let err = ValidationError::new("email");
                errors.add("email", err);
            }
        }

        // Validate phone if provided
        if let Some(ref phone) = self.phone {
            if !phone.chars().all(|c| c.is_ascii_digit() || "+-() ".contains(c)) || phone.len() < 7 {
                let mut err = ValidationError::new("custom");
                err.message = Some("Invalid phone format".into());
                errors.add("phone", err);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetContactResponse {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub title: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateLeadRequest {

    pub first_name: String,


    pub last_name: String,


    pub email: String,

    pub phone: Option<String>,


    pub company: String,


    pub status: String,


    pub source: String,

    pub title: Option<String>,
}

impl Validate for CreateLeadRequest {
    fn validate(&self) -> Result<(), ValidationErrors> {
        use validator::ValidationErrors;

        let mut errors = ValidationErrors::new();

        // Validate first_name
        if self.first_name.is_empty() {
            let mut err = ValidationError::new("length");
            err.add_param("min".into(), &1usize);
            err.add_param("value".into(), &self.first_name);
            err.message = Some("First name is required".into());
            errors.add("first_name", err);
        }

        // Validate last_name
        if self.last_name.is_empty() {
            let mut err = ValidationError::new("length");
            err.add_param("min".into(), &1usize);
            err.add_param("value".into(), &self.last_name);
            err.message = Some("Last name is required".into());
            errors.add("last_name", err);
        }

        // Validate email
        if !self.email.contains('@') || !self.email.contains('.') {
            let mut err = ValidationError::new("email");
            err.message = Some("Invalid email format".into());
            errors.add("email", err);
        }

        // Validate company
        if self.company.is_empty() {
            let mut err = ValidationError::new("length");
            err.add_param("min".into(), &1usize);
            err.add_param("value".into(), &self.company);
            err.message = Some("Company is required".into());
            errors.add("company", err);
        }

        // Validate status
        if self.status.is_empty() {
            let mut err = ValidationError::new("length");
            err.add_param("min".into(), &1usize);
            err.add_param("value".into(), &self.status);
            err.message = Some("Status is required".into());
            errors.add("status", err);
        } else {
            // Validate that status is one of the allowed values
            let valid_statuses = vec!["New", "Contacted", "Qualified", "Lost"];
            if !valid_statuses.contains(&self.status.as_str()) {
                let mut err = ValidationError::new("invalid");
                err.message = Some("Invalid status value".into());
                errors.add("status", err);
            }
        }

        // Validate source
        if self.source.is_empty() {
            let mut err = ValidationError::new("length");
            err.add_param("min".into(), &1usize);
            err.add_param("value".into(), &self.source);
            err.message = Some("Source is required".into());
            errors.add("source", err);
        }

        // Validate phone if provided
        if let Some(ref phone) = self.phone {
            if !phone.chars().all(|c| c.is_ascii_digit() || "+-() ".contains(c)) || phone.len() < 7 {
                let mut err = ValidationError::new("custom");
                err.message = Some("Invalid phone format".into());
                errors.add("phone", err);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateLeadRequest {

    pub first_name: Option<String>,


    pub last_name: Option<String>,


    pub email: Option<String>,

    pub phone: Option<String>,


    pub company: Option<String>,


    pub status: Option<String>,


    pub source: Option<String>,

    pub title: Option<String>,
}

impl Validate for UpdateLeadRequest {
    fn validate(&self) -> Result<(), ValidationErrors> {
        use validator::ValidationErrors;

        let mut errors = ValidationErrors::new();

        // Validate first_name if provided
        if let Some(ref name) = self.first_name {
            if name.is_empty() {
                let mut err = ValidationError::new("length");
                err.add_param("min".into(), &1usize);
                err.add_param("value".into(), name);
                errors.add("first_name", err);
            }
        }

        // Validate last_name if provided
        if let Some(ref name) = self.last_name {
            if name.is_empty() {
                let mut err = ValidationError::new("length");
                err.add_param("min".into(), &1usize);
                err.add_param("value".into(), name);
                errors.add("last_name", err);
            }
        }

        // Validate email if provided
        if let Some(ref email) = self.email {
            if !email.contains('@') || !email.contains('.') {
                let err = ValidationError::new("email");
                errors.add("email", err);
            }
        }

        // Validate company if provided
        if let Some(ref comp) = self.company {
            if comp.is_empty() {
                let mut err = ValidationError::new("length");
                err.add_param("min".into(), &1usize);
                err.add_param("value".into(), comp);
                errors.add("company", err);
            }
        }

        // Validate status if provided
        if let Some(ref status) = self.status {
            if status.is_empty() {
                let mut err = ValidationError::new("length");
                err.add_param("min".into(), &1usize);
                err.add_param("value".into(), status);
                errors.add("status", err);
            }
        }

        // Validate source if provided
        if let Some(ref src) = self.source {
            if src.is_empty() {
                let mut err = ValidationError::new("length");
                err.add_param("min".into(), &1usize);
                err.add_param("value".into(), src);
                errors.add("source", err);
            }
        }

        // Validate phone if provided
        if let Some(ref phone) = self.phone {
            if !phone.chars().all(|c| c.is_ascii_digit() || "+-() ".contains(c)) || phone.len() < 7 {
                let mut err = ValidationError::new("custom");
                err.message = Some("Invalid phone format".into());
                errors.add("phone", err);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetLeadResponse {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub company: String,
    pub status: String,
    pub source: String,
    pub title: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateOpportunityRequest {

    pub name: String,

    pub amount: Option<f64>,


    pub stage: String,


    pub probability: Option<i32>,

    pub close_date: Option<chrono::NaiveDate>,


    pub contact_id: String,

    pub description: Option<String>,
}

impl Validate for CreateOpportunityRequest {
    fn validate(&self) -> Result<(), ValidationErrors> {
        use validator::ValidationErrors;

        let mut errors = ValidationErrors::new();

        // Validate name
        if self.name.is_empty() {
            let mut err = ValidationError::new("length");
            err.add_param("min".into(), &1usize);
            err.add_param("value".into(), &self.name);
            err.message = Some("Opportunity name is required".into());
            errors.add("name", err);
        }

        // Validate stage
        if self.stage.is_empty() {
            let mut err = ValidationError::new("length");
            err.add_param("min".into(), &1usize);
            err.add_param("value".into(), &self.stage);
            err.message = Some("Stage is required".into());
            errors.add("stage", err);
        }

        // Validate probability if provided
        if let Some(prob) = self.probability {
            if prob < 0 || prob > 100 {
                let mut err = ValidationError::new("range");
                err.add_param("min".into(), &0i32);
                err.add_param("max".into(), &100i32);
                err.add_param("value".into(), &prob);
                err.message = Some("Probability must be between 0 and 100".into());
                errors.add("probability", err);
            }
        }

        // Validate contact_id
        if self.contact_id.is_empty() {
            let mut err = ValidationError::new("length");
            err.add_param("min".into(), &1usize);
            err.add_param("value".into(), &self.contact_id);
            err.message = Some("Contact ID is required".into());
            errors.add("contact_id", err);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateOpportunityRequest {

    pub name: Option<String>,

    pub amount: Option<f64>,


    pub stage: Option<String>,


    pub probability: Option<i32>,

    pub close_date: Option<chrono::NaiveDate>,

    pub description: Option<String>,
}

impl Validate for UpdateOpportunityRequest {
    fn validate(&self) -> Result<(), ValidationErrors> {
        use validator::ValidationErrors;

        let mut errors = ValidationErrors::new();

        // Validate name if provided
        if let Some(ref name) = self.name {
            if name.is_empty() {
                let mut err = ValidationError::new("length");
                err.add_param("min".into(), &1usize);
                err.add_param("value".into(), name);
                errors.add("name", err);
            }
        }

        // Validate stage if provided
        if let Some(ref stage) = self.stage {
            if stage.is_empty() {
                let mut err = ValidationError::new("length");
                err.add_param("min".into(), &1usize);
                err.add_param("value".into(), stage);
                errors.add("stage", err);
            }
        }

        // Validate probability if provided
        if let Some(prob) = self.probability {
            if prob < 0 || prob > 100 {
                let mut err = ValidationError::new("range");
                err.add_param("min".into(), &0i32);
                err.add_param("max".into(), &100i32);
                err.add_param("value".into(), &prob);
                errors.add("probability", err);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateOpportunityStageRequest {

    pub stage: String,
}

impl Validate for UpdateOpportunityStageRequest {
    fn validate(&self) -> Result<(), ValidationErrors> {
        use validator::ValidationErrors;

        let mut errors = ValidationErrors::new();

        // Validate stage
        if self.stage.is_empty() {
            let mut err = ValidationError::new("length");
            err.add_param("min".into(), &1usize);
            err.add_param("value".into(), &self.stage);
            err.message = Some("Stage is required".into());
            errors.add("stage", err);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetOpportunityResponse {
    pub id: String,
    pub name: String,
    pub amount: Option<f64>,
    pub stage: String,
    pub probability: Option<i32>,
    pub close_date: Option<chrono::NaiveDate>,
    pub contact_id: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct LeadFilter {
    pub status: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ContactFilter {
    pub email: Option<String>,
    pub company: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpportunityFilter {
    pub stage: Option<String>,
    pub min_amount: Option<f64>,
    pub max_amount: Option<f64>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConvertLeadRequest {
    pub opportunity_name: String,
    pub estimated_value: Option<f64>,
}
