//! Search index database queries

use chrono::{DateTime, Utc};
use cms_entity::search::{SearchIndexRun, SearchIndexRunStatus};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

// ============================================
// SearchIndexRun
// ============================================

#[derive(Debug, FromRow)]
struct SearchIndexRunRow {
    id: String,
    project_id: String,
    branch_id: Option<String>,
    language_id: Option<String>,
    status: SearchIndexRunStatus,
    pages_indexed: i32,
    error_message: Option<String>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SearchIndexRunRow> for SearchIndexRun {
    fn from(row: SearchIndexRunRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            branch_id: row.branch_id,
            language_id: row.language_id,
            status: row.status,
            pages_indexed: row.pages_indexed,
            error_message: row.error_message,
            started_at: row.started_at,
            completed_at: row.completed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// SearchIndexRun queries
pub struct SearchIndexRunQueries;

impl SearchIndexRunQueries {
    /// Get search index run by ID
    pub async fn get_by_id(
        pool: &PgPool,
        run_id: &str,
    ) -> Result<Option<SearchIndexRun>, AppError> {
        let row = sqlx::query_as::<_, SearchIndexRunRow>(
            "SELECT * FROM \"SearchIndexRun\" WHERE id = $1",
        )
        .bind(run_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get search index runs by project ID
    pub async fn get_by_project(
        pool: &PgPool,
        project_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<SearchIndexRun>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"SearchIndexRun\" WHERE project_id = ");
        query_builder.push_bind(project_id);

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
            .build_query_as::<SearchIndexRunRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Get the latest search index run for a project
    pub async fn get_latest_by_project(
        pool: &PgPool,
        project_id: &str,
    ) -> Result<Option<SearchIndexRun>, AppError> {
        let row = sqlx::query_as::<_, SearchIndexRunRow>(
            "SELECT * FROM \"SearchIndexRun\" WHERE project_id = $1 ORDER BY created_at DESC \
             LIMIT 1",
        )
        .bind(project_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Create a new search index run
    pub async fn create(
        pool: &PgPool,
        project_id: &str,
        branch_id: Option<&str>,
        language_id: Option<&str>,
        status: SearchIndexRunStatus,
    ) -> Result<SearchIndexRun, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, SearchIndexRunRow>(
            r#"
            INSERT INTO "SearchIndexRun" (id, project_id, branch_id, language_id, status, pages_indexed, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(project_id)
        .bind(branch_id)
        .bind(language_id)
        .bind(status)
        .bind(0)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update search index run status
    pub async fn update_status(
        pool: &PgPool,
        run_id: &str,
        status: SearchIndexRunStatus,
        error_message: Option<&str>,
    ) -> Result<SearchIndexRun, AppError> {
        let row = sqlx::query_as::<_, SearchIndexRunRow>(
            "UPDATE \"SearchIndexRun\" SET status = $1, error_message = $2, updated_at = $3 WHERE \
             id = $4 RETURNING *",
        )
        .bind(status)
        .bind(error_message)
        .bind(Utc::now())
        .bind(run_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update search index run progress
    pub async fn update_progress(
        pool: &PgPool,
        run_id: &str,
        pages_indexed: i32,
        started_at: Option<DateTime<Utc>>,
    ) -> Result<SearchIndexRun, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"SearchIndexRun\" SET pages_indexed = ");
        query_builder.push_bind(pages_indexed);

        if let Some(started_at) = started_at {
            query_builder.push(", started_at = ");
            query_builder.push_bind(started_at);
        }

        query_builder.push(", updated_at = ");
        query_builder.push_bind(Utc::now());
        query_builder.push(" WHERE id = ");
        query_builder.push_bind(run_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<SearchIndexRunRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Mark search index run as completed
    pub async fn mark_completed(
        pool: &PgPool,
        run_id: &str,
        pages_indexed: i32,
    ) -> Result<SearchIndexRun, AppError> {
        let row = sqlx::query_as::<_, SearchIndexRunRow>(
            r#"
            UPDATE "SearchIndexRun" 
            SET status = $1, pages_indexed = $2, completed_at = $3, updated_at = $4 
            WHERE id = $5 RETURNING *
            "#,
        )
        .bind(SearchIndexRunStatus::Completed)
        .bind(pages_indexed)
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(run_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Mark a partial/failed search run and preserve the first few actionable
    /// errors for operators.
    pub async fn mark_failed(
        pool: &PgPool,
        run_id: &str,
        pages_indexed: i32,
        error_message: &str,
    ) -> Result<SearchIndexRun, AppError> {
        let now = Utc::now();
        let row = sqlx::query_as::<_, SearchIndexRunRow>(
            r#"UPDATE "SearchIndexRun"
               SET status = $1, pages_indexed = $2, error_message = $3,
                   completed_at = $4, updated_at = $4
               WHERE id = $5 RETURNING *"#,
        )
        .bind(SearchIndexRunStatus::Failed)
        .bind(pages_indexed)
        .bind(error_message)
        .bind(now)
        .bind(run_id)
        .fetch_one(pool)
        .await?;
        Ok(row.into())
    }

    /// Count search index runs by project ID
    pub async fn count_by_project(pool: &PgPool, project_id: &str) -> Result<i64, AppError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM \"SearchIndexRun\" WHERE project_id = $1")
                .bind(project_id)
                .fetch_one(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(count)
    }
}
