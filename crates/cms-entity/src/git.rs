//! Git entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::common::{Id, Timestamp};

/// Git provider types
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "\"GitProvider\"", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GitProvider {
    Github,
    Gitlab,
    Bitbucket,
    AzureDevops,
}

/// Git sync operation type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum GitSyncOperationType {
    Full,
    Incremental,
    Manual,
    // Uppercase aliases
    FULL,
    INCREMENTAL,
    MANUAL,
}

impl sqlx::Type<sqlx::Postgres> for GitSyncOperationType {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("\"GitSyncOperationType\"")
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for GitSyncOperationType {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        match s {
            "FULL" | "full" => Ok(GitSyncOperationType::Full),
            "INCREMENTAL" | "incremental" => Ok(GitSyncOperationType::Incremental),
            "MANUAL" | "manual" => Ok(GitSyncOperationType::Manual),
            other => Err(format!("unknown GitSyncOperationType: {}", other).into()),
        }
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Postgres> for GitSyncOperationType {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = match self {
            GitSyncOperationType::Full | GitSyncOperationType::FULL => "FULL",
            GitSyncOperationType::Incremental | GitSyncOperationType::INCREMENTAL => "INCREMENTAL",
            GitSyncOperationType::Manual | GitSyncOperationType::MANUAL => "MANUAL",
        };
        <&str as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&s, buf)
    }
}

/// Git sync operation status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "\"GitSyncOperationStatus\"", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GitSyncOperationStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Conflict,
}

/// Git connection entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GitConnection {
    pub id: Id,
    pub project_id: Id,
    pub provider: GitProvider,
    pub repository: String,
    pub branch: String,
    pub access_token: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Git connection response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GitConnectionResponse {
    pub id: Id,
    pub project_id: Id,
    pub provider: GitProvider,
    pub repository: String,
    pub branch: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<GitConnection> for GitConnectionResponse {
    fn from(conn: GitConnection) -> Self {
        Self {
            id: conn.id,
            project_id: conn.project_id,
            provider: conn.provider,
            repository: conn.repository,
            branch: conn.branch,
            created_at: conn.created_at,
            updated_at: conn.updated_at,
        }
    }
}

/// Git connection create request
#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
pub struct CreateGitConnectionRequest {
    #[validate(length(min = 1, message = "Project ID is required"))]
    pub project_id: String,
    pub provider: GitProvider,
    #[validate(length(min = 1, message = "Repository is required"))]
    pub repository: String,
    #[serde(default = "default_branch")]
    #[validate(length(min = 1, message = "Branch is required"))]
    pub branch: String,
    #[validate(length(min = 1, message = "Access token is required"))]
    pub access_token: String,
}

fn default_branch() -> String {
    "main".to_string()
}

/// Git connection update request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateGitConnectionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
}

/// Git sync operation entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GitSyncOperation {
    pub id: Id,
    pub connection_id: Id,
    pub operation_type: GitSyncOperationType,
    pub status: GitSyncOperationStatus,
    pub commit_hash: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Git sync operation response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GitSyncOperationResponse {
    pub id: Id,
    pub connection_id: Id,
    pub operation_type: GitSyncOperationType,
    pub status: GitSyncOperationStatus,
    pub commit_hash: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<GitSyncOperation> for GitSyncOperationResponse {
    fn from(op: GitSyncOperation) -> Self {
        Self {
            id: op.id,
            connection_id: op.connection_id,
            operation_type: op.operation_type,
            status: op.status,
            commit_hash: op.commit_hash,
            error_message: op.error_message,
            started_at: op.started_at,
            completed_at: op.completed_at,
            created_at: op.created_at,
            updated_at: op.updated_at,
        }
    }
}

/// Git file state entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GitFileState {
    pub id: Id,
    pub project_id: Id,
    pub path: String,
    pub git_path: String,
    pub last_commit_hash: Option<String>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Git file state response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GitFileStateResponse {
    pub id: Id,
    pub project_id: Id,
    pub path: String,
    pub git_path: String,
    pub last_commit_hash: Option<String>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<GitFileState> for GitFileStateResponse {
    fn from(state: GitFileState) -> Self {
        Self {
            id: state.id,
            project_id: state.project_id,
            path: state.path,
            git_path: state.git_path,
            last_commit_hash: state.last_commit_hash,
            last_sync_at: state.last_sync_at,
            created_at: state.created_at,
            updated_at: state.updated_at,
        }
    }
}

/// Git conflict entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GitConflict {
    pub id: Id,
    pub project_id: Id,
    pub file_path: String,
    pub conflict_type: String,
    pub our_content: String,
    pub their_content: String,
    pub resolved_content: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Id>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Git conflict response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GitConflictResponse {
    pub id: Id,
    pub project_id: Id,
    pub file_path: String,
    pub conflict_type: String,
    pub our_content: String,
    pub their_content: String,
    pub resolved_content: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Id>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<GitConflict> for GitConflictResponse {
    fn from(conflict: GitConflict) -> Self {
        Self {
            id: conflict.id,
            project_id: conflict.project_id,
            file_path: conflict.file_path,
            conflict_type: conflict.conflict_type,
            our_content: conflict.our_content,
            their_content: conflict.their_content,
            resolved_content: conflict.resolved_content,
            resolved_at: conflict.resolved_at,
            resolved_by: conflict.resolved_by,
            created_at: conflict.created_at,
            updated_at: conflict.updated_at,
        }
    }
}

/// Resolve conflict request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ResolveGitConflictRequest {
    pub resolved_content: String,
}

/// Git pull request entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GitPullRequest {
    pub id: Id,
    pub connection_id: Id,
    pub pr_number: i32,
    pub title: String,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Git pull request response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GitPullRequestResponse {
    pub id: Id,
    pub connection_id: Id,
    pub pr_number: i32,
    pub title: String,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<GitPullRequest> for GitPullRequestResponse {
    fn from(pr: GitPullRequest) -> Self {
        Self {
            id: pr.id,
            connection_id: pr.connection_id,
            pr_number: pr.pr_number,
            title: pr.title,
            state: pr.state,
            created_at: pr.created_at,
            updated_at: pr.updated_at,
        }
    }
}

/// Git preview entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GitPreview {
    pub id: Id,
    pub pull_request_id: Id,
    pub deployment_id: Option<Id>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Git preview response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GitPreviewResponse {
    pub id: Id,
    pub pull_request_id: Id,
    pub deployment_id: Option<Id>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<GitPreview> for GitPreviewResponse {
    fn from(preview: GitPreview) -> Self {
        Self {
            id: preview.id,
            pull_request_id: preview.pull_request_id,
            deployment_id: preview.deployment_id,
            created_at: preview.created_at,
            updated_at: preview.updated_at,
        }
    }
}

/// Git webhook delivery entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GitWebhookDelivery {
    pub id: Id,
    pub connection_id: Id,
    pub delivery_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Git audit event entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GitAuditEvent {
    pub id: Id,
    pub connection_id: Id,
    pub action: String,
    pub user_id: Option<Id>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// List git sync operations query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListGitSyncOperationsQuery {
    #[serde(default)]
    pub connection_id: Option<Id>,
    #[serde(default)]
    pub status: Option<GitSyncOperationStatus>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}
