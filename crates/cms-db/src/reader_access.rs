//! Reader Access database queries
//!
//! This module contains all database queries related to reader access,
//! including readers, audiences, grants, invitations, and JWT replay tracking.

use chrono::{DateTime, Utc};
use cms_entity::reader_access::{
    Audience, AudienceGrant, AudienceGrantResponse, AudienceResponse, JwtAccessProvider, JwtReplay,
    Reader, ReaderAudience, ReaderAuditLog, ReaderInvitation, ReaderInvitationResponse,
    ReaderResponse, ReaderSession,
};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

// ============================================
// Reader
// ============================================

#[derive(Debug, FromRow)]
struct ReaderRow {
    id: String,
    email: String,
    name: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ReaderRow> for Reader {
    fn from(row: ReaderRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<ReaderRow> for ReaderResponse {
    fn from(row: ReaderRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Reader queries
pub struct ReaderQueries;

impl ReaderQueries {
    pub async fn get_by_id(pool: &PgPool, reader_id: &str) -> Result<Option<Reader>, AppError> {
        let row = sqlx::query_as::<_, ReaderRow>("SELECT * FROM \"Reader\" WHERE id = $1")
            .bind(reader_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_email(pool: &PgPool, email: &str) -> Result<Option<Reader>, AppError> {
        let row = sqlx::query_as::<_, ReaderRow>("SELECT * FROM \"Reader\" WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// List all readers granted access to any audience belonging to a project.
    pub async fn get_by_project(
        pool: &PgPool,
        project_id: &str,
    ) -> Result<Vec<Reader>, AppError> {
        let rows = sqlx::query_as::<_, ReaderRow>(
            r#"
            SELECT DISTINCT r.* FROM "Reader" r
            JOIN "ReaderAudience" ra ON ra.reader_id = r.id
            JOIN "Audience" a ON ra.audience_id = a.id
            WHERE a.project_id = $1
            ORDER BY r.created_at
            "#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn create(
        pool: &PgPool,
        email: &str,
        name: Option<&str>,
    ) -> Result<Reader, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, ReaderRow>(
            r#"
            INSERT INTO "Reader" (id, email, name, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(email)
        .bind(name)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                AppError::Conflict("Reader with this email already exists".to_string())
            } else {
                AppError::Database(e.into())
            }
        })?;

        Ok(row.into())
    }

    pub async fn update(
        pool: &PgPool,
        reader_id: &str,
        name: Option<&str>,
    ) -> Result<Reader, AppError> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("UPDATE \"Reader\" SET ");

        if let Some(name) = name {
            query_builder.push("name = ");
            query_builder.push_bind(name);
        } else {
            query_builder.push("name = NULL");
        }

        query_builder.push(", updated_at = ");
        query_builder.push_bind(Utc::now());
        query_builder.push(" WHERE id = ");
        query_builder.push_bind(reader_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<ReaderRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn delete(pool: &PgPool, reader_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"Reader\" WHERE id = $1")
            .bind(reader_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}

// ============================================
// Audience
// ============================================

#[derive(Debug, FromRow)]
struct AudienceRow {
    id: String,
    project_id: String,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<AudienceRow> for Audience {
    fn from(row: AudienceRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            description: row.description,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<AudienceRow> for AudienceResponse {
    fn from(row: AudienceRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            description: row.description,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Audience queries
pub struct AudienceQueries;

impl AudienceQueries {
    pub async fn get_by_id(pool: &PgPool, audience_id: &str) -> Result<Option<Audience>, AppError> {
        let row = sqlx::query_as::<_, AudienceRow>("SELECT * FROM \"Audience\" WHERE id = $1")
            .bind(audience_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_project(
        pool: &PgPool,
        project_id: &str,
    ) -> Result<Vec<Audience>, AppError> {
        let rows =
            sqlx::query_as::<_, AudienceRow>("SELECT * FROM \"Audience\" WHERE project_id = $1")
                .bind(project_id)
                .fetch_all(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn create(
        pool: &PgPool,
        project_id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<Audience, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, AudienceRow>(
            r#"
            INSERT INTO "Audience" (id, project_id, name, description, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(name)
        .bind(description)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn update(
        pool: &PgPool,
        audience_id: &str,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<Audience, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"Audience\" SET ");

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

        if has_updates {
            query_builder.push(", updated_at = ");
            query_builder.push_bind(Utc::now());
        }

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(audience_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<AudienceRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn delete(pool: &PgPool, audience_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"Audience\" WHERE id = $1")
            .bind(audience_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}

// ============================================
// ReaderAudience
// ============================================

#[derive(Debug, FromRow)]
struct ReaderAudienceRow {
    id: String,
    reader_id: String,
    audience_id: String,
    created_at: DateTime<Utc>,
}

/// ReaderAudience queries
pub struct ReaderAudienceQueries;

impl ReaderAudienceQueries {
    pub async fn get_by_reader_and_audience(
        pool: &PgPool,
        reader_id: &str,
        audience_id: &str,
    ) -> Result<Option<ReaderAudience>, AppError> {
        let row = sqlx::query_as::<_, ReaderAudienceRow>(
            "SELECT * FROM \"ReaderAudience\" WHERE reader_id = $1 AND audience_id = $2",
        )
        .bind(reader_id)
        .bind(audience_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| ReaderAudience {
            id: r.id,
            reader_id: r.reader_id,
            audience_id: r.audience_id,
            created_at: r.created_at,
        }))
    }

    pub async fn has_grant_for_project(
        pool: &PgPool,
        reader_id: &str,
        project_id: &str,
    ) -> Result<bool, AppError> {
        // Check if reader has access to any audience for this project
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM "ReaderAudience" ra
            JOIN "Audience" a ON ra.audience_id = a.id
            WHERE ra.reader_id = $1 AND a.project_id = $2
            "#,
        )
        .bind(reader_id)
        .bind(project_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(count > 0)
    }

    pub async fn create(
        pool: &PgPool,
        reader_id: &str,
        audience_id: &str,
    ) -> Result<ReaderAudience, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, ReaderAudienceRow>(
            r#"
            INSERT INTO "ReaderAudience" (id, reader_id, audience_id, created_at)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(reader_id)
        .bind(audience_id)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                AppError::Conflict("Reader already has access to this audience".to_string())
            } else {
                AppError::Database(e.into())
            }
        })?;

        Ok(ReaderAudience {
            id: row.id,
            reader_id: row.reader_id,
            audience_id: row.audience_id,
            created_at: row.created_at,
        })
    }

    pub async fn delete(
        pool: &PgPool,
        reader_id: &str,
        audience_id: &str,
    ) -> Result<bool, AppError> {
        let result =
            sqlx::query("DELETE FROM \"ReaderAudience\" WHERE reader_id = $1 AND audience_id = $2")
                .bind(reader_id)
                .bind(audience_id)
                .execute(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}

// ============================================
// AudienceGrant
// ============================================

#[derive(Debug, FromRow)]
struct AudienceGrantRow {
    id: String,
    audience_id: String,
    project_id: String,
    branch_id: Option<String>,
    language_id: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<AudienceGrantRow> for AudienceGrant {
    fn from(row: AudienceGrantRow) -> Self {
        Self {
            id: row.id,
            audience_id: row.audience_id,
            project_id: row.project_id,
            branch_id: row.branch_id,
            language_id: row.language_id,
            created_at: row.created_at,
        }
    }
}

impl From<AudienceGrantRow> for AudienceGrantResponse {
    fn from(row: AudienceGrantRow) -> Self {
        Self {
            id: row.id,
            audience_id: row.audience_id,
            project_id: row.project_id,
            branch_id: row.branch_id,
            language_id: row.language_id,
            created_at: row.created_at,
        }
    }
}

/// AudienceGrant queries
pub struct AudienceGrantQueries;

impl AudienceGrantQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        grant_id: &str,
    ) -> Result<Option<AudienceGrant>, AppError> {
        let row =
            sqlx::query_as::<_, AudienceGrantRow>("SELECT * FROM \"AudienceGrant\" WHERE id = $1")
                .bind(grant_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_audience(
        pool: &PgPool,
        audience_id: &str,
    ) -> Result<Vec<AudienceGrant>, AppError> {
        let rows = sqlx::query_as::<_, AudienceGrantRow>(
            "SELECT * FROM \"AudienceGrant\" WHERE audience_id = $1",
        )
        .bind(audience_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn has_grant_for_branch(
        pool: &PgPool,
        reader_id: &str,
        project_id: &str,
        branch_id: &str,
    ) -> Result<bool, AppError> {
        // Check if reader has access to an audience that has a grant for this branch
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM "ReaderAudience" ra
            JOIN "AudienceGrant" ag ON ra.audience_id = ag.audience_id
            WHERE ra.reader_id = $1 AND ag.project_id = $2 AND (ag.branch_id = $3 OR ag.branch_id IS NULL)
            "#
        )
        .bind(reader_id)
        .bind(project_id)
        .bind(branch_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(count > 0)
    }

    pub async fn create(
        pool: &PgPool,
        audience_id: &str,
        project_id: &str,
        branch_id: Option<&str>,
        language_id: Option<&str>,
    ) -> Result<AudienceGrant, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, AudienceGrantRow>(
            r#"
            INSERT INTO "AudienceGrant" (id, audience_id, project_id, branch_id, language_id, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(audience_id)
        .bind(project_id)
        .bind(branch_id)
        .bind(language_id)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn delete(pool: &PgPool, grant_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"AudienceGrant\" WHERE id = $1")
            .bind(grant_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}

// ============================================
// ReaderInvitation
// ============================================

#[derive(Debug, FromRow)]
struct ReaderInvitationRow {
    id: String,
    audience_id: String,
    email: String,
    token: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl From<ReaderInvitationRow> for ReaderInvitation {
    fn from(row: ReaderInvitationRow) -> Self {
        Self {
            id: row.id,
            audience_id: row.audience_id,
            email: row.email,
            token: row.token,
            expires_at: row.expires_at,
            created_at: row.created_at,
        }
    }
}

impl From<ReaderInvitationRow> for ReaderInvitationResponse {
    fn from(row: ReaderInvitationRow) -> Self {
        Self {
            id: row.id,
            audience_id: row.audience_id,
            email: row.email,
            expires_at: row.expires_at,
            created_at: row.created_at,
        }
    }
}

/// ReaderInvitation queries
pub struct ReaderInvitationQueries;

impl ReaderInvitationQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        invitation_id: &str,
    ) -> Result<Option<ReaderInvitation>, AppError> {
        let row = sqlx::query_as::<_, ReaderInvitationRow>(
            "SELECT * FROM \"ReaderInvitation\" WHERE id = $1",
        )
        .bind(invitation_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_token(
        pool: &PgPool,
        token: &str,
    ) -> Result<Option<ReaderInvitation>, AppError> {
        let row = sqlx::query_as::<_, ReaderInvitationRow>(
            "SELECT * FROM \"ReaderInvitation\" WHERE token = $1",
        )
        .bind(token)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_audience(
        pool: &PgPool,
        audience_id: &str,
    ) -> Result<Vec<ReaderInvitation>, AppError> {
        let rows = sqlx::query_as::<_, ReaderInvitationRow>(
            "SELECT * FROM \"ReaderInvitation\" WHERE audience_id = $1",
        )
        .bind(audience_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn create(
        pool: &PgPool,
        audience_id: &str,
        email: &str,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<ReaderInvitation, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, ReaderInvitationRow>(
            r#"
            INSERT INTO "ReaderInvitation" (id, audience_id, email, token, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(audience_id)
        .bind(email)
        .bind(token)
        .bind(expires_at)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                AppError::Conflict("Invitation with this token already exists".to_string())
            } else {
                AppError::Database(e.into())
            }
        })?;

        Ok(row.into())
    }

    pub async fn delete(pool: &PgPool, invitation_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"ReaderInvitation\" WHERE id = $1")
            .bind(invitation_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}

// ============================================
// ReaderSession
// ============================================

#[derive(Debug, FromRow)]
struct ReaderSessionRow {
    id: String,
    reader_id: String,
    session_token: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

/// ReaderSession queries
pub struct ReaderSessionQueries;

impl ReaderSessionQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        session_id: &str,
    ) -> Result<Option<ReaderSession>, AppError> {
        let row =
            sqlx::query_as::<_, ReaderSessionRow>("SELECT * FROM \"ReaderSession\" WHERE id = $1")
                .bind(session_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| ReaderSession {
            id: r.id,
            reader_id: r.reader_id,
            session_token: r.session_token,
            expires_at: r.expires_at,
            created_at: r.created_at,
        }))
    }

    pub async fn get_by_token(
        pool: &PgPool,
        token: &str,
    ) -> Result<Option<ReaderSession>, AppError> {
        let row = sqlx::query_as::<_, ReaderSessionRow>(
            "SELECT * FROM \"ReaderSession\" WHERE session_token = $1",
        )
        .bind(token)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| ReaderSession {
            id: r.id,
            reader_id: r.reader_id,
            session_token: r.session_token,
            expires_at: r.expires_at,
            created_at: r.created_at,
        }))
    }

    pub async fn create(
        pool: &PgPool,
        reader_id: &str,
        session_token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<ReaderSession, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, ReaderSessionRow>(
            r#"
            INSERT INTO "ReaderSession" (id, reader_id, session_token, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(reader_id)
        .bind(session_token)
        .bind(expires_at)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(ReaderSession {
            id: row.id,
            reader_id: row.reader_id,
            session_token: row.session_token,
            expires_at: row.expires_at,
            created_at: row.created_at,
        })
    }

    pub async fn delete(pool: &PgPool, session_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"ReaderSession\" WHERE id = $1")
            .bind(session_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}

// ============================================
// JwtAccessProvider
// ============================================

#[derive(Debug, FromRow)]
struct JwtAccessProviderRow {
    id: String,
    name: String,
    issuer: String,
    audience: String,
    secret: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<JwtAccessProviderRow> for JwtAccessProvider {
    fn from(row: JwtAccessProviderRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            issuer: row.issuer,
            audience: row.audience,
            // secret is intentionally excluded from the entity for security
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// JwtAccessProvider queries
pub struct JwtAccessProviderQueries;

impl JwtAccessProviderQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        provider_id: &str,
    ) -> Result<Option<JwtAccessProvider>, AppError> {
        let row = sqlx::query_as::<_, JwtAccessProviderRow>(
            "SELECT * FROM \"JwtAccessProvider\" WHERE id = $1",
        )
        .bind(provider_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_issuer_and_audience(
        pool: &PgPool,
        issuer: &str,
        audience: &str,
    ) -> Result<Option<JwtAccessProvider>, AppError> {
        let row = sqlx::query_as::<_, JwtAccessProviderRow>(
            "SELECT * FROM \"JwtAccessProvider\" WHERE issuer = $1 AND audience = $2",
        )
        .bind(issuer)
        .bind(audience)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn create(
        pool: &PgPool,
        name: &str,
        issuer: &str,
        audience: &str,
        secret: &str,
    ) -> Result<JwtAccessProvider, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, JwtAccessProviderRow>(
            r#"
            INSERT INTO "JwtAccessProvider" (id, name, issuer, audience, secret, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(name)
        .bind(issuer)
        .bind(audience)
        .bind(secret)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn delete(pool: &PgPool, provider_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"JwtAccessProvider\" WHERE id = $1")
            .bind(provider_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}

// ============================================
// JwtReplay
// ============================================

#[derive(Debug, FromRow)]
struct JwtReplayRow {
    id: String,
    jwt_id: String,
    provider_id: String,
    used_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl From<JwtReplayRow> for JwtReplay {
    fn from(row: JwtReplayRow) -> Self {
        Self {
            id: row.id,
            jwt_id: row.jwt_id,
            provider_id: row.provider_id,
            used_at: row.used_at,
            created_at: row.created_at,
        }
    }
}

/// JwtReplay queries
pub struct JwtReplayQueries;

impl JwtReplayQueries {
    pub async fn get_by_jwt_id_and_provider(
        pool: &PgPool,
        jwt_id: &str,
        provider_id: &str,
    ) -> Result<Option<JwtReplay>, AppError> {
        let row = sqlx::query_as::<_, JwtReplayRow>(
            "SELECT * FROM \"JwtReplay\" WHERE jwt_id = $1 AND provider_id = $2",
        )
        .bind(jwt_id)
        .bind(provider_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn is_replayed(
        pool: &PgPool,
        jwt_id: &str,
        provider_id: &str,
    ) -> Result<bool, AppError> {
        let exists = JwtReplayQueries::get_by_jwt_id_and_provider(pool, jwt_id, provider_id)
            .await?
            .is_some();

        Ok(exists)
    }

    pub async fn create(
        pool: &PgPool,
        jwt_id: &str,
        provider_id: &str,
    ) -> Result<JwtReplay, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, JwtReplayRow>(
            r#"
            INSERT INTO "JwtReplay" (id, jwt_id, provider_id, used_at, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(jwt_id)
        .bind(provider_id)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                AppError::Conflict("JWT already used".to_string())
            } else {
                AppError::Database(e.into())
            }
        })?;

        Ok(row.into())
    }
}

// ============================================
// ReaderAuditLog
// ============================================

#[derive(Debug, FromRow)]
struct ReaderAuditLogRow {
    id: String,
    reader_id: String,
    project_id: String,
    action: String,
    metadata: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl From<ReaderAuditLogRow> for ReaderAuditLog {
    fn from(row: ReaderAuditLogRow) -> Self {
        Self {
            id: row.id,
            reader_id: row.reader_id,
            project_id: row.project_id,
            action: row.action,
            metadata: row.metadata,
            created_at: row.created_at,
        }
    }
}

/// ReaderAuditLog queries
pub struct ReaderAuditLogQueries;

impl ReaderAuditLogQueries {
    pub async fn create(
        pool: &PgPool,
        reader_id: &str,
        project_id: &str,
        action: &str,
        metadata: serde_json::Value,
    ) -> Result<ReaderAuditLog, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, ReaderAuditLogRow>(
            r#"
            INSERT INTO "ReaderAuditLog" (id, reader_id, project_id, action, metadata, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(reader_id)
        .bind(project_id)
        .bind(action)
        .bind(metadata)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn get_by_reader(
        pool: &PgPool,
        reader_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<ReaderAuditLog>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"ReaderAuditLog\" WHERE reader_id = ");
        query_builder.push_bind(reader_id);

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
            .build_query_as::<ReaderAuditLogRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}
