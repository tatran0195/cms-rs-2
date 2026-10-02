//! Branch database queries

use chrono::{DateTime, Utc};
use cms_entity::{
    branch::{Branch, BranchResponse},
    common::Id,
};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

/// Database representation of a branch row
#[derive(Debug, FromRow)]
struct BranchRow {
    id: String,
    project_id: String,
    name: String,
    slug: String,
    description: Option<String>,
    is_default: bool,
    is_protected: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<BranchRow> for Branch {
    fn from(row: BranchRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            slug: row.slug,
            description: row.description,
            is_default: row.is_default,
            is_protected: row.is_protected,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<BranchRow> for BranchResponse {
    fn from(row: BranchRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            slug: row.slug,
            description: row.description,
            is_default: row.is_default,
            is_protected: row.is_protected,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Branch queries
pub struct BranchQueries;

impl BranchQueries {
    /// Get a branch by ID
    pub async fn get_by_id(pool: &PgPool, branch_id: &str) -> Result<Option<Branch>, AppError> {
        let row = sqlx::query_as::<_, BranchRow>("SELECT * FROM \"Branch\" WHERE id = $1")
            .bind(branch_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get branches by project
    pub async fn get_by_project(
        pool: &PgPool,
        project_id: &str,
        search: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Branch>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"Branch\" WHERE project_id = ");
        query_builder.push_bind(project_id);

        if let Some(search) = search {
            query_builder.push(" AND (name ILIKE ");
            query_builder.push_bind(format!("%{}%", search));
            query_builder.push(" OR description ILIKE ");
            query_builder.push_bind(format!("%{}%", search));
            query_builder.push(")");
        }

        query_builder.push(" ORDER BY is_default DESC, created_at DESC");

        if let Some(limit) = limit {
            query_builder.push(" LIMIT ");
            query_builder.push_bind(limit);
        }

        if let Some(offset) = offset {
            query_builder.push(" OFFSET ");
            query_builder.push_bind(offset);
        }

        let rows = query_builder
            .build_query_as::<BranchRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Get the default branch for a project
    pub async fn get_default(pool: &PgPool, project_id: &str) -> Result<Option<Branch>, AppError> {
        let row = sqlx::query_as::<_, BranchRow>(
            "SELECT * FROM \"Branch\" WHERE project_id = $1 AND is_default = true LIMIT 1",
        )
        .bind(project_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get a branch by project and name
    pub async fn get_by_project_and_name(
        pool: &PgPool,
        project_id: &str,
        name: &str,
    ) -> Result<Option<Branch>, AppError> {
        let row = sqlx::query_as::<_, BranchRow>(
            "SELECT * FROM \"Branch\" WHERE project_id = $1 AND name = $2",
        )
        .bind(project_id)
        .bind(name)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get a branch by project and slug (the stable route identifier).
    pub async fn get_by_project_and_slug(
        pool: &PgPool,
        project_id: &str,
        slug: &str,
    ) -> Result<Option<Branch>, AppError> {
        let row = sqlx::query_as::<_, BranchRow>(
            "SELECT * FROM \"Branch\" WHERE project_id = $1 AND slug = $2",
        )
        .bind(project_id)
        .bind(slug)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(Into::into))
    }

    /// Create a new branch
    pub async fn create(
        pool: &PgPool,
        project_id: &str,
        name: &str,
        description: Option<&str>,
        is_default: bool,
        is_protected: bool,
    ) -> Result<Branch, AppError> {
        let id = Uuid::new_v4().to_string();
        let slug = name.to_lowercase().replace(' ', "-");
        let now = Utc::now();

        // If this is the default branch, make sure no other branch is default
        if is_default {
            sqlx::query("UPDATE \"Branch\" SET is_default = false WHERE project_id = $1")
                .bind(project_id)
                .execute(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;
        }

        let row = sqlx::query_as::<_, BranchRow>(
            r#"
            INSERT INTO "Branch" (id, project_id, name, slug, description, is_default, is_protected, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(project_id)
        .bind(name)
        .bind(&slug)
        .bind(description)
        .bind(is_default)
        .bind(is_protected)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                AppError::Conflict("Branch with this name already exists in this project".to_string())
            } else {
                AppError::Database(e.into())
            }
        })?;

        Ok(row.into())
    }

    /// Update a branch
    pub async fn update(
        pool: &PgPool,
        branch_id: &str,
        name: Option<&str>,
        description: Option<&str>,
        is_default: Option<bool>,
        is_public: Option<bool>,
    ) -> Result<Branch, AppError> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("UPDATE \"Branch\" SET ");

        let mut has_updates = false;
        if let Some(name) = name {
            query_builder.push("name = ");
            query_builder.push_bind(name);
            has_updates = true;
        }
        if let Some(description) = description {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("description = ");
            query_builder.push_bind(description);
            has_updates = true;
        }
        if let Some(is_default) = is_default {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("is_default = ");
            query_builder.push_bind(is_default);
            has_updates = true;
        }
        if let Some(is_public) = is_public {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("is_public = ");
            query_builder.push_bind(is_public);
            has_updates = true;
        }

        if has_updates {
            query_builder.push(", updated_at = ");
            query_builder.push_bind(Utc::now());
        }

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(branch_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<BranchRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Delete a branch
    pub async fn delete(pool: &PgPool, branch_id: &str) -> Result<bool, AppError> {
        // Check if branch has pages before deleting
        let page_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM \"Page\" WHERE branch_id = $1")
                .bind(branch_id)
                .fetch_one(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        if page_count > 0 {
            return Err(AppError::Conflict(
                "Cannot delete branch with pages".to_string(),
            ));
        }

        let result = sqlx::query("DELETE FROM \"Branch\" WHERE id = $1")
            .bind(branch_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Count branches by project
    pub async fn count_by_project(
        pool: &PgPool,
        project_id: &str,
        search: Option<&str>,
    ) -> Result<i64, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) as count FROM \"Branch\" WHERE project_id = ");
        query_builder.push_bind(project_id);

        if let Some(search) = search {
            query_builder.push(" AND (name ILIKE ");
            query_builder.push_bind(format!("%{}%", search));
            query_builder.push(" OR description ILIKE ");
            query_builder.push_bind(format!("%{}%", search));
            query_builder.push(")");
        }

        let count: i64 = query_builder
            .build()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?
            .get::<i64, _>("count");

        Ok(count)
    }

    /// Check if a branch name is available in a project
    pub async fn is_name_available(
        pool: &PgPool,
        project_id: &str,
        name: &str,
        exclude_branch_id: Option<&str>,
    ) -> Result<bool, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) as count FROM \"Branch\" WHERE project_id = ");
        query_builder.push_bind(project_id);
        query_builder.push(" AND name = ");
        query_builder.push_bind(name);

        if let Some(exclude_id) = exclude_branch_id {
            query_builder.push(" AND id != ");
            query_builder.push_bind(exclude_id);
        }

        let count: i64 = query_builder
            .build()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?
            .get::<i64, _>("count");

        Ok(count == 0)
    }

    /// Check if a branch slug (name) is available — alias for is_name_available
    pub async fn is_slug_available(
        pool: &PgPool,
        project_id: &str,
        slug: &str,
        exclude_branch_id: Option<&str>,
    ) -> Result<bool, AppError> {
        Self::is_name_available(pool, project_id, slug, exclude_branch_id).await
    }

    /// Get a branch by its slug (name)
    pub async fn get_by_slug(
        pool: &PgPool,
        project_id: &str,
        slug: &str,
    ) -> Result<Option<Branch>, AppError> {
        Self::get_by_project_and_name(pool, project_id, slug).await
    }

    /// Set a branch as the default branch for a project
    pub async fn set_default(
        pool: &PgPool,
        branch_id: &str,
        project_id: &str,
    ) -> Result<Branch, AppError> {
        let now = Utc::now();

        // First, unset any existing default
        sqlx::query(
            "UPDATE \"Branch\" SET is_default = false, updated_at = $1 WHERE project_id = $2",
        )
        .bind(now)
        .bind(project_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        // Then set the new default
        let row = sqlx::query_as::<_, BranchRow>(
            "UPDATE \"Branch\" SET is_default = true, updated_at = $1 WHERE id = $2 RETURNING *",
        )
        .bind(now)
        .bind(branch_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }
}
