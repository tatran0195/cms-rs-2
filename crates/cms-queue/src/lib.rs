//! CMS Job Queue
//!
//! This crate provides a job queue with pluggable backends:
//! - In-memory (default): jobs are processed in the same process
//! - Redis (optional): jobs are processed by a separate worker process
//!
//! The queue follows the same trait pattern as storage and search,
//! allowing the default deployment to run without any external services.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cms_config::QueueConfig;
use cms_error::AppError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod postgres;
pub use postgres::PostgresJobQueue;

/// Job ID type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct JobId(pub String);

impl JobId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

/// Job type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobType {
    Analytics,
    Email,
    Export,
    Git,
    Publish,
    Search,
    Usage,
    Reaper,
}

/// Job status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Retrying,
}

/// Job envelope containing job data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEnvelope {
    pub id: JobId,
    pub job_type: JobType,
    pub payload: serde_json::Value,
    pub status: JobStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub retry_count: u32,
}

impl JobEnvelope {
    pub fn new(job_type: JobType, payload: serde_json::Value) -> Self {
        Self {
            id: JobId::new(),
            job_type,
            payload,
            status: JobStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            retry_count: 0,
        }
    }
}

/// The JobQueue trait defines the interface for job queues
#[async_trait]
pub trait JobQueue: Send + Sync {
    /// Enqueue a job for immediate processing
    async fn enqueue(&self, job: JobEnvelope) -> Result<JobId, AppError>;

    /// Enqueue a job with a delay
    async fn enqueue_delayed(&self, job: JobEnvelope, delay: Duration) -> Result<JobId, AppError>;

    /// Enqueue a repeatable job (cron-like)
    async fn enqueue_repeatable(
        &self,
        job: JobEnvelope,
        schedule: String,
    ) -> Result<JobId, AppError>;

    /// Consume the next job (for worker)
    async fn consume(&self, consumer_name: &str) -> Result<JobEnvelope, AppError>;

    /// Acknowledge a job as completed
    async fn ack(&self, job_id: &JobId) -> Result<(), AppError>;

    /// Negative acknowledge (job failed, may be retried)
    async fn nack(&self, job_id: &JobId, error_message: &str) -> Result<(), AppError>;

    /// Get a job by ID
    async fn get_job(&self, job_id: JobId) -> Result<Option<JobEnvelope>, AppError>;

    /// List jobs with optional filters
    async fn list_jobs(
        &self,
        status: Option<JobStatus>,
        job_type: Option<JobType>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<JobEnvelope>, AppError>;

    /// Retry a failed job
    async fn retry_job(&self, job_id: JobId) -> Result<(), AppError>;

    /// Delete a job
    async fn delete_job(&self, job_id: JobId) -> Result<bool, AppError>;

    /// Start job consumers (for in-memory backend)
    async fn start_consumers(&self) -> Result<(), AppError>;
}

/// Create a JobQueue implementation based on configuration
pub async fn create_job_queue(config: &QueueConfig) -> Result<Arc<dyn JobQueue>, AppError> {
    match config.backend.as_str() {
        "memory" => {
            let queue = MemoryJobQueue::new(config.workers);
            Ok(Arc::new(queue))
        }
        "postgres" => Err(AppError::InvalidInput(
            "PostgreSQL queue creation requires the initialized database pool".to_string(),
        )),
        "redis" => {
            #[cfg(feature = "redis")]
            {
                if let Some(redis_url) = &config.redis_url {
                    let queue = RedisJobQueue::new(redis_url.clone(), config.max_retries).await?;
                    Ok(Arc::new(queue))
                } else {
                    Err(AppError::Storage("Redis URL not configured".to_string()))
                }
            }
            #[cfg(not(feature = "redis"))]
            {
                Err(AppError::Storage(
                    "Redis backend requires the 'redis' feature".to_string(),
                ))
            }
        }
        _ => Err(AppError::Storage(format!(
            "Unknown queue backend: {}",
            config.backend
        ))),
    }
}

/// Create a configured queue using the application's already-migrated PostgreSQL pool.
pub async fn create_job_queue_with_pool(
    config: &QueueConfig,
    pool: cms_db::PgPool,
) -> Result<Arc<dyn JobQueue>, AppError> {
    match config.backend.as_str() {
        "postgres" => Ok(Arc::new(PostgresJobQueue::new(
            pool,
            config.max_retries,
        ))),
        _ => create_job_queue(config).await,
    }
}

/// Start job consumers for the queue
pub async fn start_consumers(queue: Arc<dyn JobQueue>) -> Result<(), AppError> {
    queue.start_consumers().await
}

/// In-memory job queue implementation
pub struct MemoryJobQueue {
    sender: tokio::sync::mpsc::Sender<JobEnvelope>,
    receiver: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<JobEnvelope>>>,
    jobs: Arc<tokio::sync::Mutex<std::collections::HashMap<JobId, JobEnvelope>>>,
    num_workers: usize,
}

impl MemoryJobQueue {
    pub fn new(num_workers: usize) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel::<JobEnvelope>(1000);

