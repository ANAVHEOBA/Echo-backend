use redis::{aio::ConnectionManager, Client};

use super::app::AppConfig;

pub type RedisPool = ConnectionManager;

pub async fn create_redis_pool(config: &AppConfig) -> Result<RedisPool, redis::RedisError> {
    let client = Client::open(config.redis_url.as_str())?;
    let manager = ConnectionManager::new(client).await?;

    tracing::info!("Redis connected");

    Ok(manager)
}

pub async fn create_redis_pool_optional(config: &AppConfig) -> Option<RedisPool> {
    match create_redis_pool(config).await {
        Ok(pool) => Some(pool),
        Err(e) => {
            tracing::warn!("Redis not available: {}. Token blacklisting will use in-memory fallback.", e);
            None
        }
    }
}
