//! Queue Business Logic
//!
//! This module contains business logic for job queue operations.

use std::sync::Arc;

use cms_queue::{JobEnvelope, JobId, JobQueue, JobStatus, JobType};
use uuid::Uuid;

use crate::{AppError, BizContext};

/// Queue service
pub struct QueueService;

impl QueueService {
    /// Enqueue a job
    pub async fn enqueue_job(
        queue: Arc<dyn JobQueue>,
        job_type: JobType,
        payload: serde_json::Value,
        delay: Option<std::time::Duration>,
    ) -> Result<JobId, AppError> {
        if let Some(delay) = delay {
            queue
                .enqueue_delayed(
                    JobEnvelope {
                        id: JobId(Uuid::new_v4().to_string()),
                        job_type,
                        payload,
                        status: JobStatus::Pending,
                        created_at: chrono::Utc::now(),
                        started_at: None,
                        completed_at: None,
                        error_message: None,
                        retry_count: 0,
                    },
                    delay,
                )
                .await
        } else {
            queue
                .enqueue(JobEnvelope {
                    id: JobId(Uuid::new_v4().to_string()),
                    job_type,
                    payload,
                    status: JobStatus::Pending,
                    created_at: chrono::Utc::now(),
                    started_at: None,
                    completed_at: None,
                    error_message: None,
                    retry_count: 0,
                })
                .await
        }
    }

    /// Get job status
    pub async fn get_job_status(
        queue: Arc<dyn JobQueue>,
        job_id: JobId,
    ) -> Result<Option<JobEnvelope>, AppError> {
        queue.get_job(job_id).await
    }

    /// List jobs
    pub async fn list_jobs(
        queue: Arc<dyn JobQueue>,
        status: Option<JobStatus>,
        job_type: Option<JobType>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<JobEnvelope>, AppError> {
        queue.list_jobs(status, job_type, limit, offset).await
    }

    /// Retry a failed job
    pub async fn retry_job(queue: Arc<dyn JobQueue>, job_id: JobId) -> Result<(), AppError> {
        queue.retry_job(job_id).await
    }

    /// Delete a job
    pub async fn delete_job(queue: Arc<dyn JobQueue>, job_id: JobId) -> Result<bool, AppError> {
        queue.delete_job(job_id).await
    }
}

/// Maximum retries before a job is considered permanently failed
const MAX_JOB_RETRIES: u32 = 5;

/// Maximum age (in seconds) for a job to be considered stale
const STALE_JOB_AGE_SECS: i64 = 3600; // 1 hour

/// Process reaper job (for worker)
///
/// The reaper cleans up stale and permanently-failed jobs from the queue.
/// It runs periodically to prevent the queue from accumulating dead jobs.
pub async fn process_reaper_job(
    _pool: &cms_db::PgPool,
    queue: Arc<dyn JobQueue>,
    _payload: &serde_json::Value,
) -> Result<(), AppError> {
    let now = chrono::Utc::now();
    let stale_threshold = now - chrono::Duration::seconds(STALE_JOB_AGE_SECS);

    // Find all pending and failed jobs for inspection
    let pending_jobs = queue
        .list_jobs(Some(JobStatus::Pending), None, Some(500), Some(0))
        .await?;

    let failed_jobs = queue
        .list_jobs(Some(JobStatus::Failed), None, Some(500), Some(0))
        .await?;

    let mut reaped = 0usize;

    // Reap stale pending jobs (stuck for too long — likely the worker that
    // picked them up crashed before acking)
    for job in pending_jobs {
        if job.created_at < stale_threshold {
            if job.retry_count >= MAX_JOB_RETRIES {
                // Permanently failed — delete it
                if queue.delete_job(job.id.clone()).await.unwrap_or(false) {
                    tracing::warn!(
                        "Reaper deleted permanently-failed stale job {} (type={:?}, retries={})",
                        job.id.0,
                        job.job_type,
                        job.retry_count
                    );
                    reaped += 1;
                }
            } else {
                // Retry it — may have been orphaned
                if queue.retry_job(job.id.clone()).await.is_ok() {
                    tracing::info!(
                        "Reaper re-queued stale job {} (type={:?}, age={}s)",
                        job.id.0,
                        job.job_type,
                        (now - job.created_at).num_seconds()
                    );
                    reaped += 1;
                }
            }
        }
    }

    // Reap permanently-failed jobs (exceeded retry limit)
    for job in failed_jobs {
        if job.retry_count >= MAX_JOB_RETRIES
            && queue.delete_job(job.id.clone()).await.unwrap_or(false)
        {
            tracing::warn!(
                "Reaper deleted permanently-failed job {} (type={:?}, retries={})",
                job.id.0,
                job.job_type,
                job.retry_count
            );
            reaped += 1;
        }
    }

    if reaped > 0 {
        tracing::info!("Reaper cleaned up {} job(s)", reaped);
    } else {
        tracing::debug!("Reaper: no stale or dead jobs found");
    }

    Ok(())
}
