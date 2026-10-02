//! OpenAPI database queries

use chrono::{DateTime, Utc};
use cms_entity::openapi::{OpenApiDocument, OpenApiDocumentResponse};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

// ============================================
// OpenApiDocument
// ============================================

#[derive(Debug, FromRow)]
struct OpenApiDocumentRow {
    id: String,
    project_id: String,
    name: String,
    url: String,
    content: Option<String>,
    parsed_at: Option<DateTime<Utc>>,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<OpenApiDocumentRow> for OpenApiDocument {
    fn from(row: OpenApiDocumentRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            url: row.url,
            content: row.content,
            parsed_at: row.parsed_at,
            error_message: row.error_message,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<OpenApiDocumentRow> for OpenApiDocumentResponse {
    fn from(row: OpenApiDocumentRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            url: row.url,
            parsed_at: row.parsed_at,
            has_error: row.error_message.is_some(),
            error_message: row.error_message,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// OpenApiDocument queries
pub struct OpenApiDocumentQueries;

impl OpenApiDocumentQueries {
    /// Get OpenAPI document by ID
    pub async fn get_by_id(
        pool: &PgPool,
        document_id: &str,
    ) -> Result<Option<OpenApiDocument>, AppError> {
        let row = sqlx::query_as::<_, OpenApiDocumentRow>(
            "SELECT * FROM \"OpenApiDocument\" WHERE id = $1",
        )
        .bind(document_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get OpenAPI document by URL and project ID
    pub async fn get_by_url(
        pool: &PgPool,
        url: &str,
        project_id: &str,
    ) -> Result<Option<OpenApiDocument>, AppError> {
        let row = sqlx::query_as::<_, OpenApiDocumentRow>(
            "SELECT * FROM \"OpenApiDocument\" WHERE url = $1 AND project_id = $2",
        )
        .bind(url)
        .bind(project_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get OpenAPI documents by project ID
    pub async fn get_by_project(
        pool: &PgPool,
        project_id: &str,
    ) -> Result<Vec<OpenApiDocument>, AppError> {
        let rows = sqlx::query_as::<_, OpenApiDocumentRow>(
            "SELECT * FROM \"OpenApiDocument\" WHERE project_id = $1 ORDER BY created_at ASC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Create a new OpenAPI document
    pub async fn create(
        pool: &PgPool,
        project_id: &str,
        name: &str,
        url: &str,
    ) -> Result<OpenApiDocument, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, OpenApiDocumentRow>(
            r#"
            INSERT INTO "OpenApiDocument" (id, project_id, name, url, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(name)
        .bind(url)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update an OpenAPI document
    pub async fn update(
        pool: &PgPool,
        document_id: &str,
        name: Option<&str>,
        url: Option<&str>,
    ) -> Result<OpenApiDocument, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"OpenApiDocument\" SET ");

        let mut has_updates = false;
        if let Some(name) = name {
            query_builder.push("name = ");
            query_builder.push_bind(name);
            has_updates = true;
        }
        if let Some(url) = url {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("url = ");
            query_builder.push_bind(url);
            has_updates = true;
        }

        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("updated_at = ");
        query_builder.push_bind(Utc::now());

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(document_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<OpenApiDocumentRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update OpenAPI document with parsed content
    pub async fn update_parsed(
        pool: &PgPool,
        document_id: &str,
        content: Option<&str>,
        parsed_at: Option<DateTime<Utc>>,
        error_message: Option<&str>,
    ) -> Result<OpenApiDocument, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"OpenApiDocument\" SET ");

        let mut has_updates = false;
        if let Some(content) = content {
            query_builder.push("content = ");
            query_builder.push_bind(content);
            has_updates = true;
        }
        if let Some(parsed_at) = parsed_at {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("parsed_at = ");
            query_builder.push_bind(parsed_at);
            has_updates = true;
        }
        if let Some(error_message) = error_message {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("error_message = ");
            query_builder.push_bind(error_message);
            has_updates = true;
        }

        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("updated_at = ");
        query_builder.push_bind(Utc::now());

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(document_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<OpenApiDocumentRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update OpenAPI document error
    pub async fn update_error(
        pool: &PgPool,
        document_id: &str,
        error_message: Option<&str>,
    ) -> Result<OpenApiDocument, AppError> {
        let row = sqlx::query_as::<_, OpenApiDocumentRow>(
            "UPDATE \"OpenApiDocument\" SET error_message = $1, updated_at = $2 WHERE id = $3 \
             RETURNING *",
        )
        .bind(error_message)
        .bind(Utc::now())
        .bind(document_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Delete an OpenAPI document
    pub async fn delete(pool: &PgPool, document_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"OpenApiDocument\" WHERE id = $1")
            .bind(document_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}
