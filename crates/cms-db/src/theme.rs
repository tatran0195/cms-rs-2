//! Theme database queries

use chrono::{DateTime, Utc};
use cms_entity::theme::{Theme, ThemeResponse};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

/// Database representation of a theme row
#[derive(Debug, FromRow)]
struct ThemeRow {
    id: String,
    project_id: String,
    name: String,
    primary_color: String,
    secondary_color: String,
    background_color: String,
    text_color: String,
    font_family: Option<String>,
    logo_url: Option<String>,
    favicon_url: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ThemeRow> for Theme {
    fn from(row: ThemeRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            primary_color: row.primary_color,
            secondary_color: row.secondary_color,
            background_color: row.background_color,
            text_color: row.text_color,
            font_family: row.font_family,
            logo_url: row.logo_url,
            favicon_url: row.favicon_url,
            config: None,
            is_global: false,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<ThemeRow> for ThemeResponse {
    fn from(row: ThemeRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            primary_color: row.primary_color,
            secondary_color: row.secondary_color,
            background_color: row.background_color,
            text_color: row.text_color,
            font_family: row.font_family,
            logo_url: row.logo_url,
            favicon_url: row.favicon_url,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Theme queries
pub struct ThemeQueries;

impl ThemeQueries {
    /// Get a theme by ID
    pub async fn get_by_id(pool: &PgPool, theme_id: &str) -> Result<Option<Theme>, AppError> {
        let row = sqlx::query_as::<_, ThemeRow>("SELECT * FROM \"ProjectSettings\" WHERE id = $1")
            .bind(theme_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get themes by project
    pub async fn get_by_project(pool: &PgPool, project_id: &str) -> Result<Vec<Theme>, AppError> {
        // Note: In the actual schema, themes are stored in ProjectSettings
        // This is a simplified query
        let rows = sqlx::query_as::<_, ThemeRow>(
            "SELECT id, project_id, theme as name, '{}'::jsonb as config, false as is_global, \
             created_at, updated_at FROM \"ProjectSettings\" WHERE project_id = $1 AND theme IS \
             NOT NULL",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Create a new theme
    pub async fn create(
        pool: &PgPool,
        project_id: &str,
        name: &str,
        config: serde_json::Value,
        is_global: bool,
    ) -> Result<Theme, AppError> {
        // In the actual schema, themes are stored as part of ProjectSettings
        // This creates a dedicated theme record
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, ThemeRow>(
            r#"
            INSERT INTO "Theme" (id, project_id, name, config, is_global, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(name)
        .bind(config)
        .bind(is_global)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update a theme
    pub async fn update(
        pool: &PgPool,
        theme_id: &str,
        name: Option<&str>,
        config: Option<&serde_json::Value>,
        is_global: Option<bool>,
    ) -> Result<Theme, AppError> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("UPDATE \"Theme\" SET ");

        let mut has_updates = false;
        if let Some(name) = name {
            query_builder.push("name = ");
            query_builder.push_bind(name);
            has_updates = true;
        }
        if let Some(config) = config {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("config = ");
            query_builder.push_bind(config);
            has_updates = true;
        }
        if let Some(is_global) = is_global {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("is_global = ");
            query_builder.push_bind(is_global);
            has_updates = true;
        }

        if has_updates {
            query_builder.push(", updated_at = ");
            query_builder.push_bind(Utc::now());
        }

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(theme_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<ThemeRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Delete a theme
    pub async fn delete(pool: &PgPool, theme_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"Theme\" WHERE id = $1")
            .bind(theme_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}