        Self {
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
            jobs: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            num_workers,
        }
    }

    async fn process_job(&self, job: JobEnvelope) -> Result<(), AppError> {
        // In a real implementation, this would call the appropriate handler
        // based on job.job_type

        // For now, just mark as completed
        let mut jobs = self.jobs.lock().await;
        if let Some(existing) = jobs.get_mut(&job.id) {
            existing.status = JobStatus::Completed;
            existing.completed_at = Some(Utc::now());
        }

        Ok(())
    }
}

#[async_trait]
impl JobQueue for MemoryJobQueue {
    async fn enqueue(&self, job: JobEnvelope) -> Result<JobId, AppError> {
        let job_id = job.id.clone();

        // Store the job
        let mut jobs = self.jobs.lock().await;
        jobs.insert(job_id.clone(), job.clone());

        // Send to channel
        self.sender
            .send(job)
            .await
            .map_err(|e| AppError::Storage(format!("Failed to enqueue job: {}", e)))?;

        Ok(job_id)
    }

    async fn enqueue_delayed(&self, job: JobEnvelope, delay: Duration) -> Result<JobId, AppError> {
        let job_id = job.id.clone();

        // Store the job
        let mut jobs = self.jobs.lock().await;
        jobs.insert(job_id.clone(), job.clone());

        // Spawn a task to send after delay
        let sender = self.sender.clone();
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            sender.send(job).await.ok();
        });

        Ok(job_id)
    }

    async fn enqueue_repeatable(
        &self,
        job: JobEnvelope,
        _schedule: String,
    ) -> Result<JobId, AppError> {
        // For in-memory, repeatable jobs are treated as one-time
        self.enqueue(job).await
    }

    async fn consume(&self, _consumer_name: &str) -> Result<JobEnvelope, AppError> {
        let mut receiver = self.receiver.lock().await;

        receiver
            .recv()
            .await
            .ok_or_else(|| AppError::Storage("No jobs available".to_string()))
    }

    async fn ack(&self, job_id: &JobId) -> Result<(), AppError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Completed;
            job.completed_at = Some(Utc::now());
        }
        Ok(())
    }

    async fn nack(&self, job_id: &JobId, error_message: &str) -> Result<(), AppError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Failed;
            job.error_message = Some(error_message.to_string());
            job.completed_at = Some(Utc::now());
        }
        Ok(())
    }

    async fn get_job(&self, job_id: JobId) -> Result<Option<JobEnvelope>, AppError> {
        let jobs = self.jobs.lock().await;
        Ok(jobs.get(&job_id).cloned())
    }

    async fn list_jobs(
        &self,
        status: Option<JobStatus>,
        job_type: Option<JobType>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<JobEnvelope>, AppError> {
        let jobs = self.jobs.lock().await;

        let mut result: Vec<JobEnvelope> = jobs.values().cloned().collect();

        // Apply filters
        if let Some(status) = status {
            result.retain(|j| j.status == status);
        }
        if let Some(job_type) = job_type {
            result.retain(|j| j.job_type == job_type);
        }

        // Apply pagination
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        result = result.into_iter().skip(offset).take(limit).collect();

        Ok(result)
    }

    async fn retry_job(&self, job_id: JobId) -> Result<(), AppError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.status = JobStatus::Pending;
            job.retry_count += 1;
            job.started_at = None;
            job.completed_at = None;
            job.error_message = None;

            // Re-queue
            let sender = self.sender.clone();
            let job_clone = job.clone();
            tokio::spawn(async move {
                sender.send(job_clone).await.ok();
            });
        }
        Ok(())
    }

    async fn delete_job(&self, job_id: JobId) -> Result<bool, AppError> {
        let mut jobs = self.jobs.lock().await;
        Ok(jobs.remove(&job_id).is_some())
    }

    async fn start_consumers(&self) -> Result<(), AppError> {
        for i in 0..self.num_workers {
            let queue = Arc::new(self.clone());
            let consumer_name = format!("worker-{}", i);

            tokio::spawn(async move {
                loop {
                    match queue.consume(&consumer_name).await {
                        Ok(job) => {
                            if let Err(e) = queue.process_job(job).await {
                                tracing::error!("Error processing job: {}", e);
                            }
                            // Auto-ack for now
                            // In a real implementation, we'd have proper ack/nack
                        }
                        Err(e) => {
                            if e.to_string() != "No jobs available" {
                                tracing::error!("Error consuming job: {}", e);
                            }
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
            });
        }

        Ok(())
    }
}

impl Clone for MemoryJobQueue {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            jobs: self.jobs.clone(),
            num_workers: self.num_workers,
        }
    }
}

