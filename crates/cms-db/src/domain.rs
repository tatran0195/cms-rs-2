//! Domain database queries

use chrono::{DateTime, Utc};
use cms_entity::domain::{Domain, DomainResponse};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

// ============================================
// Domain
// ============================================

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

/// Normalize and validate a public custom hostname before it reaches storage or
/// host routing. IDNs are converted to their ASCII A-label form; schemes,
/// ports, paths, wildcards, IP literals, and malformed DNS labels are rejected.
pub fn normalize_hostname(hostname: &str) -> Result<String, AppError> {
    let candidate = hostname.trim();
    let candidate = candidate.strip_suffix('.').unwrap_or(candidate);
    if candidate.is_empty()
        || candidate.len() > 1024
        || candidate.chars().any(char::is_whitespace)
        || candidate.contains('/')
        || candidate.contains('\\')
        || candidate.contains('@')
        || candidate.contains(':')
        || candidate.contains('?')
        || candidate.contains('#')
        || candidate.contains('*')
    {
        return Err(AppError::InvalidInput(
            "Invalid custom hostname".to_string(),
        ));
    }

    let hostname = idna::domain_to_ascii_strict(candidate)
        .map_err(|_| AppError::InvalidInput("Invalid custom hostname".to_string()))?
        .to_ascii_lowercase();
    if hostname.len() > 253
        || hostname.parse::<std::net::IpAddr>().is_ok()
        || hostname.ends_with('.')
    {
        return Err(AppError::InvalidInput(
            "Invalid custom hostname".to_string(),
        ));
    }

    let labels: Vec<&str> = hostname.split('.').collect();
    if labels.len() < 2
        || labels.iter().any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
        || labels
            .last()
            .is_some_and(|tld| tld.len() < 2 || tld.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(AppError::InvalidInput(
            "Invalid custom hostname".to_string(),
        ));
    }
    Ok(hostname)
}

fn map_domain_write_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.code().as_deref() == Some("23505") {
            return AppError::Conflict("A domain with this hostname already exists".to_string());
        }
    }
    match error {
        sqlx::Error::RowNotFound => AppError::NotFound("Domain not found".to_string()),
        error => AppError::Database(error.into()),
    }
}

/// Domain queries
pub struct DomainQueries;

