//! Git database queries

use chrono::{DateTime, Utc};
use cms_entity::git::{
    GitAuditEvent, GitConflict, GitConnection, GitFileState, GitPreview, GitProvider,
    GitPullRequest, GitSyncOperation, GitSyncOperationStatus, GitSyncOperationType,
    GitWebhookDelivery,
};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

// ============================================
// GitConnection
// ============================================

#[derive(Debug, FromRow)]
struct GitConnectionRow {
    id: String,
    project_id: String,
    provider: GitProvider,
    repository: String,
    branch: String,
    access_token: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<GitConnectionRow> for GitConnection {
    fn from(row: GitConnectionRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            provider: row.provider,
            repository: row.repository,
            branch: row.branch,
            access_token: row.access_token,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// GitConnection queries
pub struct GitConnectionQueries;

impl GitConnectionQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        connection_id: &str,
    ) -> Result<Option<GitConnection>, AppError> {
        let row =
            sqlx::query_as::<_, GitConnectionRow>("SELECT * FROM \"GitConnection\" WHERE id = $1")
                .bind(connection_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_project(
        pool: &PgPool,
        project_id: &str,
    ) -> Result<Option<GitConnection>, AppError> {
        let row = sqlx::query_as::<_, GitConnectionRow>(
            "SELECT * FROM \"GitConnection\" WHERE project_id = $1 LIMIT 1",
        )
        .bind(project_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn create(
        pool: &PgPool,
        project_id: &str,
        provider: GitProvider,
        repository: &str,
        branch: &str,
        access_token: &str,
    ) -> Result<GitConnection, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, GitConnectionRow>(
            r#"
            INSERT INTO "GitConnection" (id, project_id, provider, repository, branch, access_token, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(project_id)
        .bind(provider)
        .bind(repository)
        .bind(branch)
        .bind(access_token)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn update(
        pool: &PgPool,
        connection_id: &str,
        repository: Option<&str>,
        branch: Option<&str>,
        access_token: Option<&str>,
    ) -> Result<GitConnection, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"GitConnection\" SET ");

        let mut has_updates = false;
        if let Some(repository) = repository {
            query_builder.push("repository = ");
            query_builder.push_bind(repository);
            has_updates = true;
        }
        if let Some(branch) = branch {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("branch = ");
            query_builder.push_bind(branch);
            has_updates = true;
        }
        if let Some(access_token) = access_token {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("access_token = ");
            query_builder.push_bind(access_token);
            has_updates = true;
        }

        if has_updates {
            query_builder.push(", updated_at = ");
            query_builder.push_bind(Utc::now());
        }

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(connection_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<GitConnectionRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn delete(pool: &PgPool, connection_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"GitConnection\" WHERE id = $1")
            .bind(connection_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}

// ============================================
// GitSyncOperation
// ============================================

#[derive(Debug, FromRow)]
struct GitSyncOperationRow {
    id: String,
    connection_id: String,
    operation_type: GitSyncOperationType,
    status: GitSyncOperationStatus,
    commit_hash: Option<String>,
    error_message: Option<String>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<GitSyncOperationRow> for GitSyncOperation {
    fn from(row: GitSyncOperationRow) -> Self {
        Self {
            id: row.id,
            connection_id: row.connection_id,
            operation_type: row.operation_type,
            status: row.status,
            commit_hash: row.commit_hash,
            error_message: row.error_message,
            started_at: row.started_at,
            completed_at: row.completed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// GitSyncOperation queries
pub struct GitSyncOperationQueries;

impl GitSyncOperationQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        operation_id: &str,
    ) -> Result<Option<GitSyncOperation>, AppError> {
        let row = sqlx::query_as::<_, GitSyncOperationRow>(
            "SELECT * FROM \"GitSyncOperation\" WHERE id = $1",
        )
        .bind(operation_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_connection(
        pool: &PgPool,
        connection_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<GitSyncOperation>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"GitSyncOperation\" WHERE connection_id = ");
        query_builder.push_bind(connection_id);

        query_builder.push(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query_builder.push(" LIMIT ");
            query_builder.push_bind(limit);
        }

        if let Some(offset) = offset {
            query_builder.push(" OFFSET ");
            query_builder.push_bind(offset);
        }

        let rows = query_builder
            .build_query_as::<GitSyncOperationRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn count_by_connection(pool: &PgPool, connection_id: &str) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM \"GitSyncOperation\" WHERE connection_id = $1",
        )
        .bind(connection_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(count)
    }

    pub async fn create(
        pool: &PgPool,
        connection_id: &str,
        operation_type: GitSyncOperationType,
        status: GitSyncOperationStatus,
    ) -> Result<GitSyncOperation, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, GitSyncOperationRow>(
            r#"
            INSERT INTO "GitSyncOperation" (id, connection_id, operation_type, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(connection_id)
        .bind(operation_type)
        .bind(status)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn update_status(
        pool: &PgPool,
        operation_id: &str,
        status: GitSyncOperationStatus,
    ) -> Result<GitSyncOperation, AppError> {
        let row = sqlx::query_as::<_, GitSyncOperationRow>(
            "UPDATE \"GitSyncOperation\" SET status = $1, updated_at = $2 WHERE id = $3 RETURNING \
             *",
        )
        .bind(status)
        .bind(Utc::now())
        .bind(operation_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn update_error(
        pool: &PgPool,
        operation_id: &str,
        error_message: &str,
    ) -> Result<GitSyncOperation, AppError> {
        let row = sqlx::query_as::<_, GitSyncOperationRow>(
            "UPDATE \"GitSyncOperation\" SET error_message = $1, status = $2, updated_at = $3 \
             WHERE id = $4 RETURNING *",
        )
        .bind(error_message)
        .bind(GitSyncOperationStatus::Failed)
        .bind(Utc::now())
        .bind(operation_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn update_started(
        pool: &PgPool,
        operation_id: &str,
    ) -> Result<GitSyncOperation, AppError> {
        let row = sqlx::query_as::<_, GitSyncOperationRow>(
            "UPDATE \"GitSyncOperation\" SET started_at = $1, status = $2, updated_at = $3 WHERE \
             id = $4 RETURNING *",
        )
        .bind(Utc::now())
        .bind(GitSyncOperationStatus::Processing)
        .bind(Utc::now())
        .bind(operation_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn update_completed(
        pool: &PgPool,
        operation_id: &str,
    ) -> Result<GitSyncOperation, AppError> {
        let row = sqlx::query_as::<_, GitSyncOperationRow>(
            "UPDATE \"GitSyncOperation\" SET completed_at = $1, status = $2, updated_at = $3 \
             WHERE id = $4 RETURNING *",
        )
        .bind(Utc::now())
        .bind(GitSyncOperationStatus::Completed)
        .bind(Utc::now())
        .bind(operation_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }
}

// ============================================
// GitFileState
// ============================================

#[derive(Debug, FromRow)]
struct GitFileStateRow {
    id: String,
    project_id: String,
    path: String,
    git_path: String,
    last_commit_hash: Option<String>,
    last_sync_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<GitFileStateRow> for GitFileState {
    fn from(row: GitFileStateRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            path: row.path,
            git_path: row.git_path,
            last_commit_hash: row.last_commit_hash,
            last_sync_at: row.last_sync_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// GitFileState queries
pub struct GitFileStateQueries;

impl GitFileStateQueries {
    pub async fn get_by_path(
        pool: &PgPool,
        project_id: &str,
        path: &str,
    ) -> Result<Option<GitFileState>, AppError> {
        let row = sqlx::query_as::<_, GitFileStateRow>(
            "SELECT * FROM \"GitFileState\" WHERE project_id = $1 AND path = $2",
        )
        .bind(project_id)
        .bind(path)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_project(
        pool: &PgPool,
        project_id: &str,
    ) -> Result<Vec<GitFileState>, AppError> {
        let rows = sqlx::query_as::<_, GitFileStateRow>(
            "SELECT * FROM \"GitFileState\" WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn create(
        pool: &PgPool,
        project_id: &str,
        path: &str,
        git_path: &str,
    ) -> Result<GitFileState, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, GitFileStateRow>(
            r#"
            INSERT INTO "GitFileState" (id, project_id, path, git_path, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(path)
        .bind(git_path)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn update(
        pool: &PgPool,
        file_state_id: &str,
        commit_hash: Option<&str>,
    ) -> Result<GitFileState, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"GitFileState\" SET ");

        if let Some(commit_hash) = commit_hash {
            query_builder.push("last_commit_hash = ");
            query_builder.push_bind(commit_hash);
            query_builder.push(", last_sync_at = ");
            query_builder.push_bind(Utc::now());
        }

        query_builder.push(", updated_at = ");
        query_builder.push_bind(Utc::now());
        query_builder.push(" WHERE id = ");
        query_builder.push_bind(file_state_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<GitFileStateRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn delete(pool: &PgPool, file_state_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"GitFileState\" WHERE id = $1")
            .bind(file_state_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}

// ============================================
// Remaining Git tables (stubs for now)
// ============================================

#[derive(Debug, FromRow)]
struct GitConflictRow {
    id: String,
    project_id: String,
    file_path: String,
    conflict_type: String,
    our_content: String,
    their_content: String,
    resolved_content: Option<String>,
    resolved_at: Option<DateTime<Utc>>,
    resolved_by: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<GitConflictRow> for GitConflict {
    fn from(r: GitConflictRow) -> Self {
        Self {
            id: r.id,
            project_id: r.project_id,
            file_path: r.file_path,
            conflict_type: r.conflict_type,
            our_content: r.our_content,
            their_content: r.their_content,
            resolved_content: r.resolved_content,
            resolved_at: r.resolved_at,
            resolved_by: r.resolved_by,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct GitPullRequestRow {
    id: String,
    connection_id: String,
    pr_number: i32,
    title: String,
    state: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<GitPullRequestRow> for GitPullRequest {
    fn from(r: GitPullRequestRow) -> Self {
        Self {
            id: r.id,
            connection_id: r.connection_id,
            pr_number: r.pr_number,
            title: r.title,
            state: r.state,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct GitPreviewRow {
    id: String,
    pull_request_id: String,
    deployment_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<GitPreviewRow> for GitPreview {
    fn from(r: GitPreviewRow) -> Self {
        Self {
            id: r.id,
            pull_request_id: r.pull_request_id,
            deployment_id: r.deployment_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// GitConflict queries
pub struct GitConflictQueries;

impl GitConflictQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        conflict_id: &str,
    ) -> Result<Option<GitConflict>, AppError> {
        let row = sqlx::query_as::<_, GitConflictRow>(
            "SELECT * FROM \"GitConflict\" WHERE id = $1",
        )
        .bind(conflict_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_project(
        pool: &PgPool,
        project_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<GitConflict>, AppError> {
        let rows = sqlx::query_as::<_, GitConflictRow>(
            "SELECT * FROM \"GitConflict\" WHERE project_id = $1 ORDER BY created_at LIMIT $2 OFFSET $3",
        )
        .bind(project_id)
        .bind(limit.unwrap_or(100))
        .bind(offset.unwrap_or(0))
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn count_by_project(pool: &PgPool, project_id: &str) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM \"GitConflict\" WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(count)
    }

    pub async fn resolve(
        pool: &PgPool,
        conflict_id: &str,
        resolved_by: &str,
        resolved_content: &str,
    ) -> Result<GitConflict, AppError> {
        let row = sqlx::query_as::<_, GitConflictRow>(
            "UPDATE \"GitConflict\" SET resolved_content = $2, resolved_at = $3, resolved_by = $4, updated_at = $3 WHERE id = $1 RETURNING *",
        )
        .bind(conflict_id)
        .bind(resolved_content)
        .bind(Utc::now())
        .bind(resolved_by)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(row.into())
    }
}

/// GitPullRequest queries
pub struct GitPullRequestQueries;

impl GitPullRequestQueries {
    pub async fn get_by_id(pool: &PgPool, pr_id: &str) -> Result<Option<GitPullRequest>, AppError> {
        let row = sqlx::query_as::<_, GitPullRequestRow>(
            "SELECT * FROM \"GitPullRequest\" WHERE id = $1",
        )
        .bind(pr_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_connection(
        pool: &PgPool,
        connection_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<GitPullRequest>, AppError> {
        let rows = sqlx::query_as::<_, GitPullRequestRow>(
            "SELECT * FROM \"GitPullRequest\" WHERE connection_id = $1 ORDER BY created_at LIMIT $2 OFFSET $3",
        )
        .bind(connection_id)
        .bind(limit.unwrap_or(100))
        .bind(offset.unwrap_or(0))
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn count_by_connection(pool: &PgPool, connection_id: &str) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM \"GitPullRequest\" WHERE connection_id = $1",
        )
        .bind(connection_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(count)
    }
}

/// GitPreview queries
pub struct GitPreviewQueries;

impl GitPreviewQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        preview_id: &str,
    ) -> Result<Option<GitPreview>, AppError> {
        let row = sqlx::query_as::<_, GitPreviewRow>(
            "SELECT * FROM \"GitPreview\" WHERE id = $1",
        )
        .bind(preview_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_pull_request(
        pool: &PgPool,
        pr_id: &str,
    ) -> Result<Vec<GitPreview>, AppError> {
        let rows = sqlx::query_as::<_, GitPreviewRow>(
            "SELECT * FROM \"GitPreview\" WHERE pull_request_id = $1 ORDER BY created_at",
        )
        .bind(pr_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

/// GitWebhookDelivery queries
pub struct GitWebhookDeliveryQueries;

impl GitWebhookDeliveryQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        delivery_id: &str,
    ) -> Result<Option<GitWebhookDelivery>, AppError> {
        Err(AppError::NotFound(
            "GitWebhookDelivery queries not yet implemented".to_string(),
        ))
    }
}

/// GitAuditEvent queries
pub struct GitAuditEventQueries;

impl GitAuditEventQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        event_id: &str,
    ) -> Result<Option<GitAuditEvent>, AppError> {
        Err(AppError::NotFound(
            "GitAuditEvent queries not yet implemented".to_string(),
        ))
    }
}
