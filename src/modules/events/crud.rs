use crate::config::DbPool;
use crate::modules::events::models::{Event, WebhookSubscription};
use crate::modules::events::schemas::EventFilter;
use sqlx::Row;
use uuid::Uuid;

pub async fn create_event(
    pool: &DbPool,
    event: Event,
) -> Result<Option<Event>, sqlx::Error> {
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
    .fetch_optional(pool)
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
) -> Result<(i64, i64, i64, i64), sqlx::Error> {
    // Events stats
    let event_stats = sqlx::query(
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

    let total: i64 = event_stats.get("total");
    let processed: i64 = event_stats.get("processed");
    let pending: i64 = event_stats.get("pending");

    // Failed events stats (from event_logs)
    let failed_stats = sqlx::query(
        "SELECT COUNT(*) as failed FROM event_logs WHERE status = 'failed'"
    )
    .fetch_one(pool)
    .await?;

    let failed: i64 = failed_stats.get("failed");

    Ok((total, processed, pending, failed))
}

// Subscriptions CRUD

pub async fn create_subscription(
    pool: &DbPool,
    subscription: WebhookSubscription,
) -> Result<WebhookSubscription, sqlx::Error> {
    let result = sqlx::query_as::<_, WebhookSubscription>(
        r#"
        INSERT INTO webhook_subscriptions (id, user_id, platform, webhook_url, secret, event_types, active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#
    )
    .bind(&subscription.id)
    .bind(&subscription.user_id)
    .bind(&subscription.platform)
    .bind(&subscription.webhook_url)
    .bind(&subscription.secret)
    .bind(&subscription.event_types)
    .bind(&subscription.active)
    .bind(&subscription.created_at)
    .bind(&subscription.updated_at)
    .fetch_one(pool)
    .await?;

    Ok(result)
}

pub async fn list_subscriptions(
    pool: &DbPool,
    user_id: &Uuid,
) -> Result<Vec<WebhookSubscription>, sqlx::Error> {
    let result = sqlx::query_as::<_, WebhookSubscription>(
        "SELECT * FROM webhook_subscriptions WHERE user_id = $1 ORDER BY created_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(result)
}

pub async fn delete_subscription(
    pool: &DbPool,
    id: &Uuid,
    user_id: &Uuid,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM webhook_subscriptions WHERE id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}