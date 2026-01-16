use crate::config::DbPool;
use crate::modules::events::models::Event;
use crate::modules::events::schemas::EventFilter;
use sqlx::Row;
use uuid::Uuid;

pub async fn create_event(
    pool: &DbPool,
    event: Event,
) -> Result<Event, sqlx::Error> {
    let result = sqlx::query_as::<_, Event>(
        r#"
        INSERT INTO events (id, event_type, source, external_id, payload, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (source, external_id) DO NOTHING
        RETURNING *
        "#
    )
    .bind(&event.id)
    .bind(&event.event_type)
    .bind(&event.source)
    .bind(&event.external_id)
    .bind(&event.payload)
    .bind(&event.created_at)
    .bind(&event.updated_at)
    .fetch_one(pool)
    .await?;

    Ok(result)
}

pub async fn get_event_by_id(
    pool: &DbPool,
    event_id: &Uuid,
) -> Result<Option<Event>, sqlx::Error> {
    let result = sqlx::query_as::<_, Event>(
        "SELECT * FROM events WHERE id = $1"
    )
    .bind(event_id)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

pub async fn list_events(
    pool: &DbPool,
    filter: EventFilter,
) -> Result<Vec<Event>, sqlx::Error> {
    let limit = filter.limit.unwrap_or(50).min(200);
    let offset = filter.page.unwrap_or(0) * limit;

    let mut query = String::from("SELECT * FROM events WHERE 1=1");
    
    if let Some(ref event_type) = filter.event_type {
        query.push_str(&format!(" AND event_type = '{}'", event_type));
    }
    
    if let Some(ref source) = filter.source {
        query.push_str(&format!(" AND source = '{}'", source));
    }
    
    if let Some(processed) = filter.processed {
        if processed {
            query.push_str(" AND processed_at IS NOT NULL");
        } else {
            query.push_str(" AND processed_at IS NULL");
        }
    }
    
    query.push_str(" ORDER BY created_at DESC");
    query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

    let results = sqlx::query_as::<_, Event>(&query)
        .fetch_all(pool)
        .await?;

    Ok(results)
}

pub async fn mark_event_processed(
    pool: &DbPool,
    event_id: &Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE events SET processed_at = NOW(), updated_at = NOW() WHERE id = $1"
    )
    .bind(event_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_event_stats(
    pool: &DbPool,
) -> Result<(i64, i64, i64), sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as total,
            COUNT(processed_at) as processed,
            COUNT(*) FILTER (WHERE processed_at IS NULL) as pending
        FROM events
        "#
    )
    .fetch_one(pool)
    .await?;

    let total: i64 = row.get("total");
    let processed: i64 = row.get("processed");
    let pending: i64 = row.get("pending");

    Ok((total, processed, pending))
}
