//! Project database queries

use chrono::{DateTime, Utc};
use cms_entity::project::{
    Project, ProjectAddon, ProjectAddonResponse, ProjectResponse, ProjectSettings,
};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

/// Database representation of a project row
#[derive(Debug, FromRow)]
struct ProjectRow {
    id: String,
    organization_id: String,
    name: String,
    slug: String,
    description: Option<String>,
    icon: Option<String>,
    is_public: bool,
    config: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Database representation of a project settings row
#[derive(Debug, FromRow)]
struct ProjectSettingsRow {
    project_id: String,
    theme: Option<String>,
    default_language: Option<String>,
    custom_domain: Option<String>,
    search_enabled: bool,
    comments_enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Database representation of a project addon row
#[derive(Debug, FromRow)]
struct ProjectAddonRow {
    id: String,
    project_id: String,
    addon_type: String,
    config: serde_json::Value,
    is_enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ProjectRow> for Project {
    fn from(row: ProjectRow) -> Self {
        Self {
            id: row.id,
            organization_id: row.organization_id,
            name: row.name,
            slug: row.slug,
            description: row.description,
            icon: row.icon,
            is_public: row.is_public,
            config: row.config,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<ProjectAddonRow> for ProjectAddon {
    fn from(row: ProjectAddonRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            addon_type: row.addon_type,
            config: row.config,
            is_enabled: row.is_enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Project queries
pub struct ProjectQueries;

impl ProjectQueries {
    /// Get a project by ID
    pub async fn get_by_id(pool: &PgPool, project_id: &str) -> Result<Option<Project>, AppError> {
        let row = sqlx::query_as::<_, ProjectRow>("SELECT * FROM \"Project\" WHERE id = $1")
            .bind(project_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Check if a project has an active platform takedown.
    pub async fn is_taken_down(pool: &PgPool, project_id: &str) -> Result<Option<bool>, AppError> {
        sqlx::query_scalar::<_, bool>(
            r#"SELECT takedown_at IS NOT NULL FROM "Project" WHERE id = $1"#,
        )
        .bind(project_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| AppError::Database(error.into()))
    }

    /// Record a platform moderation takedown.
    pub async fn set_takedown(
        pool: &PgPool,
        project_id: &str,
        reason: &str,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"UPDATE "Project"
               SET takedown_at = NOW(), takedown_reason = $2, updated_at = NOW()
               WHERE id = $1"#,
        )
        .bind(project_id)
        .bind(reason)
        .execute(pool)
        .await
        .map_err(|error| AppError::Database(error.into()))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Project not found".to_string()));
        }
        Ok(())
    }

    /// Restore a project previously removed from public service.
    pub async fn restore_takedown(pool: &PgPool, project_id: &str) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"UPDATE "Project"
               SET takedown_at = NULL, takedown_reason = NULL, updated_at = NOW()
               WHERE id = $1"#,
        )
        .bind(project_id)
        .execute(pool)
        .await
        .map_err(|error| AppError::Database(error.into()))?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Project not found".to_string()));
        }
        Ok(())
    }

    /// Get a project by slug
    pub async fn get_by_slug(
        pool: &PgPool,
        organization_id: &str,
        slug: &str,
    ) -> Result<Option<Project>, AppError> {
        let row = sqlx::query_as::<_, ProjectRow>(
            "SELECT * FROM \"Project\" WHERE organization_id = $1 AND slug = $2",
        )
        .bind(organization_id)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get a project by slug across any organization
    pub async fn get_by_slug_global(
        pool: &PgPool,
        slug: &str,
    ) -> Result<Option<Project>, AppError> {
        let row =
            sqlx::query_as::<_, ProjectRow>("SELECT * FROM \"Project\" WHERE slug = $1 LIMIT 1")
                .bind(slug)
                .fetch_optional(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get projects by organization
    pub async fn get_by_organization(
        pool: &PgPool,
        organization_id: &str,
        is_public: Option<bool>,
        search: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Project>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"Project\" WHERE organization_id = ");
        query_builder.push_bind(organization_id);

        if let Some(is_public) = is_public {
            query_builder.push(" AND is_public = ");
            query_builder.push_bind(is_public);
        }

        if let Some(search) = search {
            query_builder.push(" AND (name ILIKE ");
            query_builder.push_bind(format!("%{}%", search));
            query_builder.push(" OR description ILIKE ");
            query_builder.push_bind(format!("%{}%", search));
            query_builder.push(")");
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
            .build_query_as::<ProjectRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Count projects by organization
    pub async fn count_by_organization(
        pool: &PgPool,
        organization_id: &str,
        is_public: Option<bool>,
        search: Option<&str>,
    ) -> Result<i64, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM \"Project\" WHERE organization_id = ");
        query_builder.push_bind(organization_id);

        if let Some(is_public) = is_public {
            query_builder.push(" AND is_public = ");
            query_builder.push_bind(is_public);
        }

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
            .get::<i64, _>(0);

        Ok(count)
    }

    /// List projects accessible by a user
    pub async fn list_by_user(pool: &PgPool, user_id: &str) -> Result<Vec<Project>, AppError> {
        let rows = sqlx::query_as::<_, ProjectRow>(
            r#"
            SELECT DISTINCT p.* FROM "Project" p
            LEFT JOIN "Member" m ON p.organization_id = m.organization_id AND m.user_id = $1
            WHERE p.is_public = true OR m.id IS NOT NULL
            ORDER BY p.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Create a new project
    pub async fn create(
        pool: &PgPool,
        organization_id: &str,
        name: &str,
        slug: &str,
        description: Option<&str>,
        icon: Option<&str>,
        is_public: bool,
    ) -> Result<Project, AppError> {
        let id = Uuid::new_v4().to_string();
        let branch_id = Uuid::new_v4().to_string();
        let language_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let mut tx = pool.begin().await?;

        let row = sqlx::query_as::<_, ProjectRow>(
            r#"
            INSERT INTO "Project" (id, organization_id, name, slug, description, icon, is_public, config, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, '{}', $8, $9)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(organization_id)
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(icon)
        .bind(is_public)
        .bind(now)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| {
            if matches!(&error, sqlx::Error::Database(db) if db.code().as_deref() == Some("23505")) {
                AppError::Conflict("Project with this slug already exists".to_string())
            } else {
                AppError::Database(error)
            }
        })?;

        // A project is not usable without a branch and a default language. Seed
        // both in the same transaction as the project instead of swallowing
        // insert failures and leaving a half-created workspace.
        sqlx::query(
            r#"
            INSERT INTO "Branch" (
                id, project_id, name, slug, description, is_default, is_protected, created_at, updated_at
            )
            VALUES ($1, $2, 'main', 'main', NULL, TRUE, TRUE, $3, $3)
            "#,
        )
        .bind(&branch_id)
        .bind(&id)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO "Language" (
                id, project_id, code, name, is_default, is_rtl, enabled,
                position, config, created_at, updated_at
            )
            VALUES ($1, $2, 'en', 'English', TRUE, FALSE, TRUE, 0, NULL, $3, $3)
            "#,
        )
        .bind(&language_id)
        .bind(&id)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row.into())
    }

    /// Update a project
    pub async fn update(
        pool: &PgPool,
        project_id: &str,
        name: Option<&str>,
        slug: Option<&str>,
        description: Option<&str>,
        icon: Option<&str>,
        is_public: Option<bool>,
        config: Option<&serde_json::Value>,
    ) -> Result<Project, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"Project\" SET ");

        let mut has_updates = false;
        if let Some(name) = name {
            query_builder.push("name = ");
            query_builder.push_bind(name);
            has_updates = true;
        }
        if let Some(slug) = slug {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("slug = ");
            query_builder.push_bind(slug);
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
        if let Some(icon) = icon {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("icon = ");
            query_builder.push_bind(icon);
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
        if let Some(config) = config {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("config = COALESCE(\"Project\".config, '{}'::jsonb) || ");
            query_builder.push_bind(config.clone());
            has_updates = true;
        }

        if !has_updates {
            return Self::get_by_id(pool, project_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Project not found".to_string()));
        }

        query_builder.push(", updated_at = ");
        query_builder.push_bind(Utc::now());

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(project_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<ProjectRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("duplicate key") {
                    AppError::Conflict("Project with this slug already exists".to_string())
                } else {
                    AppError::Database(e.into())
                }
            })?;

        Ok(row.into())
    }

    /// Delete a project
    pub async fn delete(pool: &PgPool, project_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"Project\" WHERE id = $1")
            .bind(project_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Check if a project slug is available
    pub async fn is_slug_available(
        pool: &PgPool,
        organization_id: &str,
        slug: &str,
        exclude_project_id: Option<&str>,
    ) -> Result<bool, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM \"Project\" WHERE organization_id = ");
        query_builder.push_bind(organization_id);
        query_builder.push(" AND slug = ");
        query_builder.push_bind(slug);

        if let Some(exclude_id) = exclude_project_id {
            query_builder.push(" AND id != ");
            query_builder.push_bind(exclude_id);
        }

        let count: i64 = query_builder
            .build()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?
            .get::<i64, _>(0);

        Ok(count == 0)
    }

    /// Get projects by multiple organization IDs
    pub async fn get_by_organizations(
        pool: &PgPool,
        org_ids: &[&str],
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<cms_entity::project::Project>, AppError> {
        let rows = sqlx::query_as::<_, ProjectRow>(
            "SELECT * FROM \"Project\" WHERE organization_id = ANY($1) ORDER BY created_at DESC \
             LIMIT $2 OFFSET $3",
        )
        .bind(org_ids)
        .bind(limit.unwrap_or(50))
        .bind(offset.unwrap_or(0))
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Count projects across multiple organizations
    pub async fn count_by_organizations(pool: &PgPool, org_ids: &[&str]) -> Result<i64, AppError> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM \"Project\" WHERE organization_id = ANY($1)",
        )
        .bind(org_ids)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(row.get::<i64, _>("count"))
    }
}

/// Project settings queries
pub struct ProjectSettingsQueries;

impl ProjectSettingsQueries {
    /// Get settings for a project
    pub async fn get(pool: &PgPool, project_id: &str) -> Result<Option<ProjectSettings>, AppError> {
        let row = sqlx::query_as::<_, ProjectSettingsRow>(
            "SELECT * FROM \"ProjectSettings\" WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| ProjectSettings {
            project_id: r.project_id,
            theme: r.theme,
            default_language: r.default_language,
            custom_domain: r.custom_domain,
            search_enabled: r.search_enabled,
            comments_enabled: r.comments_enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    /// Create or update settings for a project
    pub async fn upsert(
        pool: &PgPool,
        project_id: &str,
        theme: Option<&str>,
        default_language: Option<&str>,
        custom_domain: Option<&str>,
        search_enabled: Option<bool>,
        comments_enabled: Option<bool>,
    ) -> Result<ProjectSettings, AppError> {
        let now = Utc::now();

        // Try to update first
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"ProjectSettings\" SET ");

        let mut has_updates = false;
        if let Some(theme) = theme {
            query_builder.push("theme = ");
            query_builder.push_bind(theme);
            has_updates = true;
        }
        if let Some(default_language) = default_language {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("default_language = ");
            query_builder.push_bind(default_language);
            has_updates = true;
        }
        if let Some(custom_domain) = custom_domain {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("custom_domain = ");
            query_builder.push_bind(custom_domain);
            has_updates = true;
        }
        if let Some(search_enabled) = search_enabled {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("search_enabled = ");
            query_builder.push_bind(search_enabled);
            has_updates = true;
        }
        if let Some(comments_enabled) = comments_enabled {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("comments_enabled = ");
            query_builder.push_bind(comments_enabled);
            has_updates = true;
        }

        if has_updates {
            query_builder.push(", updated_at = ");
            query_builder.push_bind(now);
        }

        query_builder.push(" WHERE project_id = ");
        query_builder.push_bind(project_id);
        query_builder.push(" RETURNING *");

        let result = query_builder
            .build_query_as::<ProjectSettingsRow>()
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        if let Some(row) = result {
            return Ok(ProjectSettings {
                project_id: row.project_id,
                theme: row.theme,
                default_language: row.default_language,
                custom_domain: row.custom_domain,
                search_enabled: row.search_enabled,
                comments_enabled: row.comments_enabled,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        // Insert if not exists
        let row = sqlx::query_as::<_, ProjectSettingsRow>(
            r#"
            INSERT INTO "ProjectSettings" (project_id, theme, default_language, custom_domain, search_enabled, comments_enabled, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#
        )
        .bind(project_id)
        .bind(theme)
        .bind(default_language)
        .bind(custom_domain)
        .bind(search_enabled.unwrap_or(true))
        .bind(comments_enabled.unwrap_or(true))
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(ProjectSettings {
            project_id: row.project_id,
            theme: row.theme,
            default_language: row.default_language,
            custom_domain: row.custom_domain,
            search_enabled: row.search_enabled,
            comments_enabled: row.comments_enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Project addon queries
pub struct ProjectAddonQueries;

impl ProjectAddonQueries {
    /// Get addons for a project
    pub async fn get_by_project(
        pool: &PgPool,
        project_id: &str,
    ) -> Result<Vec<ProjectAddon>, AppError> {
        let rows = sqlx::query_as::<_, ProjectAddonRow>(
            "SELECT * FROM \"ProjectAddon\" WHERE project_id = $1 ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Get a specific addon
    pub async fn get_by_id(
        pool: &PgPool,
        addon_id: &str,
    ) -> Result<Option<ProjectAddon>, AppError> {
        let row =
            sqlx::query_as::<_, ProjectAddonRow>("SELECT * FROM \"ProjectAddon\" WHERE id = $1")
                .bind(addon_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Create a new addon
    pub async fn create(
        pool: &PgPool,
        project_id: &str,
        addon_type: &str,
        config: serde_json::Value,
        is_enabled: bool,
    ) -> Result<ProjectAddon, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, ProjectAddonRow>(
            r#"
            INSERT INTO "ProjectAddon" (id, project_id, addon_type, config, is_enabled, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(project_id)
        .bind(addon_type)
        .bind(config)
        .bind(is_enabled)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update an addon
    pub async fn update(
        pool: &PgPool,
        addon_id: &str,
        config: Option<serde_json::Value>,
        is_enabled: Option<bool>,
    ) -> Result<ProjectAddon, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"ProjectAddon\" SET ");

        let mut has_updates = false;
        if let Some(config) = config {
            query_builder.push("config = ");
            query_builder.push_bind(config);
            has_updates = true;
        }
        if let Some(is_enabled) = is_enabled {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("is_enabled = ");
            query_builder.push_bind(is_enabled);
            has_updates = true;
        }

        if has_updates {
            query_builder.push(", updated_at = ");
            query_builder.push_bind(Utc::now());
        }

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(addon_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<ProjectAddonRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Delete an addon
    pub async fn delete(pool: &PgPool, addon_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"ProjectAddon\" WHERE id = $1")
            .bind(addon_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}
