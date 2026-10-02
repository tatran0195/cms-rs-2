//! Authentication database queries
//!
//! This module contains all database queries related to authentication:
//! users, sessions, accounts, API keys, and verification tokens.

use chrono::{DateTime, Utc};
use cms_entity::auth::{
    Account, ApiKey, ApiKeyResponse, Session, User, UserResponse, VerificationToken,
};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

/// Database representation of a user row
#[derive(Debug, FromRow)]
struct UserRow {
    id: String,
    email: String,
    name: Option<String>,
    image: Option<String>,
    email_verified: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Database representation of a session row
#[derive(Debug, FromRow)]
struct SessionRow {
    id: String,
    user_id: String,
    session_token: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Database representation of an account row
#[derive(Debug, FromRow)]
struct AccountRow {
    id: String,
    user_id: String,
    provider: String,
    provider_account_id: String,
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    token_type: Option<String>,
    scope: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Database representation of an API key row
#[derive(Debug, FromRow)]
struct ApiKeyRow {
    id: String,
    user_id: String,
    name: String,
    key: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
}

/// Database representation of a verification token row
#[derive(Debug, FromRow)]
struct VerificationTokenRow {
    id: String,
    identifier: String,
    token: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            name: row.name,
            image: row.image,
            email_verified: row.email_verified,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<SessionRow> for Session {
    fn from(row: SessionRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            session_token: row.session_token,
            expires_at: row.expires_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<AccountRow> for Account {
    fn from(row: AccountRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            provider: row.provider,
            provider_account_id: row.provider_account_id,
            access_token: row.access_token,
            refresh_token: row.refresh_token,
            expires_at: row.expires_at,
            token_type: row.token_type,
            scope: row.scope,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<ApiKeyRow> for ApiKey {
    fn from(row: ApiKeyRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            name: row.name,
            key: row.key,
            created_at: row.created_at,
            updated_at: row.updated_at,
            last_used_at: row.last_used_at,
        }
    }
}

impl From<VerificationTokenRow> for VerificationToken {
    fn from(row: VerificationTokenRow) -> Self {
        Self {
            id: row.id,
            identifier: row.identifier,
            token: row.token,
            expires_at: row.expires_at,
            created_at: row.created_at,
        }
    }
}

/// User queries
pub struct UserQueries;

impl UserQueries {
    /// Get a user by ID
    pub async fn get_by_id(pool: &PgPool, user_id: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query_as::<_, UserRow>("SELECT * FROM \"User\" WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Check whether an account has been suspended by a platform operator.
    pub async fn is_suspended(pool: &PgPool, user_id: &str) -> Result<bool, AppError> {
        Ok(sqlx::query_scalar::<_, bool>(
            r#"SELECT suspended_at IS NOT NULL FROM "User" WHERE id = $1"#,
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .unwrap_or(false))
    }

    /// Check the explicit platform-operator role (workspace ownership is not system admin).
    pub async fn is_system_admin(pool: &PgPool, user_id: &str) -> Result<bool, AppError> {
        Ok(sqlx::query_scalar::<_, bool>(
            r#"SELECT role = 'admin' FROM "User" WHERE id = $1"#,
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .unwrap_or(false))
    }

    /// Get a user by email
    pub async fn get_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AppError> {
        let row = sqlx::query_as::<_, UserRow>("SELECT * FROM \"User\" WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Create a new user
    pub async fn create(
        pool: &PgPool,
        email: &str,
        name: Option<&str>,
        image: Option<&str>,
        email_verified: bool,
    ) -> Result<User, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, UserRow>(
            r#"
            INSERT INTO "User" (id, email, name, image, email_verified, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(email)
        .bind(name)
        .bind(image)
        .bind(email_verified)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update a user
    pub async fn update(
        pool: &PgPool,
        user_id: &str,
        name: Option<&str>,
        image: Option<&str>,
    ) -> Result<User, AppError> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("UPDATE \"User\" SET ");

        let mut has_updates = false;
        if let Some(name) = name {
            query_builder.push("name = ");
            query_builder.push_bind(name);
            has_updates = true;
        }
        if let Some(image) = image {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("image = ");
            query_builder.push_bind(image);
            has_updates = true;
        }

        if has_updates {
            query_builder.push(", updated_at = ");
            query_builder.push_bind(Utc::now());
        }

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(user_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<UserRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Delete a user
    pub async fn delete(pool: &PgPool, user_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"User\" WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Get multiple users by IDs
    pub async fn get_by_ids(pool: &PgPool, user_ids: &[&str]) -> Result<Vec<User>, AppError> {
        if user_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows = sqlx::query_as::<_, UserRow>("SELECT * FROM \"User\" WHERE id = ANY($1)")
            .bind(user_ids)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Update a user's email verification status
    pub async fn update_verified(
        pool: &PgPool,
        user_id: &str,
        verified: bool,
    ) -> Result<User, AppError> {
        let row = sqlx::query_as::<_, UserRow>(
            "UPDATE \"User\" SET email_verified = $1, updated_at = $2 WHERE id = $3 RETURNING *",
        )
        .bind(verified)
        .bind(Utc::now())
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }
}

/// Session queries
pub struct SessionQueries;

impl SessionQueries {
    /// Get a session by ID
    pub async fn get_by_id(pool: &PgPool, session_id: &str) -> Result<Option<Session>, AppError> {
        let row = sqlx::query_as::<_, SessionRow>("SELECT * FROM \"Session\" WHERE id = $1")
            .bind(session_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get a session by token
    pub async fn get_by_token(pool: &PgPool, token: &str) -> Result<Option<Session>, AppError> {
        let row =
            sqlx::query_as::<_, SessionRow>("SELECT * FROM \"Session\" WHERE session_token = $1")
                .bind(token)
                .fetch_optional(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Create a new session
    pub async fn create(
        pool: &PgPool,
        user_id: &str,
        session_token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<Session, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, SessionRow>(
            r#"
            INSERT INTO "Session" (id, user_id, session_token, expires_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(session_token)
        .bind(expires_at)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Delete a session
    pub async fn delete(pool: &PgPool, session_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"Session\" WHERE id = $1")
            .bind(session_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all sessions for a user
    pub async fn delete_all_for_user(pool: &PgPool, user_id: &str) -> Result<u64, AppError> {
        let result = sqlx::query("DELETE FROM \"Session\" WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected())
    }
}

/// API Key queries
pub struct ApiKeyQueries;

impl ApiKeyQueries {
    /// Get an API key by ID
    pub async fn get_by_id(pool: &PgPool, key_id: &str) -> Result<Option<ApiKey>, AppError> {
        let row = sqlx::query_as::<_, ApiKeyRow>("SELECT * FROM \"ApiKey\" WHERE id = $1")
            .bind(key_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get an API key by the key value (hashed)
    pub async fn get_by_key(pool: &PgPool, hashed_key: &str) -> Result<Option<ApiKey>, AppError> {
        let row = sqlx::query_as::<_, ApiKeyRow>("SELECT * FROM \"ApiKey\" WHERE key = $1")
            .bind(hashed_key)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get API keys for a user
    pub async fn get_all_for_user(
        pool: &PgPool,
        user_id: &str,
    ) -> Result<Vec<ApiKeyResponse>, AppError> {
        let rows = sqlx::query_as::<_, ApiKeyRow>(
            "SELECT * FROM \"ApiKey\" WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| ApiKey::from(r).into()).collect())
    }

    /// List all of a user's API keys as full rows (including the hashed key, so
    /// the last-four display digits can be derived). Ordered newest first.
    pub async fn get_all_for_user_raw(
        pool: &PgPool,
        user_id: &str,
    ) -> Result<Vec<ApiKey>, AppError> {
        let rows = sqlx::query_as::<_, ApiKeyRow>(
            "SELECT * FROM \"ApiKey\" WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(ApiKey::from).collect())
    }

    /// Create a new API key
    pub async fn create(
        pool: &PgPool,
        user_id: &str,
        name: &str,
        hashed_key: &str,
    ) -> Result<ApiKey, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, ApiKeyRow>(
            r#"
            INSERT INTO "ApiKey" (id, user_id, name, key, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(name)
        .bind(hashed_key)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update API key last used time
    pub async fn update_last_used(pool: &PgPool, key_id: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE \"ApiKey\" SET last_used_at = $1 WHERE id = $2")
            .bind(Utc::now())
            .bind(key_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(())
    }

    /// Delete an API key
    pub async fn delete(pool: &PgPool, key_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"ApiKey\" WHERE id = $1")
            .bind(key_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}

/// Verification token queries
pub struct VerificationTokenQueries;

impl VerificationTokenQueries {
    /// Get a verification token by identifier
    pub async fn get_by_identifier(
        pool: &PgPool,
        identifier: &str,
    ) -> Result<Option<VerificationToken>, AppError> {
        let row = sqlx::query_as::<_, VerificationTokenRow>(
            "SELECT * FROM \"VerificationToken\" WHERE identifier = $1",
        )
        .bind(identifier)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get a verification token by token value
    pub async fn get_by_token(
        pool: &PgPool,
        token: &str,
    ) -> Result<Option<VerificationToken>, AppError> {
        let row = sqlx::query_as::<_, VerificationTokenRow>(
            "SELECT * FROM \"VerificationToken\" WHERE token = $1",
        )
        .bind(token)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Create a verification token
    pub async fn create(
        pool: &PgPool,
        identifier: &str,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<VerificationToken, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, VerificationTokenRow>(
            r#"
            INSERT INTO "VerificationToken" (id, identifier, token, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(identifier)
        .bind(token)
        .bind(expires_at)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Atomically replace any prior code for this namespaced identifier.
    pub async fn create_or_replace(
        pool: &PgPool,
        identifier: &str,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<VerificationToken, AppError> {
        let mut tx = pool.begin().await.map_err(|e| AppError::Database(e.into()))?;
        sqlx::query("DELETE FROM \"VerificationToken\" WHERE identifier = $1")
            .bind(identifier)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Database(e.into()))?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let row = sqlx::query_as::<_, VerificationTokenRow>(
            r#"INSERT INTO "VerificationToken" (id, identifier, token, expires_at, created_at)
               VALUES ($1, $2, $3, $4, $5) RETURNING *"#,
        )
        .bind(&id)
        .bind(identifier)
        .bind(token)
        .bind(expires_at)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        tx.commit().await.map_err(|e| AppError::Database(e.into()))?;
        Ok(row.into())
    }

    /// Atomically consume a matching, unexpired one-time token.
    pub async fn consume(
        pool: &PgPool,
        identifier: &str,
        token: &str,
    ) -> Result<Option<VerificationToken>, AppError> {
        let row = sqlx::query_as::<_, VerificationTokenRow>(
            r#"DELETE FROM "VerificationToken"
               WHERE identifier = $1 AND token = $2 AND expires_at >= $3
               RETURNING *"#,
        )
        .bind(identifier)
        .bind(token)
        .bind(Utc::now())
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(row.map(Into::into))
    }

    /// Delete a token after its delivery failed, without deleting a concurrent resend.
    pub async fn delete_matching(
        pool: &PgPool,
        identifier: &str,
        token: &str,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "DELETE FROM \"VerificationToken\" WHERE identifier = $1 AND token = $2",
        )
        .bind(identifier)
        .bind(token)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(result.rows_affected() > 0)
    }

    /// Delete a verification token
    pub async fn delete(pool: &PgPool, token_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"VerificationToken\" WHERE id = $1")
            .bind(token_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}

/// Per-identifier OTP send throttling. The insert/update is atomic across API instances.
pub struct EmailOtpThrottleQueries;

impl EmailOtpThrottleQueries {
    pub async fn allow_request(
        pool: &PgPool,
        identifier: &str,
        cooldown_seconds: u64,
    ) -> Result<bool, AppError> {
        let allowed: Option<String> = sqlx::query_scalar(
            r#"INSERT INTO "EmailOtpThrottle" (identifier, last_sent_at)
               VALUES ($1, NOW())
               ON CONFLICT (identifier) DO UPDATE
                 SET last_sent_at = NOW()
                 WHERE "EmailOtpThrottle".last_sent_at <=
                       NOW() - make_interval(secs => $2::double precision)
               RETURNING identifier"#,
        )
        .bind(identifier)
        .bind(cooldown_seconds as f64)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;
        Ok(allowed.is_some())
    }
}

/// Account queries
pub struct AccountQueries;

impl AccountQueries {
    /// Get an account by provider and provider account ID
    pub async fn get_by_provider(
        pool: &PgPool,
        provider: &str,
        provider_account_id: &str,
    ) -> Result<Option<Account>, AppError> {
        let row = sqlx::query_as::<_, AccountRow>(
            "SELECT * FROM \"Account\" WHERE provider = $1 AND provider_account_id = $2",
        )
        .bind(provider)
        .bind(provider_account_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get accounts for a user
    pub async fn get_all_for_user(pool: &PgPool, user_id: &str) -> Result<Vec<Account>, AppError> {
        let rows = sqlx::query_as::<_, AccountRow>("SELECT * FROM \"Account\" WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Create an account
    pub async fn create(
        pool: &PgPool,
        user_id: &str,
        provider: &str,
        provider_account_id: &str,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
        token_type: Option<&str>,
        scope: Option<&str>,
    ) -> Result<Account, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, AccountRow>(
            r#"
            INSERT INTO "Account" (id, user_id, provider, provider_account_id, access_token, refresh_token, expires_at, token_type, scope, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(user_id)
        .bind(provider)
        .bind(provider_account_id)
        .bind(access_token)
        .bind(refresh_token)
        .bind(expires_at)
        .bind(token_type)
        .bind(scope)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update an account
    pub async fn update(
        pool: &PgPool,
        account_id: &str,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<Account, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"Account\" SET ");

        let mut has_updates = false;
        if let Some(token) = access_token {
            query_builder.push("access_token = ");
            query_builder.push_bind(token);
            has_updates = true;
        }
        if let Some(token) = refresh_token {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("refresh_token = ");
            query_builder.push_bind(token);
            has_updates = true;
        }
        if let Some(expires) = expires_at {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("expires_at = ");
            query_builder.push_bind(expires);
            has_updates = true;
        }

        if has_updates {
            query_builder.push(", updated_at = ");
            query_builder.push_bind(Utc::now());
        }

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(account_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<AccountRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Delete an account
    pub async fn delete(pool: &PgPool, account_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"Account\" WHERE id = $1")
            .bind(account_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}
