//! Durable PostgreSQL-backed queue.
//!
//! Claims use `FOR UPDATE SKIP LOCKED`; each job is persisted before a worker can
//! consume it and remains recoverable after process restart or worker failure.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cms_db::PgPool;
use cms_error::AppError;
use sqlx::{self, FromRow, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{JobEnvelope, JobId, JobQueue, JobStatus, JobType};

#[derive(Debug, FromRow)]
struct JobRow {
    id: String,
    job_type: String,
    payload: serde_json::Value,
    status: String,
    retry_count: i32,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}

impl TryFrom<JobRow> for JobEnvelope {
    type Error = AppError;

    fn try_from(row: JobRow) -> Result<Self, Self::Error> {
        let job_type = serde_json::from_value(serde_json::Value::String(row.job_type))?;
        let status = serde_json::from_value(serde_json::Value::String(row.status))?;
        Ok(Self {
            id: JobId(row.id),
            job_type,
            payload: row.payload,
            status,
            created_at: row.created_at,
            started_at: row.started_at,
            completed_at: row.completed_at,
            error_message: row.error_message,
            retry_count: row.retry_count.max(0) as u32,
        })
    }
}

#[derive(Clone)]
pub struct PostgresJobQueue {
    pool: PgPool,
    max_retries: usize,
    poll_interval: Duration,
    last_lease_reap: std::sync::Arc<tokio::sync::Mutex<tokio::time::Instant>>,
}

impl PostgresJobQueue {
    pub fn new(pool: PgPool, max_retries: usize) -> Self {
        Self {
            pool,
            max_retries,
            poll_interval: Duration::from_millis(500),
            last_lease_reap: std::sync::Arc::new(tokio::sync::Mutex::new(
                tokio::time::Instant::now(),
            )),
        }
    }

    async fn reap_expired_leases(&self) -> Result<(), AppError> {
        let mut last_reap = self.last_lease_reap.lock().await;
        if last_reap.elapsed() < Duration::from_secs(30) {
            return Ok(());
        }
        *last_reap = tokio::time::Instant::now();
        drop(last_reap);

        let max_retries = self.max_retries.max(1).min(i32::MAX as usize) as i32;
        sqlx::query(
            r#"WITH expired_jobs AS (
                   UPDATE "CmsJob"
                   SET status = CASE WHEN retry_count + 1 >= $1 THEN 'failed' ELSE 'retrying' END,
                       retry_count = retry_count + 1,
                       error_message = 'Worker lease expired; job will be retried',
                       available_at = NOW(),
                       completed_at = CASE WHEN retry_count + 1 >= $1 THEN NOW() ELSE NULL END,
                       locked_by = NULL,
                       locked_at = NULL
                   WHERE status = 'processing'
                     AND locked_at < NOW() - INTERVAL '1 hour'
                   RETURNING payload, status
               )
               UPDATE "Deployment" AS deployment
               SET status = CASE WHEN expired_jobs.status = 'failed'
                                 THEN 'FAILED'::"DeploymentStatus"
                                 ELSE 'PENDING'::"DeploymentStatus" END,
                   error_message = CASE WHEN expired_jobs.status = 'failed'
                                        THEN 'Worker lease expired after maximum retries'
                                        ELSE NULL END,
                   updated_at = NOW()
               FROM expired_jobs
               WHERE deployment.id = expired_jobs.payload->>'deployment_id'
                 AND deployment.status IN ('BUILDING', 'DEPLOYING')"#,
        )
        .bind(max_retries)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    fn job_type_text(job_type: &JobType) -> Result<String, AppError> {
        serde_json::to_value(job_type)?
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| AppError::InvalidInput("Invalid queue job type".to_string()))
    }

    fn status_text(status: &JobStatus) -> Result<String, AppError> {
        serde_json::to_value(status)?
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| AppError::InvalidInput("Invalid queue job status".to_string()))
    }

    async fn insert(
        &self,
        job: JobEnvelope,
        available_at: DateTime<Utc>,
    ) -> Result<JobId, AppError> {
        let id = if job.id.0.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            job.id.0.clone()
        };
        let job_type = Self::job_type_text(&job.job_type)?;
        sqlx::query(
            r#"INSERT INTO "CmsJob" (id, job_type, payload, status, retry_count,
                                      error_message, created_at, started_at, completed_at,
                                      available_at)
               VALUES ($1, $2, $3, 'pending', 0, NULL, $4, NULL, NULL, $5)"#,
        )
        .bind(&id)
        .bind(job_type)
        .bind(job.payload)
        .bind(job.created_at)
        .bind(available_at)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(JobId(id))
    }

    async fn load_one(&self, id: &str) -> Result<Option<JobEnvelope>, AppError> {
        let row = sqlx::query_as::<_, JobRow>(
            r#"SELECT id, job_type, payload, status, retry_count, error_message,
                      created_at, started_at, completed_at
               FROM "CmsJob" WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;
        row.map(TryInto::try_into).transpose()
    }
}

#[async_trait]
impl JobQueue for PostgresJobQueue {
    async fn enqueue(&self, job: JobEnvelope) -> Result<JobId, AppError> {
        self.insert(job, Utc::now()).await
    }

    async fn enqueue_delayed(&self, job: JobEnvelope, delay: Duration) -> Result<JobId, AppError> {
        let delay_seconds = delay.as_secs().min(i64::MAX as u64) as i64;
        let available_at = Utc::now() + chrono::Duration::seconds(delay_seconds);
        self.insert(job, available_at).await
    }

