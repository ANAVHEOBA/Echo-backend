use sqlx::PgPool;
use crate::modules::crm_integration::models::{Contact, Lead, Opportunity};
use crate::modules::crm_integration::schemas::{
    CreateContactRequest, CreateLeadRequest, CreateOpportunityRequest,
    ContactFilter, LeadFilter, OpportunityFilter
};

// =============================================================================
// CONTACT CRUD OPERATIONS
// =============================================================================

pub async fn create_contact(pool: &PgPool, contact_data: CreateContactRequest) -> Result<Contact, sqlx::Error> {
    let contact = Contact::new(
        contact_data.first_name,
        contact_data.last_name,
        contact_data.email,
    );

    let row = sqlx::query_as::<_, Contact>(
        r#"
        INSERT INTO contacts (id, first_name, last_name, email, phone, company, title)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, first_name, last_name, email, phone, company, title, created_at, updated_at
        "#
    )
    .bind(&contact.id)
    .bind(&contact.first_name)
    .bind(&contact.last_name)
    .bind(&contact.email)
    .bind(&contact.phone)
    .bind(&contact.company)
    .bind(&contact.title)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn get_contact_by_id(pool: &PgPool, id: &str) -> Result<Option<Contact>, sqlx::Error> {
    let result = sqlx::query_as::<_, Contact>(
        r#"
        SELECT id, first_name, last_name, email, phone, company, title, created_at, updated_at
        FROM contacts
        WHERE id = $1
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await;

    result
}

pub async fn list_contacts(pool: &PgPool, filters: ContactFilter) -> Result<Vec<Contact>, sqlx::Error> {
    let mut query_builder = sqlx::QueryBuilder::new("SELECT id, first_name, last_name, email, phone, company, title, created_at, updated_at FROM contacts WHERE 1=1");

    if let Some(email) = filters.email {
        query_builder.push(" AND email ILIKE ").push_bind(format!("%{}%", email));
    }
    if let Some(company) = filters.company {
        query_builder.push(" AND company ILIKE ").push_bind(format!("%{}%", company));
    }

    query_builder.push(" ORDER BY created_at DESC");

    let query = query_builder.build_query_as::<Contact>();
    query.fetch_all(pool).await
}

pub async fn update_contact(
    pool: &PgPool,
    id: &str,
    first_name: Option<String>,
    last_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    company: Option<String>,
    title: Option<String>,
) -> Result<Option<Contact>, sqlx::Error> {
    let mut query_builder = sqlx::QueryBuilder::new("UPDATE contacts SET updated_at = NOW()");

    if let Some(ref name) = first_name {
        query_builder.push(", first_name = ").push_bind(name);
    }
    if let Some(ref name) = last_name {
        query_builder.push(", last_name = ").push_bind(name);
    }
    if let Some(ref addr) = email {
        query_builder.push(", email = ").push_bind(addr);
    }
    if let Some(ref num) = phone {
        query_builder.push(", phone = ").push_bind(num);
    }
    if let Some(ref comp) = company {
        query_builder.push(", company = ").push_bind(comp);
    }
    if let Some(ref t) = title {
        query_builder.push(", title = ").push_bind(t);
    }

    query_builder.push(" WHERE id = ").push_bind(id);
    query_builder.push(" RETURNING id, first_name, last_name, email, phone, company, title, created_at, updated_at");

    let query = query_builder.build_query_as::<Contact>();

    query.fetch_optional(pool).await
}

pub async fn delete_contact(pool: &PgPool, id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM contacts WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

// =============================================================================
// OPPORTUNITY CRUD OPERATIONS
// =============================================================================

pub async fn create_opportunity(pool: &PgPool, opportunity_data: CreateOpportunityRequest) -> Result<Opportunity, sqlx::Error> {
    let opportunity = Opportunity::new(
        opportunity_data.name.clone(),
        opportunity_data.contact_id.clone(),
    );

    let row = sqlx::query_as::<_, Opportunity>(
        r#"
        INSERT INTO opportunities (id, name, amount, stage, probability, close_date, contact_id, description, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, name, amount, stage, probability, close_date, contact_id, description, created_at, updated_at
        "#
    )
    .bind(&opportunity.id)
    .bind(&opportunity_data.name)
    .bind(opportunity_data.amount)
    .bind(&opportunity_data.stage)
    .bind(opportunity_data.probability)
    .bind(opportunity_data.close_date)
    .bind(&opportunity_data.contact_id)
    .bind(opportunity_data.description)
    .bind(&opportunity.created_at)
    .bind(&opportunity.updated_at)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn get_opportunity_by_id(pool: &PgPool, id: &str) -> Result<Option<Opportunity>, sqlx::Error> {
    sqlx::query_as::<_, Opportunity>(
        r#"
        SELECT id, name, amount, stage, probability, close_date, contact_id, description, created_at, updated_at
        FROM opportunities
        WHERE id = $1
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn list_opportunities(pool: &PgPool, filters: OpportunityFilter) -> Result<Vec<Opportunity>, sqlx::Error> {
    let mut query_builder = sqlx::QueryBuilder::new("SELECT id, name, amount, stage, probability, close_date, contact_id, description, created_at, updated_at FROM opportunities WHERE 1=1");

    if let Some(stage) = filters.stage {
        query_builder.push(" AND stage = ").push_bind(stage);
    }
    if let Some(min) = filters.min_amount {
        query_builder.push(" AND amount >= ").push_bind(min);
    }
    if let Some(max) = filters.max_amount {
        query_builder.push(" AND amount <= ").push_bind(max);
    }
    if let Some(start) = filters.start_date {
        query_builder.push(" AND close_date >= ").push_bind(start);
    }
    if let Some(end) = filters.end_date {
        query_builder.push(" AND close_date <= ").push_bind(end);
    }

    query_builder.push(" ORDER BY created_at DESC");

    let query = query_builder.build_query_as::<Opportunity>();
    query.fetch_all(pool).await
}

pub async fn update_opportunity(
    pool: &PgPool,
    id: &str,
    name: Option<String>,
    amount: Option<f64>,
    stage: Option<String>,
    probability: Option<i32>,
    close_date: Option<chrono::NaiveDate>,
    description: Option<String>,
) -> Result<Option<Opportunity>, sqlx::Error> {
    let mut query_builder = sqlx::QueryBuilder::new("UPDATE opportunities SET updated_at = NOW()");

    if let Some(ref n) = name {
        query_builder.push(", name = ").push_bind(n);
    }
    if let Some(amt) = amount {
        query_builder.push(", amount = ").push_bind(amt);
    }
    if let Some(ref s) = stage {
        query_builder.push(", stage = ").push_bind(s);
    }
    if let Some(prob) = probability {
        query_builder.push(", probability = ").push_bind(prob);
    }
    if let Some(date) = close_date {
        query_builder.push(", close_date = ").push_bind(date);
    }
    if let Some(ref desc) = description {
        query_builder.push(", description = ").push_bind(desc);
    }

    query_builder.push(" WHERE id = ").push_bind(id);
    query_builder.push(" RETURNING id, name, amount, stage, probability, close_date, contact_id, description, created_at, updated_at");

    let query = query_builder.build_query_as::<Opportunity>();

    match query.fetch_optional(pool).await {
        Ok(opportunity) => Ok(opportunity),
        Err(e) => Err(e),
    }
}

pub async fn update_opportunity_stage(
    pool: &PgPool,
    id: &str,
    stage: &str,
) -> Result<Option<Opportunity>, sqlx::Error> {
    sqlx::query_as::<_, Opportunity>(
        r#"
        UPDATE opportunities
        SET stage = $1, updated_at = NOW()
        WHERE id = $2
        RETURNING id, name, amount, stage, probability, close_date, contact_id, description, created_at, updated_at
        "#
    )
    .bind(stage)
    .bind(id)
    .fetch_optional(pool)
    .await
}


pub async fn delete_opportunity(pool: &PgPool, id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM opportunities WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

// =============================================================================
// LEAD CRUD OPERATIONS
// =============================================================================

pub async fn create_lead(pool: &PgPool, lead_data: CreateLeadRequest) -> Result<Lead, sqlx::Error> {
    let lead = Lead::new(
        lead_data.first_name,
        lead_data.last_name,
        lead_data.email,
        lead_data.company,
        lead_data.source,
    );

    let row = sqlx::query_as::<_, Lead>(
        r#"
        INSERT INTO leads (id, first_name, last_name, email, phone, company, status, source, title)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, first_name, last_name, email, phone, company, status, source, title, created_at, updated_at
        "#
    )
    .bind(&lead.id)
    .bind(&lead.first_name)
    .bind(&lead.last_name)
    .bind(&lead.email)
    .bind(&lead.phone)
    .bind(&lead.company)
    .bind(&lead.status)
    .bind(&lead.source)
    .bind(&lead.title)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn get_lead_by_id(pool: &PgPool, id: &str) -> Result<Option<Lead>, sqlx::Error> {
    let result = sqlx::query_as::<_, Lead>(
        r#"
        SELECT id, first_name, last_name, email, phone, company, status, source, title, created_at, updated_at
        FROM leads
        WHERE id = $1
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await;

    result
}

pub async fn list_leads(pool: &PgPool, filters: LeadFilter) -> Result<Vec<Lead>, sqlx::Error> {
    let mut query_builder = sqlx::QueryBuilder::new("SELECT id, first_name, last_name, email, phone, company, status, source, title, created_at, updated_at FROM leads WHERE 1=1");

    if let Some(status) = filters.status {
        query_builder.push(" AND status = ").push_bind(status);
    }
    if let Some(source) = filters.source {
        query_builder.push(" AND source = ").push_bind(source);
    }

    query_builder.push(" ORDER BY created_at DESC");

    let query = query_builder.build_query_as::<Lead>();
    query.fetch_all(pool).await
}

pub async fn update_lead(
    pool: &PgPool,
    id: &str,
    first_name: Option<String>,
    last_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    company: Option<String>,
    status: Option<String>,
    source: Option<String>,
    title: Option<String>,
) -> Result<Option<Lead>, sqlx::Error> {
    let mut query_builder = sqlx::QueryBuilder::new("UPDATE leads SET updated_at = NOW()");

    if let Some(ref name) = first_name {
        query_builder.push(", first_name = ").push_bind(name);
    }
    if let Some(ref name) = last_name {
        query_builder.push(", last_name = ").push_bind(name);
    }
    if let Some(ref addr) = email {
        query_builder.push(", email = ").push_bind(addr);
    }
    if let Some(ref num) = phone {
        query_builder.push(", phone = ").push_bind(num);
    }
    if let Some(ref comp) = company {
        query_builder.push(", company = ").push_bind(comp);
    }
    if let Some(ref stat) = status {
        query_builder.push(", status = ").push_bind(stat);
    }
    if let Some(ref src) = source {
        query_builder.push(", source = ").push_bind(src);
    }
    if let Some(ref t) = title {
        query_builder.push(", title = ").push_bind(t);
    }

    query_builder.push(" WHERE id = ").push_bind(id);
    query_builder.push(" RETURNING id, first_name, last_name, email, phone, company, status, source, title, created_at, updated_at");

    let query = query_builder.build_query_as::<Lead>();

    match query.fetch_optional(pool).await {
        Ok(lead) => Ok(lead),
        Err(e) => Err(e),
    }
}

pub async fn delete_lead(pool: &PgPool, id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM leads WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}