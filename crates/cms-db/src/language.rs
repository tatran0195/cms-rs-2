//! Language database queries

use chrono::{DateTime, Utc};
use cms_entity::language::{
    Language, LanguageResponse, ProjectTranslation, ProjectTranslationResponse,
};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

/// Database representation of a language row.
#[derive(Debug, FromRow)]
struct LanguageRow {
    id: String,
    project_id: String,
    code: String,
    name: String,
    is_default: bool,
    is_rtl: bool,
    enabled: bool,
    position: i32,
    config: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Database representation of a project translation row.
#[derive(Debug, FromRow)]
struct ProjectTranslationRow {
    id: String,
    project_id: String,
    language_id: String,
    name: Option<String>,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<LanguageRow> for Language {
    fn from(row: LanguageRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            code: row.code,
            name: row.name,
            is_default: row.is_default,
            is_rtl: row.is_rtl,
            enabled: row.enabled,
            position: row.position,
            config: row.config,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<LanguageRow> for LanguageResponse {
    fn from(row: LanguageRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            code: row.code,
            name: row.name,
            is_default: row.is_default,
            is_rtl: row.is_rtl,
            enabled: row.enabled,
            position: row.position,
            config: row.config,
            translation: None,
            coverage: None,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<ProjectTranslationRow> for ProjectTranslation {
    fn from(row: ProjectTranslationRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            language_id: row.language_id,
            name: row.name,
            description: row.description,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<ProjectTranslationRow> for ProjectTranslationResponse {
    fn from(row: ProjectTranslationRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            language_id: row.language_id,
            name: row.name,
            description: row.description,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Language queries.
pub struct LanguageQueries;

impl LanguageQueries {
    /// Get a language by ID.
    pub async fn get_by_id(pool: &PgPool, language_id: &str) -> Result<Option<Language>, AppError> {
        let row = sqlx::query_as::<_, LanguageRow>("SELECT * FROM \"Language\" WHERE id = $1")
            .bind(language_id)
            .fetch_optional(pool)
            .await?;
        Ok(row.map(Into::into))
    }

    /// Get a language by BCP-47 tag, case-insensitively.
    pub async fn get_by_code(
        pool: &PgPool,
        project_id: &str,
        code: &str,
    ) -> Result<Option<Language>, AppError> {
        let row = sqlx::query_as::<_, LanguageRow>(
            "SELECT * FROM \"Language\" WHERE project_id = $1 AND lower(code) = lower($2)",
        )
        .bind(project_id)
        .bind(code)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(Into::into))
    }

    /// Get the project's default language.
    pub async fn get_default(
        pool: &PgPool,
        project_id: &str,
    ) -> Result<Option<Language>, AppError> {
        let row = sqlx::query_as::<_, LanguageRow>(
            "SELECT * FROM \"Language\" WHERE project_id = $1 AND is_default = TRUE LIMIT 1",
        )
        .bind(project_id)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(Into::into))
    }

    /// List project languages in their configured UI order.
    pub async fn get_by_project(
        pool: &PgPool,
        project_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Language>, AppError> {
        let mut query =
            QueryBuilder::<Postgres>::new("SELECT * FROM \"Language\" WHERE project_id = ");
        query.push_bind(project_id);
        query.push(" ORDER BY is_default DESC, position ASC, created_at ASC, id ASC");
        if let Some(limit) = limit {
            query.push(" LIMIT ");
            query.push_bind(limit);
        }
        if let Some(offset) = offset {
            query.push(" OFFSET ");
            query.push_bind(offset);
        }
        let rows = query
            .build_query_as::<LanguageRow>()
            .fetch_all(pool)
            .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Count languages belonging to a project.
    pub async fn count_by_project(pool: &PgPool, project_id: &str) -> Result<i64, AppError> {
        Ok(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM \"Language\" WHERE project_id = $1")
                .bind(project_id)
                .fetch_one(pool)
                .await?,
        )
    }

    /// Backwards-compatible create helper. New business paths should use
    /// `create_with_metadata` to persist the full language settings.
    pub async fn create(
        pool: &PgPool,
        project_id: &str,
        code: &str,
        name: &str,
        is_default: bool,
        is_rtl: bool,
    ) -> Result<Language, AppError> {
        Self::create_with_metadata(
            pool, project_id, code, name, is_default, is_rtl, true, None, None,
        )
        .await
    }

    /// Create a language while serializing default/order decisions against the
    /// owning project. The first language is always made the default.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_with_metadata(
        pool: &PgPool,
        project_id: &str,
        code: &str,
        name: &str,
        is_default: bool,
        is_rtl: bool,
        enabled: bool,
        position: Option<i32>,
        config: Option<&serde_json::Value>,
    ) -> Result<Language, AppError> {
        let mut tx = pool.begin().await?;
        let project =
            sqlx::query_scalar::<_, String>("SELECT id FROM \"Project\" WHERE id = $1 FOR UPDATE")
                .bind(project_id)
                .fetch_optional(&mut *tx)
                .await?;
        if project.is_none() {
            return Err(AppError::NotFound("Project not found".to_string()));
        }

        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM \"Language\" WHERE project_id = $1")
                .bind(project_id)
                .fetch_one(&mut *tx)
                .await?;
        let make_default = is_default || count == 0;
        if make_default {
            sqlx::query("UPDATE \"Language\" SET is_default = FALSE WHERE project_id = $1")
                .bind(project_id)
                .execute(&mut *tx)
                .await?;
        }

        let next_position = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT MAX(position) FROM \"Language\" WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_one(&mut *tx)
        .await?
        .map_or(0, |max_position| max_position.saturating_add(1));

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let row = sqlx::query_as::<_, LanguageRow>(
            r#"
            INSERT INTO "Language" (
                id, project_id, code, name, is_default, is_rtl, enabled,
                position, config, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(code)
        .bind(name)
        .bind(make_default)
        .bind(is_rtl)
        .bind(if make_default { true } else { enabled })
        .bind(position.unwrap_or(next_position))
        .bind(config.cloned())
        .bind(now)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_language_write_error)?;

        tx.commit().await?;
        Ok(row.into())
    }

    /// Backwards-compatible metadata update helper.
    pub async fn update(
        pool: &PgPool,
        language_id: &str,
        name: Option<&str>,
        is_rtl: Option<bool>,
    ) -> Result<Language, AppError> {
        Self::update_with_metadata(pool, language_id, name, is_rtl, None, None, None, None).await
    }

    /// Atomically update language metadata/default selection. Project-row locking
    /// prevents concurrent default switches from leaving multiple defaults.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_with_metadata(
        pool: &PgPool,
        language_id: &str,
        name: Option<&str>,
        is_rtl: Option<bool>,
        is_default: Option<bool>,
        enabled: Option<bool>,
        position: Option<i32>,
        config: Option<Option<&serde_json::Value>>,
    ) -> Result<Language, AppError> {
        let mut tx = pool.begin().await?;
        let project_id =
            sqlx::query_scalar::<_, String>("SELECT project_id FROM \"Language\" WHERE id = $1")
                .bind(language_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| AppError::NotFound("Language not found".to_string()))?;

        let project =
            sqlx::query_scalar::<_, String>("SELECT id FROM \"Project\" WHERE id = $1 FOR UPDATE")
                .bind(&project_id)
                .fetch_optional(&mut *tx)
                .await?;
        if project.is_none() {
            return Err(AppError::NotFound("Project not found".to_string()));
        }

        let current = sqlx::query_as::<_, (bool, bool)>(
            "SELECT is_default, enabled FROM \"Language\" WHERE id = $1 FOR UPDATE",
        )
        .bind(language_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Language not found".to_string()))?;

        if is_default == Some(false) && current.0 {
            return Err(AppError::Conflict(
                "A project must always have a default language; choose another default first"
                    .to_string(),
            ));
        }
        if current.0 && enabled == Some(false) {
            return Err(AppError::Conflict(
                "The default language cannot be disabled".to_string(),
            ));
        }

        if is_default == Some(true) {
            sqlx::query("UPDATE \"Language\" SET is_default = FALSE WHERE project_id = $1")
                .bind(&project_id)
                .execute(&mut *tx)
                .await?;
        }

        let mut query = QueryBuilder::<Postgres>::new("UPDATE \"Language\" SET ");
        let mut has_updates = false;
        if let Some(name) = name {
            query.push("name = ");
            query.push_bind(name);
            has_updates = true;
        }
        if let Some(is_rtl) = is_rtl {
            push_comma(&mut query, &mut has_updates);
            query.push("is_rtl = ");
            query.push_bind(is_rtl);
        }
        if let Some(is_default) = is_default {
            push_comma(&mut query, &mut has_updates);
            query.push("is_default = ");
            query.push_bind(is_default);
        }
        let effective_enabled = if is_default == Some(true) {
            Some(true)
        } else {
            enabled
        };
        if let Some(enabled) = effective_enabled {
            push_comma(&mut query, &mut has_updates);
            query.push("enabled = ");
            query.push_bind(enabled);
        }
        if let Some(position) = position {
            push_comma(&mut query, &mut has_updates);
            query.push("position = ");
            query.push_bind(position);
        }
        if let Some(config) = config {
            push_comma(&mut query, &mut has_updates);
            query.push("config = ");
            query.push_bind(config.cloned());
        }
        push_comma(&mut query, &mut has_updates);
        query.push("updated_at = ");
        query.push_bind(Utc::now());
        query.push(" WHERE id = ");
        query.push_bind(language_id);
        query.push(" RETURNING *");

        let row = query
            .build_query_as::<LanguageRow>()
            .fetch_one(&mut *tx)
            .await
            .map_err(map_language_write_error)?;
        tx.commit().await?;
        Ok(row.into())
    }

    /// Delete a non-default language. The project lock serializes deletion with
    /// language creation/default changes so a project can never lose its final
    /// language or be left without a default. Pages/translations cascade by schema.
    pub async fn delete(pool: &PgPool, language_id: &str) -> Result<bool, AppError> {
        let mut tx = pool.begin().await?;
        let Some(project_id) =
            sqlx::query_scalar::<_, String>("SELECT project_id FROM \"Language\" WHERE id = $1")
                .bind(language_id)
                .fetch_optional(&mut *tx)
                .await?
        else {
            return Ok(false);
        };

        let project =
            sqlx::query_scalar::<_, String>("SELECT id FROM \"Project\" WHERE id = $1 FOR UPDATE")
                .bind(&project_id)
                .fetch_optional(&mut *tx)
                .await?;
        if project.is_none() {
            return Err(AppError::NotFound("Project not found".to_string()));
        }

        let current = sqlx::query_as::<_, (bool,)>(
            "SELECT is_default FROM \"Language\" WHERE id = $1 FOR UPDATE",
        )
        .bind(language_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((is_default,)) = current else {
            return Ok(false);
        };
        if is_default {
            return Err(AppError::Conflict(
                "Cannot delete the default language".to_string(),
            ));
        }

        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM \"Language\" WHERE project_id = $1")
                .bind(&project_id)
                .fetch_one(&mut *tx)
                .await?;
        if count <= 1 {
            return Err(AppError::Conflict(
                "A project must have at least one language".to_string(),
            ));
        }

        let result = sqlx::query("DELETE FROM \"Language\" WHERE id = $1")
            .bind(language_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(result.rows_affected() > 0)
    }

    /// Set a language as the project's default.
    pub async fn set_default(pool: &PgPool, language_id: &str) -> Result<Language, AppError> {
        Self::update_with_metadata(
            pool,
            language_id,
            None,
            None,
            Some(true),
            Some(true),
            None,
            None,
        )
        .await
    }
}

fn push_comma(query: &mut QueryBuilder<Postgres>, has_updates: &mut bool) {
    if *has_updates {
        query.push(", ");
    }
    *has_updates = true;
}

fn map_language_write_error(error: sqlx::Error) -> AppError {
    match &error {
        sqlx::Error::Database(db_error) if db_error.code().as_deref() == Some("23505") => {
            AppError::Conflict(
                "Language with this code already exists for this project".to_string(),
            )
        }
        _ => AppError::Database(error),
    }
}

/// Project translation queries.
pub struct ProjectTranslationQueries;

impl ProjectTranslationQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        translation_id: &str,
    ) -> Result<Option<ProjectTranslation>, AppError> {
        let row = sqlx::query_as::<_, ProjectTranslationRow>(
            "SELECT * FROM \"ProjectTranslation\" WHERE id = $1",
        )
        .bind(translation_id)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn get_by_project(
        pool: &PgPool,
        project_id: &str,
    ) -> Result<Vec<ProjectTranslation>, AppError> {
        let rows = sqlx::query_as::<_, ProjectTranslationRow>(
            "SELECT * FROM \"ProjectTranslation\" WHERE project_id = $1 ORDER BY created_at ASC, id ASC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_by_language(
        pool: &PgPool,
        language_id: &str,
    ) -> Result<Option<ProjectTranslation>, AppError> {
        let row = sqlx::query_as::<_, ProjectTranslationRow>(
            "SELECT * FROM \"ProjectTranslation\" WHERE language_id = $1",
        )
        .bind(language_id)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn count_by_language(pool: &PgPool, language_id: &str) -> Result<i64, AppError> {
        Ok(sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM \"ProjectTranslation\" WHERE language_id = $1",
        )
        .bind(language_id)
        .fetch_one(pool)
        .await?)
    }

    /// Atomic upsert avoids duplicate-create races. `None` retains an existing
    /// value, matching PATCH semantics; explicit clearing can be added through a
    /// dedicated tri-state request if the UI needs it.
    pub async fn upsert(
        pool: &PgPool,
        project_id: &str,
        language_id: &str,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<ProjectTranslation, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let row = sqlx::query_as::<_, ProjectTranslationRow>(
            r#"
            INSERT INTO "ProjectTranslation" (
                id, project_id, language_id, name, description, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (project_id, language_id) DO UPDATE SET
                name = COALESCE(EXCLUDED.name, "ProjectTranslation".name),
                description = COALESCE(EXCLUDED.description, "ProjectTranslation".description),
                updated_at = EXCLUDED.updated_at
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(language_id)
        .bind(name)
        .bind(description)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?;
        Ok(row.into())
    }

    pub async fn delete(pool: &PgPool, translation_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"ProjectTranslation\" WHERE id = $1")
            .bind(translation_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
