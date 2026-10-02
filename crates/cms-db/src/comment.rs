//! Comment database queries

use chrono::{DateTime, Utc};
use cms_entity::comment::Comment;
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

/// Database representation of a comment row
#[derive(Debug, FromRow)]
struct CommentRow {
    id: String,
    page_id: String,
    user_id: Option<String>,
    reader_id: Option<String>,
    parent_id: Option<String>,
    content: String,
    resolved: bool,
    resolved_at: Option<DateTime<Utc>>,
    resolved_by: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CommentRow> for Comment {
    fn from(row: CommentRow) -> Self {
        Self {
            id: row.id,
            page_id: row.page_id,
            user_id: row.user_id,
            reader_id: row.reader_id,
            parent_id: row.parent_id,
            content: row.content,
            resolved: row.resolved,
            resolved_at: row.resolved_at,
            resolved_by: row.resolved_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Comment queries
pub struct CommentQueries;

impl CommentQueries {
    /// Get a comment by ID
    pub async fn get_by_id(pool: &PgPool, comment_id: &str) -> Result<Option<Comment>, AppError> {
        let row = sqlx::query_as::<_, CommentRow>("SELECT * FROM \"Comment\" WHERE id = $1")
            .bind(comment_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get comments by page
    pub async fn get_by_page(
        pool: &PgPool,
        page_id: &str,
        parent_id: Option<&str>,
        resolved: Option<bool>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Comment>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"Comment\" WHERE page_id = ");
        query_builder.push_bind(page_id);

        if let Some(parent_id) = parent_id {
            if parent_id.is_empty() {
                query_builder.push(" AND parent_id IS NULL");
            } else {
                query_builder.push(" AND parent_id = ");
                query_builder.push_bind(parent_id);
            }
        }

        if let Some(resolved) = resolved {
            query_builder.push(" AND resolved = ");
            query_builder.push_bind(resolved);
        }

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
            .build_query_as::<CommentRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Get comments across a project (any page owned by the project)
    pub async fn get_by_project(
        pool: &PgPool,
        project_id: &str,
        parent_id: Option<&str>,
        resolved: Option<bool>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Comment>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT c.* FROM \"Comment\" c \
             JOIN \"Page\" p ON c.page_id = p.id WHERE p.project_id = ",
        );
        query_builder.push_bind(project_id);

        if let Some(parent_id) = parent_id {
            if parent_id.is_empty() {
                query_builder.push(" AND c.parent_id IS NULL");
            } else {
                query_builder.push(" AND c.parent_id = ");
                query_builder.push_bind(parent_id);
            }
        }

        if let Some(resolved) = resolved {
            query_builder.push(" AND c.resolved = ");
            query_builder.push_bind(resolved);
        }

        query_builder.push(" ORDER BY c.created_at DESC");

        if let Some(limit) = limit {
            query_builder.push(" LIMIT ");
            query_builder.push_bind(limit);
        }

        if let Some(offset) = offset {
            query_builder.push(" OFFSET ");
            query_builder.push_bind(offset);
        }

        let rows = query_builder
            .build_query_as::<CommentRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Count comments by page
    pub async fn count_by_page(
        pool: &PgPool,
        page_id: &str,
        parent_id: Option<&str>,
        resolved: Option<bool>,
    ) -> Result<i64, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM \"Comment\" WHERE page_id = ");
        query_builder.push_bind(page_id);

        if let Some(parent_id) = parent_id {
            if parent_id.is_empty() {
                query_builder.push(" AND parent_id IS NULL");
            } else {
                query_builder.push(" AND parent_id = ");
                query_builder.push_bind(parent_id);
            }
        }

        if let Some(resolved) = resolved {
            query_builder.push(" AND resolved = ");
            query_builder.push_bind(resolved);
        }

        let count: i64 = query_builder
            .build()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?
            .get::<i64, _>(0);

        Ok(count)
    }

    /// Create a new comment
    pub async fn create(
        pool: &PgPool,
        page_id: &str,
        user_id: Option<&str>,
        reader_id: Option<&str>,
        parent_id: Option<&str>,
        content: &str,
    ) -> Result<Comment, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, CommentRow>(
            r#"
            INSERT INTO "Comment" (id, page_id, user_id, reader_id, parent_id, content, resolved, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(page_id)
        .bind(user_id)
        .bind(reader_id)
        .bind(parent_id)
        .bind(content)
        .bind(false)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update a comment
    pub async fn update(
        pool: &PgPool,
        comment_id: &str,
        content: Option<&str>,
        resolved: Option<bool>,
    ) -> Result<Comment, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"Comment\" SET ");

        let mut has_updates = false;
        if let Some(content) = content {
            query_builder.push("content = ");
            query_builder.push_bind(content);
            has_updates = true;
        }
        if let Some(resolved) = resolved {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("resolved = ");
            query_builder.push_bind(resolved);

            if resolved {
                query_builder.push(", resolved_at = ");
                query_builder.push_bind(Utc::now());
            } else {
                query_builder.push(", resolved_at = NULL");
            }
            has_updates = true;
        }

        if has_updates {
            query_builder.push(", updated_at = ");
            query_builder.push_bind(Utc::now());
        }

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(comment_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<CommentRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Delete a comment
    pub async fn delete(pool: &PgPool, comment_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"Comment\" WHERE id = $1")
            .bind(comment_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Get replies for a comment
    pub async fn get_replies(
        pool: &PgPool,
        parent_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Comment>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"Comment\" WHERE parent_id = ");
        query_builder.push_bind(parent_id);

        query_builder.push(" ORDER BY created_at ASC");

        if let Some(limit) = limit {
            query_builder.push(" LIMIT ");
            query_builder.push_bind(limit);
        }

        if let Some(offset) = offset {
            query_builder.push(" OFFSET ");
            query_builder.push_bind(offset);
        }

        let rows = query_builder
            .build_query_as::<CommentRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}