/// Redis job queue implementation
#[cfg(feature = "redis")]
pub struct RedisJobQueue {
    client: deadpool_redis::Pool,
    max_retries: usize,
}

#[cfg(feature = "redis")]
impl RedisJobQueue {
    pub async fn new(redis_url: String, max_retries: usize) -> Result<Self, AppError> {
        let config = deadpool_redis::Config::from_url(redis_url);
        let pool = config
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .map_err(|e| AppError::Storage(e.to_string()))?;

        Ok(Self {
            client: pool,
            max_retries,
        })
    }
}

#[cfg(feature = "redis")]
#[async_trait]
impl JobQueue for RedisJobQueue {
    async fn enqueue(&self, job: JobEnvelope) -> Result<JobId, AppError> {
        let job_id = job.id.clone();

        let mut conn = self
            .client
            .get()
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;
        let serialized = serde_json::to_string(&job)?;

        redis::cmd("LPUSH")
            .arg("cms:queue:pending")
            .arg(serialized)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        // Store job metadata
        let metadata = serde_json::json!({
            "status": "pending",
            "created_at": job.created_at.to_rfc3339(),
            "retry_count": 0,
        });

        redis::cmd("HSET")
            .arg(format!("cms:jobs:{}", job_id.0))
            .arg("metadata")
            .arg(metadata.to_string())
            .arg("payload")
            .arg(serde_json::to_string(&job.payload)?)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        Ok(job_id)
    }

    async fn enqueue_delayed(&self, job: JobEnvelope, delay: Duration) -> Result<JobId, AppError> {
        let job_id = job.id.clone();

        let mut conn = self
            .client
            .get()
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;
        let serialized = serde_json::to_string(&job)?;

        // Use Redis sorted set for delayed jobs
        let score = Utc::now().timestamp() + delay.as_secs() as i64;

        redis::cmd("ZADD")
            .arg("cms:queue:delayed")
            .arg(score)
            .arg(serialized)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        // Store job metadata
        let metadata = serde_json::json!({
            "status": "pending",
            "created_at": job.created_at.to_rfc3339(),
            "retry_count": 0,
            "delayed_until": score,
        });

        redis::cmd("HSET")
            .arg(format!("cms:jobs:{}", job_id.0))
            .arg("metadata")
            .arg(metadata.to_string())
            .arg("payload")
            .arg(serde_json::to_string(&job.payload)?)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        Ok(job_id)
    }

