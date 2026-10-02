//! Deployment database queries

use chrono::{DateTime, Utc};
use cms_entity::{
    deployment::{Deployment, DeploymentResponse, DeploymentStatus},
    domain::{Domain, DomainResponse},
};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

/// Database representation of a deployment row
#[derive(Debug, FromRow)]
struct DeploymentRow {
    id: String,
    project_id: String,
    branch_id: Option<String>,
    status: DeploymentStatus,
    build_logs: Option<String>,
    error_message: Option<String>,
    deployed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Database representation of a domain row
#[derive(Debug, FromRow)]
struct DomainRow {
    id: String,
    deployment_id: String,
    hostname: String,
    is_primary: bool,
    ssl_certificate: Option<String>,
    ssl_certificate_expires_at: Option<DateTime<Utc>>,
    ssl_status: String,
    ssl_last_error: Option<String>,
    ssl_checked_at: Option<DateTime<Utc>>,
    verified_at: Option<DateTime<Utc>>,
    verification_token: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Immutable snapshot metadata for a successfully published release.
#[derive(Debug, Clone, FromRow)]
pub struct DeploymentSnapshot {
    pub id: String,
    pub deployment_id: String,
    pub project_id: String,
    pub branch_id: String,
    pub version: i64,
    pub snapshot: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Latest successfully deployed release metadata for one documentation branch.
#[derive(Debug, Clone, FromRow)]
pub struct DeploymentBranchRelease {
    pub deployment_id: String,
    pub branch_id: String,
    pub branch_name: String,
    pub branch_slug: String,
    pub is_default: bool,
    pub version: i64,
}

/// A successful immutable release entry for a site's public changelog.
#[derive(Debug, Clone, FromRow)]
pub struct DeploymentRelease {
    pub deployment_id: String,
    pub branch_name: String,
    pub branch_slug: String,
    pub version: i64,
    pub build_logs: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Small page fields needed to build cross-language alternate links without
/// loading all document bodies into memory.
#[derive(Debug, Clone, FromRow)]
pub struct DeploymentSnapshotPageSummary {
    pub page_id: String,
    pub language_id: Option<String>,
    pub path: String,
    pub translation_key: Option<String>,
}

/// Published page metadata for listings and sitemaps, without the body JSON.
#[derive(Debug, Clone, FromRow)]
pub struct DeploymentSnapshotPageListing {
    pub page_id: String,
    pub path: String,
    pub title: String,
    pub slug: String,
    pub parent_id: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl From<DeploymentRow> for Deployment {
    fn from(row: DeploymentRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            branch_id: row.branch_id,
            status: row.status,
            build_logs: row.build_logs,
            error_message: row.error_message,
            deployed_at: row.deployed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<DeploymentRow> for DeploymentResponse {
    fn from(row: DeploymentRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            branch_id: row.branch_id,
            status: row.status,
            build_logs: row.build_logs,
            error_message: row.error_message,
            deployed_at: row.deployed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<DomainRow> for Domain {
    fn from(row: DomainRow) -> Self {
        Self {
            id: row.id,
            deployment_id: row.deployment_id,
            hostname: row.hostname,
            is_primary: row.is_primary,
            ssl_certificate: row.ssl_certificate,
            ssl_certificate_expires_at: row.ssl_certificate_expires_at,
            ssl_status: row.ssl_status,
            ssl_last_error: row.ssl_last_error,
            ssl_checked_at: row.ssl_checked_at,
            verified_at: row.verified_at,
            verification_token: row.verification_token,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<DomainRow> for DomainResponse {
    fn from(row: DomainRow) -> Self {
        Self {
            id: row.id,
            deployment_id: row.deployment_id,
            hostname: row.hostname,
            is_primary: row.is_primary,
            ssl_certificate: row.ssl_certificate,
            ssl_certificate_expires_at: row.ssl_certificate_expires_at,
            ssl_status: row.ssl_status,
            ssl_last_error: row.ssl_last_error,
            ssl_checked_at: row.ssl_checked_at,
            verified_at: row.verified_at,
            is_verified: row.verified_at.is_some(),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Deployment queries
pub struct DeploymentQueries;

impl DeploymentQueries {
    /// Get a deployment by ID
    pub async fn get_by_id(
        pool: &PgPool,
        deployment_id: &str,
    ) -> Result<Option<Deployment>, AppError> {
        let row = sqlx::query_as::<_, DeploymentRow>("SELECT * FROM \"Deployment\" WHERE id = $1")
            .bind(deployment_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get deployments by project
    pub async fn get_by_project(
        pool: &PgPool,
        project_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Deployment>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"Deployment\" WHERE project_id = ");
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
            .build_query_as::<DeploymentRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Count successfully active deployments for one project branch. Pending,
    /// failed, and deleted attempts do not consume a public release number.
    pub async fn count_active_by_project_branch(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
    ) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM "Deployment" WHERE project_id = $1 AND branch_id = $2 AND status = $3"#,
        )
        .bind(project_id)
        .bind(branch_id)
        .bind(DeploymentStatus::Active)
        .fetch_one(pool)
        .await
        .map_err(|error| AppError::Database(error.into()))?;
        Ok(count)
    }

    /// Capture a consistent point-in-time snapshot for a deployment in one SQL
    /// statement. JSON is built from the same MVCC snapshot and includes only
    /// enabled languages and published pages for the selected branch.
    pub async fn capture_snapshot(
        pool: &PgPool,
        deployment_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        let snapshot = sqlx::query_scalar::<_, serde_json::Value>(
            r#"
            SELECT jsonb_build_object(
                'project', to_jsonb(project),
                'branch', to_jsonb(branch),
                'branches', COALESCE((
                    SELECT jsonb_agg(to_jsonb(all_branches)
                        ORDER BY all_branches.is_default DESC, all_branches.created_at)
                    FROM "Branch" AS all_branches
                    WHERE all_branches.project_id = deployment.project_id
                ), '[]'::JSONB),
                'languages', COALESCE((
                    SELECT jsonb_agg(to_jsonb(language)
                        ORDER BY language.position, language.created_at)
                    FROM "Language" AS language
                    WHERE language.project_id = deployment.project_id
                      AND language.enabled = TRUE
                ), '[]'::JSONB),
                'translations', COALESCE((
                    SELECT jsonb_agg(to_jsonb(translation) ORDER BY translation.created_at)
                    FROM "ProjectTranslation" AS translation
                    JOIN "Language" AS language ON language.id = translation.language_id
                    WHERE translation.project_id = deployment.project_id
                      AND language.enabled = TRUE
                ), '[]'::JSONB),
                'openapi', (
                    SELECT document.content
                    FROM "OpenApiDocument" AS document
                    WHERE document.project_id = deployment.project_id
                    ORDER BY document.updated_at DESC, document.created_at DESC
                    LIMIT 1
                ),
                'pages', COALESCE((
                    SELECT jsonb_agg(to_jsonb(page) ORDER BY page.language_id, page.path)
                    FROM "Page" AS page
                    JOIN "Language" AS language ON language.id = page.language_id
                    WHERE page.project_id = deployment.project_id
                      AND page.branch_id = deployment.branch_id
                      AND page.is_published = TRUE
                      AND language.enabled = TRUE
                ), '[]'::JSONB)
            )
            FROM "Deployment" AS deployment
            JOIN "Project" AS project ON project.id = deployment.project_id
            JOIN "Branch" AS branch ON branch.id = deployment.branch_id
            WHERE deployment.id = $1
            "#,
        )
        .bind(deployment_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::Conflict("Deployment has no valid project branch".to_string()))?;
        Ok(snapshot)
    }

    /// Atomically store a successful release snapshot, assign its immutable
    /// branch-local version, and transition the deployment to ACTIVE.
    pub async fn publish_snapshot(
        pool: &PgPool,
        deployment_id: &str,
        snapshot: &serde_json::Value,
        actor_user_id: Option<&str>,
    ) -> Result<i64, AppError> {
        let project_id = snapshot
            .get("project")
            .and_then(|project| project.get("id"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| AppError::InvalidInput("Snapshot is missing project id".to_string()))?;
        let branch_id = snapshot
            .get("branch")
            .and_then(|branch| branch.get("id"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| AppError::InvalidInput("Snapshot is missing branch id".to_string()))?;
        let mut metadata_snapshot = snapshot.clone();
        metadata_snapshot
            .as_object_mut()
            .ok_or_else(|| AppError::InvalidInput("Snapshot must be a JSON object".to_string()))?
            .insert("pages".to_string(), serde_json::json!([]));

        let mut tx = pool.begin().await?;
        let project =
            sqlx::query_scalar::<_, String>("SELECT id FROM \"Project\" WHERE id = $1 FOR UPDATE")
                .bind(project_id)
                .fetch_optional(&mut *tx)
                .await?;
        if project.is_none() {
            return Err(AppError::NotFound("Project not found".to_string()));
        }

        let deployment = sqlx::query_as::<_, (String, Option<String>, DeploymentStatus)>(
            r#"SELECT project_id, branch_id, status FROM "Deployment"
               WHERE id = $1 FOR UPDATE"#,
        )
        .bind(deployment_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;
        if deployment.0 != project_id || deployment.1.as_deref() != Some(branch_id) {
            return Err(AppError::Conflict(
                "Deployment scope does not match its content snapshot".to_string(),
            ));
        }
        if matches!(&deployment.2, DeploymentStatus::Active) {
            let existing = sqlx::query_scalar::<_, i64>(
                r#"SELECT version FROM "DeploymentSnapshot" WHERE deployment_id = $1"#,
            )
            .bind(deployment_id)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some(version) = existing {
                tx.commit().await?;
                return Ok(version);
            }
        }
        if !matches!(
            &deployment.2,
            DeploymentStatus::Building | DeploymentStatus::Deploying
        ) {
            return Err(AppError::Conflict(
                "Only a building deployment can be published".to_string(),
            ));
        }

        let version: i64 = sqlx::query_scalar(
            r#"SELECT COALESCE(MAX(version), 0) + 1 FROM "DeploymentSnapshot"
               WHERE project_id = $1 AND branch_id = $2"#,
        )
        .bind(project_id)
        .bind(branch_id)
        .fetch_one(&mut *tx)
        .await?;
        let now = Utc::now();
        sqlx::query(
            r#"INSERT INTO "DeploymentSnapshot"
               (id, deployment_id, project_id, branch_id, version, snapshot, created_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(deployment_id)
        .bind(project_id)
        .bind(branch_id)
        .bind(version)
        .bind(&metadata_snapshot)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|error| AppError::Database(error.into()))?;
        sqlx::query(
            r#"INSERT INTO "DeploymentSnapshotPageIndex"
               (deployment_id, project_id, branch_id, page_id, language_id, path, kind,
                page_content, title, slug, parent_id, is_published, page_updated_at)
               SELECT $1, $2, $3, page->>'id', NULLIF(page->>'language_id', ''),
                      page->>'path', UPPER(COALESCE(NULLIF(page->>'kind', ''), 'PAGE')),
                      page, COALESCE(page->>'title', ''), COALESCE(page->>'slug', ''),
                      NULLIF(page->>'parent_id', ''),
                      COALESCE((page->>'is_published')::BOOLEAN, FALSE),
                      COALESCE(NULLIF(page->>'updated_at', '')::TIMESTAMPTZ, NOW())
               FROM jsonb_array_elements(COALESCE($4->'pages', '[]'::JSONB)) AS page
               WHERE page ? 'id' AND page ? 'path'"#,
        )
        .bind(deployment_id)
        .bind(project_id)
        .bind(branch_id)
        .bind(snapshot)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"UPDATE "Deployment" SET status = $1, deployed_at = $2, updated_at = $2
               WHERE id = $3"#,
        )
        .bind(DeploymentStatus::Active)
        .bind(now)
        .bind(deployment_id)
        .execute(&mut *tx)
        .await?;
        if let Some(actor_user_id) = actor_user_id {
            let organization_id = snapshot
                .get("project")
                .and_then(|project| project.get("organization_id"))
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    AppError::InvalidInput("Snapshot is missing organization id".to_string())
                })?;
            sqlx::query(
                r#"INSERT INTO "PlatformEvent" (
                       id, organization_id, user_id, event_type, metadata, created_at
                   )
                   SELECT $1, $2, actor.id, 'publish_ready',
                          jsonb_build_object(
                              'auto', FALSE,
                              'deployment_id', $3,
                              'project_id', $4,
                              'branch_id', $5
                          ), $6
                   FROM "User" actor WHERE actor.id = $7"#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(organization_id)
            .bind(deployment_id)
            .bind(project_id)
            .bind(branch_id)
            .bind(now)
            .bind(actor_user_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(version)
    }

    /// Read one immutable snapshot by deployment ID.
    pub async fn get_snapshot_by_deployment(
        pool: &PgPool,
        deployment_id: &str,
    ) -> Result<Option<DeploymentSnapshot>, AppError> {
        Ok(sqlx::query_as::<_, DeploymentSnapshot>(
            r#"SELECT snapshot.id, snapshot.deployment_id, snapshot.project_id,
                      snapshot.branch_id, snapshot.version, snapshot.snapshot, snapshot.created_at
               FROM "DeploymentSnapshot" AS snapshot
               JOIN "Deployment" AS deployment ON deployment.id = snapshot.deployment_id
               WHERE snapshot.deployment_id = $1 AND deployment.status = 'ACTIVE'"#,
        )
        .bind(deployment_id)
        .fetch_optional(pool)
        .await?)
    }

    /// Fetch the latest immutable release on a branch.
    pub async fn get_latest_snapshot_for_branch(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
    ) -> Result<Option<DeploymentSnapshot>, AppError> {
        Ok(sqlx::query_as::<_, DeploymentSnapshot>(
            r#"SELECT snapshot.id, snapshot.deployment_id, snapshot.project_id,
                      snapshot.branch_id, snapshot.version, snapshot.snapshot, snapshot.created_at
               FROM "DeploymentSnapshot" AS snapshot
               JOIN "Deployment" AS deployment ON deployment.id = snapshot.deployment_id
               WHERE snapshot.project_id = $1 AND snapshot.branch_id = $2
                 AND deployment.status = 'ACTIVE'
               ORDER BY snapshot.version DESC LIMIT 1"#,
        )
        .bind(project_id)
        .bind(branch_id)
        .fetch_optional(pool)
        .await?)
    }

    /// Find the most recent published snapshot containing a page ID. The
    /// normalized lookup index keeps this bounded without scanning Markdown.
    pub async fn get_latest_snapshot_for_page(
        pool: &PgPool,
        page_id: &str,
    ) -> Result<Option<DeploymentSnapshot>, AppError> {
        Ok(sqlx::query_as::<_, DeploymentSnapshot>(
            r#"SELECT snapshot.id, snapshot.deployment_id, snapshot.project_id,
                      snapshot.branch_id, snapshot.version, snapshot.snapshot, snapshot.created_at
               FROM "DeploymentSnapshotPageIndex" AS page_index
               JOIN "DeploymentSnapshot" AS snapshot
                 ON snapshot.deployment_id = page_index.deployment_id
               JOIN "Deployment" AS deployment ON deployment.id = snapshot.deployment_id
               WHERE page_index.page_id = $1 AND deployment.status = 'ACTIVE'
               ORDER BY snapshot.created_at DESC, snapshot.version DESC LIMIT 1"#,
        )
        .bind(page_id)
        .fetch_optional(pool)
        .await?)
    }

    /// Read only published listing metadata for one release/language. The
    /// partial covering index avoids transferring or decompressing page bodies.
    pub async fn get_snapshot_page_listing(
        pool: &PgPool,
        deployment_id: &str,
        language_id: &str,
    ) -> Result<Vec<DeploymentSnapshotPageListing>, AppError> {
        Ok(sqlx::query_as::<_, DeploymentSnapshotPageListing>(
            r#"SELECT page_id, path, title, slug, parent_id,
                      page_updated_at AS updated_at
               FROM "DeploymentSnapshotPageIndex"
               WHERE deployment_id = $1 AND language_id = $2
                 AND kind = 'PAGE' AND is_published = TRUE
               ORDER BY path"#,
        )
        .bind(deployment_id)
        .bind(language_id)
        .fetch_all(pool)
        .await?)
    }

    /// Load immutable page bodies from the normalized snapshot page store. An
    /// optional language keeps ordinary public page reads bounded.
    pub async fn get_snapshot_pages(
        pool: &PgPool,
        deployment_id: &str,
        language_id: Option<&str>,
    ) -> Result<Vec<cms_entity::page::Page>, AppError> {
        let rows = sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT page_content FROM "DeploymentSnapshotPageIndex"
               WHERE deployment_id = $1 AND ($2::TEXT IS NULL OR language_id = $2)
               ORDER BY language_id NULLS FIRST, path"#,
        )
        .bind(deployment_id)
        .bind(language_id)
        .fetch_all(pool)
        .await?;
        rows.into_iter()
            .map(|row| serde_json::from_value(row).map_err(AppError::Serialization))
            .collect()
    }

    /// Read one retained immutable page by ID.
    pub async fn get_snapshot_page(
        pool: &PgPool,
        deployment_id: &str,
        page_id: &str,
    ) -> Result<Option<cms_entity::page::Page>, AppError> {
        let row = sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT page_content FROM "DeploymentSnapshotPageIndex"
               WHERE deployment_id = $1 AND page_id = $2"#,
        )
        .bind(deployment_id)
        .bind(page_id)
        .fetch_optional(pool)
        .await?;
        row.map(|row| serde_json::from_value(row).map_err(AppError::Serialization))
            .transpose()
    }

    /// Read one page by its immutable release path and language. The lookup
    /// uses the release/language/path index and avoids loading the site's other
    /// page bodies for an ordinary SSR request.
    pub async fn get_snapshot_page_by_path(
        pool: &PgPool,
        deployment_id: &str,
        language_id: &str,
        path: &str,
    ) -> Result<Option<cms_entity::page::Page>, AppError> {
        let row = sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT page_content FROM "DeploymentSnapshotPageIndex"
               WHERE deployment_id = $1 AND language_id = $2 AND path = $3
                 AND UPPER(kind) = 'PAGE'
               LIMIT 1"#,
        )
        .bind(deployment_id)
        .bind(language_id)
        .bind(path)
        .fetch_optional(pool)
        .await?;
        row.map(|row| serde_json::from_value(row).map_err(AppError::Serialization))
            .transpose()
    }

    /// Read only page IDs, paths, and translation keys for alternate links.
    pub async fn get_snapshot_page_summaries(
        pool: &PgPool,
        deployment_id: &str,
    ) -> Result<Vec<DeploymentSnapshotPageSummary>, AppError> {
        Ok(sqlx::query_as::<_, DeploymentSnapshotPageSummary>(
            r#"SELECT page_id, language_id, path,
                      page_content->>'translation_key' AS translation_key
               FROM "DeploymentSnapshotPageIndex"
               WHERE deployment_id = $1
               ORDER BY language_id NULLS FIRST, path"#,
        )
        .bind(deployment_id)
        .fetch_all(pool)
        .await?)
    }

    /// Number of immutable pages captured by one release.
    pub async fn get_snapshot_page_count(
        pool: &PgPool,
        deployment_id: &str,
    ) -> Result<i64, AppError> {
        Ok(sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM "DeploymentSnapshotPageIndex"
               WHERE deployment_id = $1 AND UPPER(kind) = 'PAGE'"#,
        )
        .bind(deployment_id)
        .fetch_one(pool)
        .await?)
    }

    /// Latest active release for each branch, without loading large page bodies.
    pub async fn get_latest_branch_releases(
        pool: &PgPool,
        project_id: &str,
    ) -> Result<Vec<DeploymentBranchRelease>, AppError> {
        Ok(sqlx::query_as::<_, DeploymentBranchRelease>(
            r#"
            SELECT DISTINCT ON (snapshot.branch_id)
                   snapshot.deployment_id,
                   snapshot.branch_id,
                   snapshot.snapshot #>> '{branch,name}' AS branch_name,
                   snapshot.snapshot #>> '{branch,slug}' AS branch_slug,
                   COALESCE((snapshot.snapshot #>> '{branch,is_default}')::BOOLEAN, FALSE) AS is_default,
                   snapshot.version
            FROM "DeploymentSnapshot" AS snapshot
            JOIN "Deployment" AS deployment ON deployment.id = snapshot.deployment_id
            WHERE snapshot.project_id = $1 AND deployment.status = 'ACTIVE'
            ORDER BY snapshot.branch_id, snapshot.version DESC
            "#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?)
    }

    /// Read immutable published releases for the public changelog. Failed and
    /// in-progress deployment attempts have no snapshot and are excluded.
    pub async fn get_releases_by_project(
        pool: &PgPool,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<DeploymentRelease>, AppError> {
        let limit = limit.clamp(1, 100);
        Ok(sqlx::query_as::<_, DeploymentRelease>(
            r#"
            SELECT snapshot.deployment_id,
                   snapshot.snapshot #>> '{branch,name}' AS branch_name,
                   snapshot.snapshot #>> '{branch,slug}' AS branch_slug,
                   snapshot.version,
                   deployment.build_logs,
                   snapshot.created_at
            FROM "DeploymentSnapshot" AS snapshot
            JOIN "Deployment" AS deployment ON deployment.id = snapshot.deployment_id
            WHERE snapshot.project_id = $1 AND deployment.status = 'ACTIVE'
            ORDER BY snapshot.created_at DESC, snapshot.version DESC
            LIMIT $2
            "#,
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await?)
    }

    /// Claim a pending deployment so two workers cannot build the same release.
    pub async fn claim_for_build(pool: &PgPool, deployment_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query(
            r#"UPDATE "Deployment" SET status = $1, updated_at = $2,
                       error_message = NULL
               WHERE id = $3 AND status IN ($4, $5)"#,
        )
        .bind(DeploymentStatus::Building)
        .bind(Utc::now())
        .bind(deployment_id)
        .bind(DeploymentStatus::Pending)
        .bind(DeploymentStatus::Failed)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Count deployments by project
    pub async fn count_by_project(pool: &PgPool, project_id: &str) -> Result<i64, AppError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM \"Deployment\" WHERE project_id = $1")
                .bind(project_id)
                .fetch_one(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(count)
    }

    /// Create a new deployment
    pub async fn create(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        status: DeploymentStatus,
    ) -> Result<Deployment, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let branch_opt = if branch_id.is_empty() {
            None
        } else {
            Some(branch_id)
        };

        let row = sqlx::query_as::<_, DeploymentRow>(
            r#"
            INSERT INTO "Deployment" (id, project_id, branch_id, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(branch_opt)
        .bind(status)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Atomically create a pending deployment and its durable publish job.
    /// This is the outbox boundary used by the HTTP publish, merge, and rollback
    /// flows when the configured queue is PostgreSQL.
    pub async fn create_pending_with_publish_job(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
        build_logs: &str,
        extra_payload: serde_json::Value,
        actor_user_id: Option<&str>,
    ) -> Result<Deployment, AppError> {
        let mut payload = extra_payload.as_object().cloned().ok_or_else(|| {
            AppError::InvalidInput("Publish job payload must be a JSON object".to_string())
        })?;
        let deployment_id = Uuid::new_v4().to_string();
        let job_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let branch_id = (!branch_id.is_empty()).then_some(branch_id);
        payload.insert(
            "deployment_id".to_string(),
            serde_json::Value::String(deployment_id.clone()),
        );
        if let Some(actor_user_id) = actor_user_id {
            payload.insert(
                "actor_user_id".to_string(),
                serde_json::Value::String(actor_user_id.to_string()),
            );
        }

        let mut tx = pool.begin().await?;
        let deployment = sqlx::query_as::<_, DeploymentRow>(
            r#"INSERT INTO "Deployment" (
                   id, project_id, branch_id, status, build_logs, created_at, updated_at
               ) VALUES ($1, $2, $3, $4, $5, $6, $6)
               RETURNING *"#,
        )
        .bind(&deployment_id)
        .bind(project_id)
        .bind(branch_id)
        .bind(DeploymentStatus::Pending)
        .bind(build_logs)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"INSERT INTO "CmsJob" (
                   id, job_type, payload, status, retry_count, created_at, available_at
               ) VALUES ($1, 'publish', $2, 'pending', 0, $3, $3)"#,
        )
        .bind(job_id)
        .bind(serde_json::Value::Object(payload))
        .bind(now)
        .execute(&mut *tx)
        .await?;
        if let Some(actor_user_id) = actor_user_id {
            sqlx::query(
                r#"INSERT INTO "PlatformEvent" (
                       id, organization_id, user_id, event_type, metadata, created_at
                   )
                   SELECT $1, project.organization_id, actor.id, 'publish_clicked',
                          jsonb_build_object(
                              'auto', FALSE,
                              'deployment_id', $3,
                              'project_id', project.id,
                              'branch_id', $5
                          ), $6
                   FROM "Project" project
                   JOIN "User" actor ON actor.id = $2
                   WHERE project.id = $4"#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(actor_user_id)
            .bind(&deployment_id)
            .bind(project_id)
            .bind(branch_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(deployment.into())
    }

    /// Update a deployment
    pub async fn update(
        pool: &PgPool,
        deployment_id: &str,
        branch_id: Option<&str>,
    ) -> Result<Deployment, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"Deployment\" SET ");

        if let Some(branch_id) = branch_id {
            query_builder.push("branch_id = ");
            query_builder.push_bind(branch_id);
        }

        query_builder.push(", updated_at = ");
        query_builder.push_bind(Utc::now());
        query_builder.push(" WHERE id = ");
        query_builder.push_bind(deployment_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<DeploymentRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update deployment status
    pub async fn update_status(
        pool: &PgPool,
        deployment_id: &str,
        status: DeploymentStatus,
    ) -> Result<Deployment, AppError> {
        let row = sqlx::query_as::<_, DeploymentRow>(
            "UPDATE \"Deployment\" SET status = $1, updated_at = $2 WHERE id = $3 RETURNING *",
        )
        .bind(status)
        .bind(Utc::now())
        .bind(deployment_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update deployment with error
    pub async fn update_error(
        pool: &PgPool,
        deployment_id: &str,
        error_message: &str,
    ) -> Result<Deployment, AppError> {
        let row = sqlx::query_as::<_, DeploymentRow>(
            "UPDATE \"Deployment\" SET error_message = $1, status = $2, updated_at = $3 WHERE id \
             = $4 RETURNING *",
        )
        .bind(error_message)
        .bind(DeploymentStatus::Failed)
        .bind(Utc::now())
        .bind(deployment_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update deployment as deployed
    pub async fn update_deployed_at(
        pool: &PgPool,
        deployment_id: &str,
    ) -> Result<Deployment, AppError> {
        let row = sqlx::query_as::<_, DeploymentRow>(
            "UPDATE \"Deployment\" SET deployed_at = $1, status = $2, updated_at = $3 WHERE id = \
             $4 RETURNING *",
        )
        .bind(Utc::now())
        .bind(DeploymentStatus::Active)
        .bind(Utc::now())
        .bind(deployment_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update deployment build logs
    pub async fn update_build_logs(
        pool: &PgPool,
        deployment_id: &str,
        logs: &str,
    ) -> Result<Deployment, AppError> {
        let row = sqlx::query_as::<_, DeploymentRow>(
            "UPDATE \"Deployment\" SET build_logs = $1, updated_at = $2 WHERE id = $3 RETURNING *",
        )
        .bind(logs)
        .bind(Utc::now())
        .bind(deployment_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Delete a deployment
    pub async fn delete(pool: &PgPool, deployment_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"Deployment\" WHERE id = $1")
            .bind(deployment_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Get all active deployments for a project
    pub async fn get_active_by_project(
        pool: &PgPool,
        project_id: &str,
    ) -> Result<Vec<Deployment>, AppError> {
        let rows = sqlx::query_as::<_, DeploymentRow>(
            "SELECT * FROM \"Deployment\" WHERE project_id = $1 AND status = $2 ORDER BY \
             deployed_at DESC",
        )
        .bind(project_id)
        .bind(DeploymentStatus::Active)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

/// Domain queries
pub struct DomainQueries;

impl DomainQueries {
    /// Get a domain by ID
    pub async fn get_by_id(pool: &PgPool, domain_id: &str) -> Result<Option<Domain>, AppError> {
        let row = sqlx::query_as::<_, DomainRow>("SELECT * FROM \"Domain\" WHERE id = $1")
            .bind(domain_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get a domain by hostname
    pub async fn get_by_hostname(
        pool: &PgPool,
        hostname: &str,
    ) -> Result<Option<Domain>, AppError> {
        let row = sqlx::query_as::<_, DomainRow>("SELECT * FROM \"Domain\" WHERE hostname = $1")
            .bind(hostname)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get domains by deployment
    pub async fn get_by_deployment(
        pool: &PgPool,
        deployment_id: &str,
    ) -> Result<Vec<Domain>, AppError> {
        let rows =
            sqlx::query_as::<_, DomainRow>("SELECT * FROM \"Domain\" WHERE deployment_id = $1")
                .bind(deployment_id)
                .fetch_all(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Get primary domain for a deployment
    pub async fn get_primary_by_deployment(
        pool: &PgPool,
        deployment_id: &str,
    ) -> Result<Option<Domain>, AppError> {
        let row = sqlx::query_as::<_, DomainRow>(
            "SELECT * FROM \"Domain\" WHERE deployment_id = $1 AND is_primary = true LIMIT 1",
        )
        .bind(deployment_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Create a new domain
    pub async fn create(
        pool: &PgPool,
        deployment_id: &str,
        hostname: &str,
        is_primary: bool,
    ) -> Result<Domain, AppError> {
        // If this is the primary domain, clear any existing primary
        if is_primary {
            sqlx::query("UPDATE \"Domain\" SET is_primary = false WHERE deployment_id = $1")
                .bind(deployment_id)
                .execute(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;
        }

        let id = Uuid::new_v4().to_string();
        let verification_token = format!("cmsrs-v1-{}", Uuid::new_v4().simple());
        let now = Utc::now();

        let row = sqlx::query_as::<_, DomainRow>(
            r#"
            INSERT INTO "Domain" (
                id, deployment_id, hostname, is_primary, verification_token, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(deployment_id)
        .bind(hostname)
        .bind(is_primary)
        .bind(&verification_token)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                AppError::Conflict("Domain already exists".to_string())
            } else {
                AppError::Database(e.into())
            }
        })?;

        Ok(row.into())
    }

    /// Update a domain
    pub async fn update(
        pool: &PgPool,
        domain_id: &str,
        is_primary: Option<bool>,
    ) -> Result<Domain, AppError> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("UPDATE \"Domain\" SET ");

        if let Some(is_primary) = is_primary {
            query_builder.push("is_primary = ");
            query_builder.push_bind(is_primary);
        }

        query_builder.push(", updated_at = ");
        query_builder.push_bind(Utc::now());
        query_builder.push(" WHERE id = ");
        query_builder.push_bind(domain_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<DomainRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Mark a domain verified only if the persisted challenge still matches.
    pub async fn verify(
        pool: &PgPool,
        domain_id: &str,
        expected_token: &str,
    ) -> Result<Domain, AppError> {
        let now = Utc::now();
        let row = sqlx::query_as::<_, DomainRow>(
            r#"UPDATE "Domain"
               SET verified_at = COALESCE(verified_at, $1), updated_at = $1
               WHERE id = $2 AND verification_token = $3
               RETURNING *"#,
        )
        .bind(now)
        .bind(domain_id)
        .bind(expected_token)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?
        .ok_or_else(|| {
            AppError::Conflict("Domain challenge changed during verification".to_string())
        })?;

        Ok(row.into())
    }

    /// Delete a domain
    pub async fn delete(pool: &PgPool, domain_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"Domain\" WHERE id = $1")
            .bind(domain_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}