    async fn enqueue_repeatable(
        &self,
        _job: JobEnvelope,
        _schedule: String,
    ) -> Result<JobId, AppError> {
        Err(AppError::InvalidInput(
            "Cron scheduling is not supported by the PostgreSQL queue; schedule jobs from a durable scheduler".to_string(),
        ))
    }

    async fn consume(&self, consumer_name: &str) -> Result<JobEnvelope, AppError> {
        loop {
            self.reap_expired_leases().await?;
            let mut tx = self.pool.begin().await.map_err(AppError::Database)?;
            let row = sqlx::query_as::<_, JobRow>(
                r#"SELECT id, job_type, payload, status, retry_count, error_message,
                          created_at, started_at, completed_at
                   FROM "CmsJob"
                   WHERE status IN ('pending', 'retrying') AND available_at <= NOW()
                   ORDER BY available_at, created_at
                   LIMIT 1
                   FOR UPDATE SKIP LOCKED"#,
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(AppError::Database)?;

            if let Some(row) = row {
                sqlx::query(
                    r#"UPDATE "CmsJob"
                       SET status = 'processing', started_at = NOW(), locked_by = $2,
                           locked_at = NOW()
                       WHERE id = $1"#,
                )
                .bind(&row.id)
                .bind(consumer_name)
                .execute(&mut *tx)
                .await
                .map_err(AppError::Database)?;
                tx.commit().await.map_err(AppError::Database)?;
                let mut job = JobEnvelope::try_from(row)?;
                job.status = JobStatus::Processing;
                job.started_at = Some(Utc::now());
                return Ok(job);
            }

            tx.rollback().await.map_err(AppError::Database)?;
            tokio::time::sleep(self.poll_interval).await;
        }
    }

    async fn ack(&self, job_id: &JobId) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"UPDATE "CmsJob"
               SET status = 'completed', completed_at = NOW(), locked_by = NULL, locked_at = NULL
               WHERE id = $1 AND status = 'processing'"#,
        )
        .bind(&job_id.0)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;
        if result.rows_affected() == 0 {
            return Err(AppError::Conflict(format!(
                "Job {} is not being processed",
                job_id.0
            )));
        }
        Ok(())
    }

    async fn nack(&self, job_id: &JobId, error_message: &str) -> Result<(), AppError> {
        let retry_count: Option<i32> = sqlx::query_scalar(
            r#"SELECT retry_count FROM "CmsJob" WHERE id = $1 AND status = 'processing'"#,
        )
        .bind(&job_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;
        let retry_count = retry_count
            .ok_or_else(|| AppError::NotFound(format!("Processing job {} not found", job_id.0)))?
            .saturating_add(1);

        if retry_count as usize >= self.max_retries.max(1) {
            sqlx::query(
                r#"UPDATE "CmsJob"
                   SET status = 'failed', retry_count = $2, error_message = $3,
                       completed_at = NOW(), locked_by = NULL, locked_at = NULL
                   WHERE id = $1"#,
            )
            .bind(&job_id.0)
            .bind(retry_count)
            .bind(error_message)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        } else {
            let backoff_seconds = 2_i64.pow(retry_count.clamp(1, 8) as u32).min(300);
            sqlx::query(
                r#"UPDATE "CmsJob"
                   SET status = 'retrying', retry_count = $2, error_message = $3,
                       available_at = NOW() + make_interval(secs => $4::double precision),
                       locked_by = NULL, locked_at = NULL
                   WHERE id = $1"#,
            )
            .bind(&job_id.0)
            .bind(retry_count)
            .bind(error_message)
            .bind(backoff_seconds as f64)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        }
        Ok(())
    }

    async fn get_job(&self, job_id: JobId) -> Result<Option<JobEnvelope>, AppError> {
        self.load_one(&job_id.0).await
    }

    async fn list_jobs(
        &self,
        status: Option<JobStatus>,
        job_type: Option<JobType>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<JobEnvelope>, AppError> {
        let mut query = QueryBuilder::<Postgres>::new(
            r#"SELECT id, job_type, payload, status, retry_count, error_message,
                      created_at, started_at, completed_at FROM "CmsJob" WHERE TRUE"#,
        );
        if let Some(status) = status {
            query
                .push(" AND status = ")
                .push_bind(Self::status_text(&status)?);
        }
        if let Some(job_type) = job_type {
            query
                .push(" AND job_type = ")
                .push_bind(Self::job_type_text(&job_type)?);
        }
        query
            .push(" ORDER BY created_at DESC LIMIT ")
            .push_bind(limit.unwrap_or(100).clamp(1, 500) as i64)
            .push(" OFFSET ")
            .push_bind(offset.unwrap_or(0).min(i64::MAX as usize) as i64);

        let rows = query
            .build_query_as::<JobRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn retry_job(&self, job_id: JobId) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"UPDATE "CmsJob"
               SET status = 'pending', retry_count = 0, error_message = NULL,
                   created_at = NOW(), available_at = NOW(), started_at = NULL,
                   completed_at = NULL, locked_by = NULL, locked_at = NULL
               WHERE id = $1 AND status = 'failed'"#,
        )
        .bind(&job_id.0)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Failed job {} not found",
                job_id.0
            )));
        }
        Ok(())
    }

    async fn delete_job(&self, job_id: JobId) -> Result<bool, AppError> {
        let result = sqlx::query(r#"DELETE FROM "CmsJob" WHERE id = $1"#)
            .bind(&job_id.0)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(result.rows_affected() > 0)
    }

    async fn start_consumers(&self) -> Result<(), AppError> {
        // Consumers need service dependencies (storage, mailer, search); the
        // cms-worker crate owns that processing loop.
        Ok(())
    }
}