    async fn enqueue_repeatable(
        &self,
        _job: JobEnvelope,
        _schedule: String,
    ) -> Result<JobId, AppError> {
        // Repeatable jobs would use Redis sorted sets with recurring scores
        Err(AppError::NotFound(
            "Repeatable jobs not yet implemented for Redis backend".to_string(),
        ))
    }

    async fn consume(&self, _consumer_name: &str) -> Result<JobEnvelope, AppError> {
        let mut conn = self
            .client
            .get()
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        // Blocking pop from the queue: BLPOP returns Option<(String, String)> -> (key, element)
        let result: Option<(String, String)> = redis::cmd("BLPOP")
            .arg("cms:queue:pending")
            .arg(30) // 30 second timeout
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        let (_, serialized) =
            result.ok_or_else(|| AppError::Storage("No jobs available".to_string()))?;

        let job: JobEnvelope = serde_json::from_str(&serialized)?;

        // Update job status to processing
        let metadata = serde_json::json!({
            "status": "processing",
            "started_at": Utc::now().to_rfc3339(),
        });

        redis::cmd("HSET")
            .arg(format!("cms:jobs:{}", job.id.0))
            .arg("metadata")
            .arg(metadata.to_string())
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        Ok(job)
    }

    async fn ack(&self, job_id: &JobId) -> Result<(), AppError> {
        let mut conn = self
            .client
            .get()
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        let metadata = serde_json::json!({
            "status": "completed",
            "completed_at": Utc::now().to_rfc3339(),
        });

        redis::cmd("HSET")
            .arg(format!("cms:jobs:{}", job_id.0))
            .arg("metadata")
            .arg(metadata.to_string())
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        // Remove from queue if still there
        redis::cmd("LREM")
            .arg("cms:queue:pending")
            .arg(0)
            .arg(job_id.0.clone())
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn nack(&self, job_id: &JobId, error_message: &str) -> Result<(), AppError> {
        let mut conn = self
            .client
            .get()
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        // Get current retry count
        let retry_count: Option<i32> = redis::cmd("HGET")
            .arg(format!("cms:jobs:{}", job_id.0))
            .arg("retry_count")
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        let retry_count = retry_count.unwrap_or(0) + 1;

        if retry_count >= self.max_retries as i32 {
            // Max retries exceeded - mark as failed
            let metadata = serde_json::json!({
                "status": "failed",
                "error_message": error_message,
                "completed_at": Utc::now().to_rfc3339(),
                "retry_count": retry_count,
            });

            redis::cmd("HSET")
                .arg(format!("cms:jobs:{}", job_id.0))
                .arg("metadata")
                .arg(metadata.to_string())
                .query_async::<()>(&mut conn)
                .await
                .map_err(|e| AppError::Storage(e.to_string()))?;

            // Move to failed queue
            redis::cmd("LPUSH")
                .arg("cms:queue:failed")
                .arg(job_id.0.clone())
                .query_async::<()>(&mut conn)
                .await
                .map_err(|e| AppError::Storage(e.to_string()))?;
        } else {
            // Re-queue
            let metadata = serde_json::json!({
                "status": "retrying",
                "retry_count": retry_count,
            });

            redis::cmd("HSET")
                .arg(format!("cms:jobs:{}", job_id.0))
                .arg("metadata")
                .arg(metadata.to_string())
                .query_async::<()>(&mut conn)
                .await
                .map_err(|e| AppError::Storage(e.to_string()))?;

            redis::cmd("LPUSH")
                .arg("cms:queue:pending")
                .arg(job_id.0.clone())
                .query_async::<()>(&mut conn)
                .await
                .map_err(|e| AppError::Storage(e.to_string()))?;
        }

        Ok(())
    }

    async fn get_job(&self, job_id: JobId) -> Result<Option<JobEnvelope>, AppError> {
        let mut conn = self
            .client
            .get()
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        let metadata: Option<String> = redis::cmd("HGET")
            .arg(format!("cms:jobs:{}", job_id.0))
            .arg("metadata")
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        let payload: Option<String> = redis::cmd("HGET")
            .arg(format!("cms:jobs:{}", job_id.0))
            .arg("payload")
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        if let (Some(metadata), Some(payload)) = (metadata, payload) {
            let metadata: serde_json::Value = serde_json::from_str(&metadata)?;
            let payload: serde_json::Value = serde_json::from_str(&payload)?;

            // Note: This is a simplified reconstruction
            // In a real implementation, we'd store the full job envelope
            Ok(Some(JobEnvelope {
                id: job_id,
                job_type: JobType::Analytics, // Would need to store this
                payload,
                status: serde_json::from_value(
                    metadata.get("status").cloned().unwrap_or_default(),
                )?,
                created_at: chrono::DateTime::parse_from_rfc3339(
                    metadata
                        .get("created_at")
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                )
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
                started_at: metadata
                    .get("started_at")
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.with_timezone(&Utc)),
                completed_at: metadata
                    .get("completed_at")
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.with_timezone(&Utc)),
                error_message: metadata
                    .get("error_message")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                retry_count: metadata
                    .get("retry_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_jobs(
        &self,
        status: Option<JobStatus>,
        job_type: Option<JobType>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<JobEnvelope>, AppError> {
        // This would scan the Redis keys
        // For now, return empty
        Err(AppError::NotFound(
            "List jobs not yet implemented for Redis backend".to_string(),
        ))
    }

    async fn retry_job(&self, job_id: JobId) -> Result<(), AppError> {
        // Reset retry count and re-queue
        let mut conn = self
            .client
            .get()
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        let metadata = serde_json::json!({
            "status": "pending",
            "retry_count": 0,
        });

        redis::cmd("HSET")
            .arg(format!("cms:jobs:{}", job_id.0))
            .arg("metadata")
            .arg(metadata.to_string())
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        redis::cmd("LPUSH")
            .arg("cms:queue:pending")
            .arg(job_id.0.clone())
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn delete_job(&self, job_id: JobId) -> Result<bool, AppError> {
        let mut conn = self
            .client
            .get()
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        let deleted: i32 = redis::cmd("DEL")
            .arg(format!("cms:jobs:{}", job_id.0))
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        Ok(deleted > 0)
    }

    async fn start_consumers(&self) -> Result<(), AppError> {
        // For Redis, consumers are separate processes
        // This is a no-op for the in-process queue
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_queue_enqueue_and_consume() {
        let queue = MemoryJobQueue::new(1);
        let queue = Arc::new(queue);

        let job = JobEnvelope::new(JobType::Analytics, serde_json::json!({}));
        let job_id = queue.enqueue(job.clone()).await.unwrap();

        assert_eq!(job_id, job.id);

        // Give the consumer time to process
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Get the job
        let retrieved = queue.get_job(job_id).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_memory_queue_list_jobs() {
        let queue = MemoryJobQueue::new(1);
        let queue = Arc::new(queue);

        // Enqueue multiple jobs
        for i in 0..5 {
            let job = JobEnvelope::new(JobType::Analytics, serde_json::json!({ "index": i }));
            queue.enqueue(job).await.unwrap();
        }

        let jobs = queue.list_jobs(None, None, None, None).await.unwrap();
        assert_eq!(jobs.len(), 5);
    }
}
