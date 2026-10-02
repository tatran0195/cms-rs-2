//! Export entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, Timestamp};

/// Export status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "\"ExportStatus\"", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExportStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// Export format
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Html,
    Pdf,
    Markdown,
    Epub,
    // Uppercase aliases for backward compatibility with biz code
    HTML,
    PDF,
    MARKDOWN,
    EPUB,
}

impl sqlx::Type<sqlx::Postgres> for ExportFormat {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("\"ExportFormat\"")
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for ExportFormat {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        match s {
            "HTML" | "html" => Ok(ExportFormat::Html),
            "PDF" | "pdf" => Ok(ExportFormat::Pdf),
            "MARKDOWN" | "markdown" => Ok(ExportFormat::Markdown),
            "EPUB" | "epub" => Ok(ExportFormat::Epub),
            other => Err(format!("unknown ExportFormat: {}", other).into()),
        }
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Postgres> for ExportFormat {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = match self {
            ExportFormat::Html | ExportFormat::HTML => "HTML",
            ExportFormat::Pdf | ExportFormat::PDF => "PDF",
            ExportFormat::Markdown | ExportFormat::MARKDOWN => "MARKDOWN",
            ExportFormat::Epub | ExportFormat::EPUB => "EPUB",
        };
        <&str as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&s, buf)
    }
}

/// Export snapshot entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExportSnapshot {
    pub id: Id,
    pub project_id: Id,
    pub branch_id: Option<Id>,
    pub language_id: Option<Id>,
    pub created_at: DateTime<Utc>,
}

/// Export snapshot response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExportSnapshotResponse {
    pub id: Id,
    pub project_id: Id,
    pub branch_id: Option<Id>,
    pub language_id: Option<Id>,
    pub created_at: DateTime<Utc>,
}

impl From<ExportSnapshot> for ExportSnapshotResponse {
    fn from(snapshot: ExportSnapshot) -> Self {
        Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            branch_id: snapshot.branch_id,
            language_id: snapshot.language_id,
            created_at: snapshot.created_at,
        }
    }
}

/// Export job entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExportJob {
    pub id: Id,
    pub snapshot_id: Id,
    pub format: ExportFormat,
    pub status: ExportStatus,
    pub output_path: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Export job response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExportJobResponse {
    pub id: Id,
    pub snapshot_id: Id,
    pub format: ExportFormat,
    pub status: ExportStatus,
    pub output_path: Option<String>,
    pub download_url: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ExportJob> for ExportJobResponse {
    fn from(job: ExportJob) -> Self {
        Self {
            id: job.id,
            snapshot_id: job.snapshot_id,
            format: job.format,
            status: job.status,
            output_path: job.output_path.clone(),
            download_url: job
                .output_path
                .as_deref()
                .map(|p| format!("/api/export/download/{}", p)),
            error_message: job.error_message,
            started_at: job.started_at,
            completed_at: job.completed_at,
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}

/// Create export job request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateExportJobRequest {
    pub snapshot_id: Id,
    pub format: ExportFormat,
}

/// Export artifact entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExportArtifact {
    pub id: Id,
    pub job_id: Id,
    pub file_name: String,
    pub file_size: i64,
    pub storage_path: String,
    pub download_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Export artifact response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExportArtifactResponse {
    pub id: Id,
    pub job_id: Id,
    pub file_name: String,
    pub file_size: i64,
    pub download_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<ExportArtifact> for ExportArtifactResponse {
    fn from(artifact: ExportArtifact) -> Self {
        Self {
            id: artifact.id,
            job_id: artifact.job_id,
            file_name: artifact.file_name,
            file_size: artifact.file_size,
            download_url: artifact.download_url,
            created_at: artifact.created_at,
        }
    }
}

/// Export schedule entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExportSchedule {
    pub id: Id,
    pub project_id: Id,
    pub format: ExportFormat,
    pub frequency: String,
    pub day_of_week: Option<i32>,
    pub day_of_month: Option<i32>,
    pub time_of_day: String,
    pub is_active: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Export schedule response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExportScheduleResponse {
    pub id: Id,
    pub project_id: Id,
    pub format: ExportFormat,
    pub frequency: String,
    pub day_of_week: Option<i32>,
    pub day_of_month: Option<i32>,
    pub time_of_day: String,
    pub is_active: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ExportSchedule> for ExportScheduleResponse {
    fn from(schedule: ExportSchedule) -> Self {
        Self {
            id: schedule.id,
            project_id: schedule.project_id,
            format: schedule.format,
            frequency: schedule.frequency,
            day_of_week: schedule.day_of_week,
            day_of_month: schedule.day_of_month,
            time_of_day: schedule.time_of_day,
            is_active: schedule.is_active,
            last_run_at: schedule.last_run_at,
            next_run_at: schedule.next_run_at,
            created_at: schedule.created_at,
            updated_at: schedule.updated_at,
        }
    }
}

/// Create export schedule request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateExportScheduleRequest {
    pub project_id: Id,
    pub format: ExportFormat,
    pub frequency: String,
    #[serde(default)]
    pub day_of_week: Option<i32>,
    #[serde(default)]
    pub day_of_month: Option<i32>,
    #[serde(default = "default_time")]
    pub time_of_day: String,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_time() -> String {
    "00:00:00".to_string()
}

fn default_true() -> bool {
    true
}

/// Update export schedule request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateExportScheduleRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ExportFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_of_week: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_of_month: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_of_day: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

/// List export jobs query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListExportJobsQuery {
    #[serde(default)]
    pub snapshot_id: Option<Id>,
    #[serde(default)]
    pub status: Option<ExportStatus>,
    #[serde(default)]
    pub format: Option<ExportFormat>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

/// Create export request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateExportRequest {
    pub project_id: Id,
    #[serde(default)]
    pub branch_id: Option<Id>,
    #[serde(default)]
    pub language_id: Option<Id>,
    pub format: ExportFormat,
    #[serde(default)]
    pub snapshot_id: Option<Id>,
}
