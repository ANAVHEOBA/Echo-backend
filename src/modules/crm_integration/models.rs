use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, FromRow, Deserialize, Serialize)]
pub struct Contact {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Contact {
    pub fn new(first_name: String, last_name: String, email: String) -> Self {
        let uuid_str = Uuid::new_v4().to_string();
        let short_uuid = &uuid_str[..std::cmp::min(8, uuid_str.len())];
        Self {
            id: format!("contact_{}", short_uuid),
            first_name,
            last_name,
            email,
            phone: None,
            company: None,
            title: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, FromRow, Deserialize, Serialize)]
pub struct Lead {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub company: String,
    pub status: String,
    pub source: String,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Lead {
    pub fn new(first_name: String, last_name: String, email: String, company: String, source: String) -> Self {
        let uuid_str = Uuid::new_v4().to_string();
        let short_uuid = &uuid_str[..std::cmp::min(8, uuid_str.len())];
        Self {
            id: format!("lead_{}", short_uuid),
            first_name,
            last_name,
            email,
            phone: None,
            company,
            status: "New".to_string(),
            source,
            title: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, FromRow, Deserialize, Serialize)]
pub struct Opportunity {
    pub id: String,
    pub name: String,
    pub amount: Option<f64>,
    pub stage: String,
    pub probability: Option<i32>,
    pub close_date: Option<chrono::NaiveDate>,
    pub contact_id: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Opportunity {
    pub fn new(name: String, contact_id: String) -> Self {
        let uuid_str = Uuid::new_v4().to_string();
        let short_uuid = &uuid_str[..std::cmp::min(8, uuid_str.len())];
        Self {
            id: format!("opp_{}", short_uuid),
            name,
            amount: None,
            stage: "New".to_string(),
            probability: Some(10),
            close_date: None,
            contact_id,
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}