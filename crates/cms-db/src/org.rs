//! Organization database queries

use chrono::{DateTime, Utc};
use cms_entity::{
    common::MemberRole,
    org::{Member, MemberResponse, Organization, OrganizationResponse},
};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

/// Database representation of an organization row
#[derive(Debug, FromRow)]
struct OrganizationRow {
    id: String,
    name: String,
    slug: String,
    description: Option<String>,
    logo: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Database representation of a member row
#[derive(Debug, FromRow)]
struct MemberRow {
    id: String,
    user_id: String,
    organization_id: String,
    role: MemberRole,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<OrganizationRow> for Organization {
    fn from(row: OrganizationRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            slug: row.slug,
            description: row.description,
            logo: row.logo,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<OrganizationRow> for OrganizationResponse {
    fn from(row: OrganizationRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            slug: row.slug,
            description: row.description,
            logo: row.logo,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<MemberRow> for Member {
    fn from(row: MemberRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            organization_id: row.organization_id,
            role: row.role,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<MemberRow> for MemberResponse {
    fn from(row: MemberRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            organization_id: row.organization_id,
            role: row.role,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Organization queries
pub struct OrganizationQueries;

impl OrganizationQueries {
    /// Get an organization by ID
    pub async fn get_by_id(pool: &PgPool, org_id: &str) -> Result<Option<Organization>, AppError> {
        let row =
            sqlx::query_as::<_, OrganizationRow>("SELECT * FROM \"Organization\" WHERE id = $1")
                .bind(org_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get an organization by slug
    pub async fn get_by_slug(pool: &PgPool, slug: &str) -> Result<Option<Organization>, AppError> {
        let row =
            sqlx::query_as::<_, OrganizationRow>("SELECT * FROM \"Organization\" WHERE slug = $1")
                .bind(slug)
                .fetch_optional(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get organizations by IDs
    pub async fn get_by_ids(
        pool: &PgPool,
        org_ids: &[&str],
    ) -> Result<Vec<Organization>, AppError> {
        let rows = sqlx::query_as::<_, OrganizationRow>(
            "SELECT * FROM \"Organization\" WHERE id = ANY($1)",
        )
        .bind(org_ids)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// List all organizations with pagination
    pub async fn list_all(
        pool: &PgPool,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Organization>, AppError> {
        let mut qb = QueryBuilder::<Postgres>::new(
            "SELECT * FROM \"Organization\" ORDER BY created_at DESC",
        );
        if let Some(l) = limit {
            qb.push(" LIMIT ");
            qb.push_bind(l);
        }
        if let Some(o) = offset {
            qb.push(" OFFSET ");
            qb.push_bind(o);
        }
        let rows = qb
            .build_query_as::<OrganizationRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Count all organizations
    pub async fn count_all(pool: &PgPool) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(r#"SELECT COUNT(*) FROM "Organization""#)
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;
        Ok(count)
    }

    /// Create a new organization
    pub async fn create(
        pool: &PgPool,
        name: &str,
        slug: &str,
        description: Option<&str>,
    ) -> Result<Organization, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, OrganizationRow>(
            r#"
            INSERT INTO "Organization" (id, name, slug, description, logo, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind::<Option<String>>(None)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                AppError::Conflict("Organization with this slug already exists".to_string())
            } else {
                AppError::Database(e.into())
            }
        })?;

        Ok(row.into())
    }

    /// Update an organization
    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        name: Option<&str>,
        description: Option<&str>,
        logo: Option<&str>,
    ) -> Result<Organization, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"Organization\" SET ");

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
        if let Some(logo) = logo {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("logo = ");
            query_builder.push_bind(logo);
            has_updates = true;
        }

        if has_updates {
            query_builder.push(", updated_at = ");
            query_builder.push_bind(Utc::now());
        }

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(org_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<OrganizationRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("duplicate key") {
                    AppError::Conflict("Organization with this slug already exists".to_string())
                } else {
                    AppError::Database(e.into())
                }
            })?;

        Ok(row.into())
    }

    /// Delete an organization
    pub async fn delete(pool: &PgPool, org_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"Organization\" WHERE id = $1")
            .bind(org_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Check if a slug is available
    pub async fn is_slug_available(
        pool: &PgPool,
        slug: &str,
        exclude_org_id: Option<&str>,
    ) -> Result<bool, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM \"Organization\" WHERE slug = ");
        query_builder.push_bind(slug);

        if let Some(exclude_id) = exclude_org_id {
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
}

/// Member queries
pub struct MemberQueries;

impl MemberQueries {
    /// Get a member by ID
    pub async fn get_by_id(pool: &PgPool, member_id: &str) -> Result<Option<Member>, AppError> {
        let row = sqlx::query_as::<_, MemberRow>("SELECT * FROM \"Member\" WHERE id = $1")
            .bind(member_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get a member by user and organization
    pub async fn get_by_user_and_org(
        pool: &PgPool,
        user_id: &str,
        org_id: &str,
    ) -> Result<Option<Member>, AppError> {
        let row = sqlx::query_as::<_, MemberRow>(
            "SELECT * FROM \"Member\" WHERE user_id = $1 AND organization_id = $2",
        )
        .bind(user_id)
        .bind(org_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get members by user
    pub async fn get_by_user(pool: &PgPool, user_id: &str) -> Result<Vec<Member>, AppError> {
        let rows = sqlx::query_as::<_, MemberRow>("SELECT * FROM \"Member\" WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Get members by organization
    pub async fn get_by_organization(
        pool: &PgPool,
        org_id: &str,
        role: Option<&MemberRole>,
        search: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Member>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT m.id, m.user_id, m.organization_id, m.role, m.created_at, m.updated_at FROM \
             \"Member\" m JOIN \"User\" u ON m.user_id = u.id WHERE m.organization_id = ",
        );
        query_builder.push_bind(org_id);

        if let Some(role) = role {
            query_builder.push(" AND m.role = ");
            query_builder.push_bind(role);
        }

        if let Some(search) = search {
            query_builder.push(" AND (u.email ILIKE ");
            query_builder.push_bind(format!("%{}%", search));
            query_builder.push(" OR u.name ILIKE ");
            query_builder.push_bind(format!("%{}%", search));
            query_builder.push(")");
        }

        query_builder.push(" ORDER BY m.created_at DESC");

        if let Some(limit) = limit {
            query_builder.push(" LIMIT ");
            query_builder.push_bind(limit);
        }

        if let Some(offset) = offset {
            query_builder.push(" OFFSET ");
            query_builder.push_bind(offset);
        }

        let rows = query_builder
            .build_query_as::<MemberRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Count members by organization
    pub async fn count_by_organization(
        pool: &PgPool,
        org_id: &str,
        role: Option<&MemberRole>,
        search: Option<&str>,
    ) -> Result<i64, AppError> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM \"Member\" m JOIN \"User\" u ON m.user_id = u.id WHERE \
             m.organization_id = ",
        );
        query_builder.push_bind(org_id);

        if let Some(role) = role {
            query_builder.push(" AND m.role = ");
            query_builder.push_bind(role);
        }

        if let Some(search) = search {
            query_builder.push(" AND (u.email ILIKE ");
            query_builder.push_bind(format!("%{}%", search));
            query_builder.push(" OR u.name ILIKE ");
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

    /// Create a new member
    pub async fn create(
        pool: &PgPool,
        user_id: &str,
        org_id: &str,
        role: MemberRole,
    ) -> Result<Member, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, MemberRow>(
            r#"
            INSERT INTO "Member" (id, user_id, organization_id, role, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(org_id)
        .bind(role)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                AppError::Conflict("User is already a member of this organization".to_string())
            } else {
                AppError::Database(e.into())
            }
        })?;

        Ok(row.into())
    }

    /// Update a member's role
    pub async fn update_role(
        pool: &PgPool,
        member_id: &str,
        role: MemberRole,
    ) -> Result<Member, AppError> {
        let row = sqlx::query_as::<_, MemberRow>(
            "UPDATE \"Member\" SET role = $1, updated_at = $2 WHERE id = $3 RETURNING *",
        )
        .bind(role)
        .bind(Utc::now())
        .bind(member_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Delete a member
    pub async fn delete(pool: &PgPool, member_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"Member\" WHERE id = $1")
            .bind(member_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}

/// Invitation row from database
#[derive(Debug, sqlx::FromRow)]
struct InvitationRow {
    id: String,
    organization_id: String,
    email: String,
    role: cms_entity::common::MemberRole,
    token: String,
    expires_at: chrono::DateTime<Utc>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl From<InvitationRow> for cms_entity::org::Invitation {
    fn from(row: InvitationRow) -> Self {
        Self {
            id: row.id,
            organization_id: row.organization_id,
            email: row.email,
            role: row.role,
            token: row.token,
            expires_at: row.expires_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Invitation queries for organization invitations
pub struct InvitationQueries;

impl InvitationQueries {
    /// Create an invitation
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        email: &str,
        role: cms_entity::common::MemberRole,
        token: &str,
        expires_at: chrono::DateTime<Utc>,
    ) -> Result<cms_entity::org::Invitation, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, InvitationRow>(
            r#"
            INSERT INTO "Invitation" (id, organization_id, email, role, token, expires_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(org_id)
        .bind(email)
        .bind(role)
        .bind(token)
        .bind(expires_at)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Get invitation by ID
    pub async fn get_by_id(
        pool: &PgPool,
        invitation_id: &str,
    ) -> Result<Option<cms_entity::org::Invitation>, AppError> {
        let row = sqlx::query_as::<_, InvitationRow>("SELECT * FROM \"Invitation\" WHERE id = $1")
            .bind(invitation_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get invitation by token
    pub async fn get_by_token(
        pool: &PgPool,
        token: &str,
    ) -> Result<Option<cms_entity::org::Invitation>, AppError> {
        let row =
            sqlx::query_as::<_, InvitationRow>("SELECT * FROM \"Invitation\" WHERE token = $1")
                .bind(token)
                .fetch_optional(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// List invitations for an organization
    pub async fn list_by_org(
        pool: &PgPool,
        org_id: &str,
    ) -> Result<Vec<cms_entity::org::InvitationResponse>, AppError> {
        let rows = sqlx::query_as::<_, InvitationRow>(
            "SELECT * FROM \"Invitation\" WHERE organization_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows
            .into_iter()
            .map(|r| cms_entity::org::Invitation::from(r).into())
            .collect())
    }

    /// Delete an invitation
    pub async fn delete(pool: &PgPool, invitation_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"Invitation\" WHERE id = $1")
            .bind(invitation_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all expired invitations
    pub async fn delete_expired(pool: &PgPool) -> Result<u64, AppError> {
        let result = sqlx::query("DELETE FROM \"Invitation\" WHERE expires_at < NOW()")
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected())
    }

    /// Get invitations for an organization (paginated)
    pub async fn get_by_organization(
        pool: &PgPool,
        org_id: &str,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Vec<cms_entity::org::InvitationResponse>, AppError> {
        let limit = page_size.unwrap_or(20);
        let offset = (page.unwrap_or(1) - 1) * limit;

        let rows = sqlx::query_as::<_, InvitationRow>(
            "SELECT * FROM \"Invitation\" WHERE organization_id = $1 ORDER BY created_at DESC \
             LIMIT $2 OFFSET $3",
        )
        .bind(org_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows
            .into_iter()
            .map(|r| cms_entity::org::Invitation::from(r).into())
            .collect())
    }

    /// Count invitations for an organization
    pub async fn count_by_organization(pool: &PgPool, org_id: &str) -> Result<i64, AppError> {
        let row =
            sqlx::query("SELECT COUNT(*) as count FROM \"Invitation\" WHERE organization_id = $1")
                .bind(org_id)
                .fetch_one(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.get::<i64, _>("count"))
    }
}
