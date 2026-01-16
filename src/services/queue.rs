use crate::config::RedisPool;
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Job {
    pub id: Uuid,
    pub task_type: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

impl Job {
    pub fn new(task_type: &str, payload: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_type: task_type.to_string(),
            payload,
            created_at: Utc::now(),
        }
    }

    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }
}

pub struct Queue {
    pool: RedisPool,
    queue_name: String,
}

impl Queue {
    pub fn new(pool: RedisPool) -> Self {
        Self {
            pool,
            queue_name: "jobs:default".to_string(),
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.queue_name = name.to_string();
        self
    }

    pub async fn enqueue(&self, job: &Job) -> Result<(), QueueError> {
        let serialized = serde_json::to_string(job)
            .map_err(|e| QueueError::Serialization(e.to_string()))?;

        let mut conn = self.pool.clone();

        let _: () = conn.lpush(&self.queue_name, serialized).await
            .map_err(|e| QueueError::Redis(e.to_string()))?;

        Ok(())
    }

    pub async fn dequeue(&self) -> Result<Option<Job>, QueueError> {
        let mut conn = self.pool.clone();

        // RPOP returns Option<String> directly
        let result: Option<String> = conn.rpop(&self.queue_name, None).await
            .map_err(|e| QueueError::Redis(e.to_string()))?;

        match result {
            Some(data) => {
                let job: Job = serde_json::from_str(&data)
                    .map_err(|e| QueueError::Serialization(e.to_string()))?;
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    pub async fn clear(&self) -> Result<(), QueueError> {
        let mut conn = self.pool.clone();
            
        let _: () = redis::cmd("DEL").arg(&self.queue_name).query_async(&mut conn).await
            .map_err(|e| QueueError::Redis(e.to_string()))?;
            
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Redis error: {0}")]
    Redis(String),
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}