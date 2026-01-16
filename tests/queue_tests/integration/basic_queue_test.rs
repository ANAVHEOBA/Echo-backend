///! Integration Tests for Job Queue System

use echo_backend::services::queue::{Queue, Job};
use crate::common::create_test_config;
use uuid::Uuid;
use serde_json::json;

// =============================================================================
// QUEUE LIFECYCLE TESTS
// =============================================================================

#[tokio::test]
async fn test_queue_lifecycle() {
    // 1. Setup
    let config = create_test_config().await;
    let pool = echo_backend::config::create_redis_pool(&config).await.expect("Failed to connect to Redis");
    
    // Initialize Queue with unique name to avoid collision
    let queue_name = format!("jobs:test:{}", Uuid::new_v4());
    let queue = Queue::new(pool).with_name(&queue_name);
    
    // Clear just in case
    let _ = queue.clear().await;

    // 2. Create a Job
    let job_id = Uuid::new_v4();
    let payload = json!({ "message": "hello world", "attempt": 1 });
    
    // Assuming Job::new(type, payload)
    let job = Job::new("email_delivery", payload.clone()).with_id(job_id);

    // 3. Enqueue the job
    let enqueue_result = queue.enqueue(&job).await;
    assert!(enqueue_result.is_ok(), "Enqueue operation should succeed");

    // 4. Dequeue the job
    let dequeue_result = queue.dequeue().await;
    assert!(dequeue_result.is_ok(), "Dequeue operation should succeed");
    
    let fetched_opt = dequeue_result.unwrap();
    assert!(fetched_opt.is_some(), "Should have received the job we just enqueued");
    
    let fetched_job = fetched_opt.unwrap();
    
    // 5. Verify Job Details
    assert_eq!(fetched_job.id, job_id, "Job ID should match");
    assert_eq!(fetched_job.task_type, "email_delivery", "Task type should match");
    assert_eq!(fetched_job.payload, payload, "Payload should match");
    
    // Cleanup
    let _ = queue.clear().await;
}

#[tokio::test]
async fn test_queue_fifo_ordering() {
    let config = create_test_config().await;
    let pool = echo_backend::config::create_redis_pool(&config).await.expect("Failed to connect to Redis");
    
    // Initialize Queue with unique name
    let queue_name = format!("jobs:test:fifo:{}", Uuid::new_v4());
    let queue = Queue::new(pool).with_name(&queue_name);
    let _ = queue.clear().await;

    // Enqueue 3 jobs
    for i in 1..=3 {
        let job = Job::new("sequence_test", json!({"order": i}));
        queue.enqueue(&job).await.expect("Enqueue failed");
    }

    // Dequeue and verify order
    for i in 1..=3 {
        let fetched = queue.dequeue().await.expect("Dequeue failed").expect("Queue empty");
        assert_eq!(fetched.payload["order"], i, "Jobs should be processed in FIFO order");
    }
    
    // Cleanup
    let _ = queue.clear().await;
}