impl DomainQueries {
    /// Get domain by ID
    pub async fn get_by_id(pool: &PgPool, domain_id: &str) -> Result<Option<Domain>, AppError> {
        let row = sqlx::query_as::<_, DomainRow>("SELECT * FROM \"Domain\" WHERE id = $1")
            .bind(domain_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get domain by hostname
    pub async fn get_by_hostname(
        pool: &PgPool,
        hostname: &str,
    ) -> Result<Option<Domain>, AppError> {
        let Ok(hostname) = normalize_hostname(hostname) else {
            return Ok(None);
        };
        let row = sqlx::query_as::<_, DomainRow>("SELECT * FROM \"Domain\" WHERE hostname = $1")
            .bind(hostname)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Resolve a public host only after domain ownership has been marked verified.
    pub async fn get_verified_by_hostname(
        pool: &PgPool,
        hostname: &str,
    ) -> Result<Option<Domain>, AppError> {
        let Ok(hostname) = normalize_hostname(hostname) else {
            return Ok(None);
        };
        let row = sqlx::query_as::<_, DomainRow>(
            "SELECT * FROM \"Domain\" WHERE hostname = $1 AND verified_at IS NOT NULL",
        )
        .bind(hostname)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get domains by deployment ID
    pub async fn get_by_deployment(
        pool: &PgPool,
        deployment_id: &str,
    ) -> Result<Vec<Domain>, AppError> {
        let rows = sqlx::query_as::<_, DomainRow>(
            "SELECT * FROM \"Domain\" WHERE deployment_id = $1 ORDER BY is_primary DESC, \
             created_at ASC",
        )
        .bind(deployment_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Create a new domain
    pub async fn create(
        pool: &PgPool,
        deployment_id: &str,
        hostname: &str,
        is_primary: bool,
    ) -> Result<Domain, AppError> {
        let hostname = normalize_hostname(hostname)?;
        let id = Uuid::new_v4().to_string();
        let verification_token = format!("cmsrs-v1-{}", Uuid::new_v4().simple());
        let now = Utc::now();
        let mut transaction = pool
            .begin()
            .await
            .map_err(|error| AppError::Database(error.into()))?;

        let deployment_exists =
            sqlx::query(r#"SELECT id FROM "Deployment" WHERE id = $1 FOR UPDATE"#)
                .bind(deployment_id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(|error| AppError::Database(error.into()))?
                .is_some();
        if !deployment_exists {
            return Err(AppError::NotFound("Deployment not found".to_string()));
        }

        // Serialize primary-domain creation for this deployment, and keep the
        // demotion + insert atomic. The deployment row lock is shared with the
        // set-primary operation below.
        if is_primary {
            sqlx::query(
                r#"UPDATE "Domain" SET is_primary = false, updated_at = $1
                   WHERE deployment_id = $2 AND is_primary = true"#,
            )
            .bind(now)
            .bind(deployment_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| AppError::Database(error.into()))?;
        }

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
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_domain_write_error)?;

        transaction
            .commit()
            .await
            .map_err(|error| AppError::Database(error.into()))?;

        Ok(row.into())
    }

    /// Update a domain
    pub async fn update(
        pool: &PgPool,
        domain_id: &str,
        hostname: Option<&str>,
        is_primary: Option<bool>,
        ssl_certificate: Option<&str>,
        ssl_certificate_expires_at: Option<DateTime<Utc>>,
    ) -> Result<Domain, AppError> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("UPDATE \"Domain\" SET ");

        let mut has_updates = false;
        if let Some(hostname) = hostname {
            let hostname = normalize_hostname(hostname)?;
            query_builder.push("hostname = ");
            query_builder.push_bind(hostname.clone());
            has_updates = true;

            // Verification and TLS observations are bound to the exact hostname.
            // Invalidate them in the same UPDATE as the hostname change, while
            // preserving them for unrelated or same-host partial updates.
            query_builder.push(", verification_token = CASE WHEN hostname IS DISTINCT FROM ");
            query_builder.push_bind(hostname.clone());
            query_builder.push(" THEN ");
            query_builder.push_bind(format!("cmsrs-v1-{}", Uuid::new_v4().simple()));
            query_builder.push(" ELSE verification_token END");
            query_builder.push(", verified_at = CASE WHEN hostname IS DISTINCT FROM ");
            query_builder.push_bind(hostname.clone());
            query_builder.push(" THEN NULL ELSE verified_at END");
            query_builder.push(", ssl_certificate = CASE WHEN hostname IS DISTINCT FROM ");
            query_builder.push_bind(hostname.clone());
            query_builder.push(" THEN NULL ELSE ");
            if let Some(certificate) = ssl_certificate {
                query_builder.push_bind(certificate);
            } else {
                query_builder.push("ssl_certificate");
            }
            query_builder.push(" END");
            query_builder
                .push(", ssl_certificate_expires_at = CASE WHEN hostname IS DISTINCT FROM ");
            query_builder.push_bind(hostname.clone());
            query_builder.push(" THEN NULL ELSE ");
            if let Some(expires_at) = ssl_certificate_expires_at.as_ref() {
                query_builder.push_bind(*expires_at);
            } else {
                query_builder.push("ssl_certificate_expires_at");
            }
            query_builder.push(" END");
            for (column, reset_value) in [
                ("ssl_status", "'PENDING'"),
                ("ssl_last_error", "NULL"),
                ("ssl_checked_at", "NULL"),
                ("acme_order_url", "NULL"),
            ] {
                query_builder.push(", ");
                query_builder.push(column);
                query_builder.push(" = CASE WHEN hostname IS DISTINCT FROM ");
                query_builder.push_bind(hostname.clone());
                query_builder.push(" THEN ");
                query_builder.push(reset_value);
                query_builder.push(" ELSE ");
                query_builder.push(column);
                query_builder.push(" END");
            }
        }
        if let Some(is_primary) = is_primary {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("is_primary = ");
            query_builder.push_bind(is_primary);
            has_updates = true;
        }
        if hostname.is_none() {
            if let Some(ssl_certificate) = ssl_certificate {
                if has_updates {
                    query_builder.push(", ");
                }
                query_builder.push("ssl_certificate = ");
                query_builder.push_bind(ssl_certificate);
                has_updates = true;
            }
            if let Some(ssl_certificate_expires_at) = ssl_certificate_expires_at {
                if has_updates {
                    query_builder.push(", ");
                }
                query_builder.push("ssl_certificate_expires_at = ");
                query_builder.push_bind(ssl_certificate_expires_at);
                has_updates = true;
            }
        }

        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("updated_at = ");
        query_builder.push_bind(Utc::now());

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(domain_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<DomainRow>()
            .fetch_one(pool)
            .await
            .map_err(map_domain_write_error)?;

        Ok(row.into())
    }

    /// Atomically make one domain the primary domain for its deployment.
    /// Locking the deployment row serializes this with primary-domain creation.
    pub async fn set_primary_for_deployment(
        pool: &PgPool,
        deployment_id: &str,
        domain_id: &str,
    ) -> Result<Domain, AppError> {
        let mut transaction = pool.begin().await?;
        let deployment_exists =
            sqlx::query(r#"SELECT id FROM "Deployment" WHERE id = $1 FOR UPDATE"#)
                .bind(deployment_id)
                .fetch_optional(&mut *transaction)
                .await?
                .is_some();
        if !deployment_exists {
            return Err(AppError::NotFound("Deployment not found".to_string()));
        }

        let now = Utc::now();
        sqlx::query(
            r#"UPDATE "Domain" SET is_primary = false, updated_at = $1
               WHERE deployment_id = $2 AND is_primary = true AND id <> $3"#,
        )
        .bind(now)
        .bind(deployment_id)
        .bind(domain_id)
        .execute(&mut *transaction)
        .await?;

        let row = sqlx::query_as::<_, DomainRow>(
            r#"UPDATE "Domain" SET is_primary = true, updated_at = $1
               WHERE id = $2 AND deployment_id = $3
               RETURNING *"#,
        )
        .bind(now)
        .bind(domain_id)
        .bind(deployment_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| AppError::NotFound("Domain not found".to_string()))?;

        transaction.commit().await?;
        Ok(row.into())
    }

    /// Mark a domain verified only if the persisted challenge has not changed.
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

    /// Revoke verification after an explicit failed re-check, failing closed.
    pub async fn unverify(
        pool: &PgPool,
        domain_id: &str,
        expected_token: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"UPDATE "Domain"
               SET verified_at = NULL, ssl_status = 'PENDING', ssl_last_error = NULL,
                   ssl_checked_at = NULL, ssl_certificate = NULL,
                   ssl_certificate_expires_at = NULL, acme_order_url = NULL, updated_at = $1
               WHERE id = $2 AND verification_token = $3"#,
        )
        .bind(Utc::now())
        .bind(domain_id)
        .bind(expected_token)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(())
    }

    /// Record a successful HTTPS request observed through the configured trusted
    /// TLS proxy. The proxy owns issuance and renewal; CMS records only the
    /// handshake observation and never receives or stores a private key.
    pub async fn mark_tls_active(pool: &PgPool, domain_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query(
            r#"UPDATE "Domain"
               SET ssl_status = 'ACTIVE', ssl_checked_at = NOW(),
                   ssl_last_error = NULL, updated_at = NOW()
               WHERE id = $1 AND verified_at IS NOT NULL"#,
        )
        .bind(domain_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
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

    /// Check if a hostname is available (not taken by another domain)
    pub async fn is_hostname_available(
        pool: &PgPool,
        hostname: &str,
        exclude_domain_id: Option<&str>,
    ) -> Result<bool, AppError> {
        let hostname = normalize_hostname(hostname)?;
        let count: i64 = if let Some(domain_id) = exclude_domain_id {
            sqlx::query_scalar(r#"SELECT COUNT(*) FROM "Domain" WHERE hostname = $1 AND id != $2"#)
                .bind(hostname)
                .bind(domain_id)
                .fetch_one(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?
        } else {
            sqlx::query_scalar(r#"SELECT COUNT(*) FROM "Domain" WHERE hostname = $1"#)
                .bind(hostname)
                .fetch_one(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?
        };

        Ok(count == 0)
    }

    /// Get the most recent verified primary domain for a project's active
    /// deployment on the requested branch.
    pub async fn get_primary_for_project_branch(
        pool: &PgPool,
        project_id: &str,
        branch_id: &str,
    ) -> Result<Option<Domain>, AppError> {
        let row = sqlx::query_as::<_, DomainRow>(
            r#"
            SELECT domain.*
            FROM "Domain" AS domain
            JOIN "Deployment" AS deployment ON deployment.id = domain.deployment_id
            WHERE deployment.project_id = $1
              AND deployment.branch_id = $2
              AND deployment.status = 'ACTIVE'
              AND domain.is_primary = TRUE
              AND domain.verified_at IS NOT NULL
            ORDER BY deployment.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(project_id)
        .bind(branch_id)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(Into::into))
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
}

#[cfg(test)]
mod hostname_tests {
    use super::normalize_hostname;

    #[test]
    fn normalizes_case_trailing_dot_and_idna() {
        assert_eq!(
            normalize_hostname(" Docs.Example.COM. ").unwrap(),
            "docs.example.com"
        );
        assert_eq!(
            normalize_hostname("tài-liệu.example.com").unwrap(),
            "xn--ti-liu-ita8532d.example.com"
        );
    }

    #[test]
    fn rejects_unsafe_or_invalid_hostnames() {
        for invalid in [
            "",
            "localhost",
            "https://docs.example.com",
            "docs.example.com:443",
            "docs.example.com/path",
            "user@docs.example.com",
            "*.example.com",
            "127.0.0.1",
            "-bad.example.com",
            "bad-.example.com",
            "bad..example.com",
            "example.123",
        ] {
            assert!(normalize_hostname(invalid).is_err(), "accepted {invalid:?}");
        }
    }
}
