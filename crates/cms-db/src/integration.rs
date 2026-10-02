//! Integration database queries

use chrono::{DateTime, Utc};
use cms_entity::integration::{
    IntegrationAuditEvent, IntegrationAuditEventResponse, IntegrationConfirmation,
    IntegrationConfirmationResponse, IntegrationIdempotencyRecord,
    IntegrationIdempotencyRecordResponse, IntegrationProvider, IntegrationWebhookDelivery,
    IntegrationWebhookDeliveryResponse, ProjectIntegration, ProjectIntegrationResponse,
};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

// ============================================
// ProjectIntegration
// ============================================

#[derive(Debug, FromRow)]
struct ProjectIntegrationRow {
    id: String,
    project_id: String,
    provider: IntegrationProvider,
    name: String,
    config: serde_json::Value,
    webhook_url: Option<String>,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ProjectIntegrationRow> for ProjectIntegration {
    fn from(row: ProjectIntegrationRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            provider: row.provider,
            name: row.name,
            config: row.config,
            webhook_url: row.webhook_url,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<ProjectIntegrationRow> for ProjectIntegrationResponse {
    fn from(row: ProjectIntegrationRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            provider: row.provider,
            name: row.name,
            config: row.config,
            webhook_url: row.webhook_url,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// ProjectIntegration queries
pub struct ProjectIntegrationQueries;

impl ProjectIntegrationQueries {
    /// Get integration by ID
    pub async fn get_by_id(
        pool: &PgPool,
        integration_id: &str,
    ) -> Result<Option<ProjectIntegration>, AppError> {
        let row = sqlx::query_as::<_, ProjectIntegrationRow>(
            "SELECT * FROM \"ProjectIntegration\" WHERE id = $1",
        )
        .bind(integration_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get integrations by project ID
    pub async fn get_by_project(
        pool: &PgPool,
        project_id: &str,
    ) -> Result<Vec<ProjectIntegration>, AppError> {
        let rows = sqlx::query_as::<_, ProjectIntegrationRow>(
            "SELECT * FROM \"ProjectIntegration\" WHERE project_id = $1 ORDER BY created_at ASC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Get integrations by project ID and provider
    pub async fn get_by_project_and_provider(
        pool: &PgPool,
        project_id: &str,
        provider: IntegrationProvider,
    ) -> Result<Vec<ProjectIntegration>, AppError> {
        let rows = sqlx::query_as::<_, ProjectIntegrationRow>(
            "SELECT * FROM \"ProjectIntegration\" WHERE project_id = $1 AND provider = $2 ORDER \
             BY created_at ASC",
        )
        .bind(project_id)
        .bind(provider)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Create a new project integration
    pub async fn create(
        pool: &PgPool,
        project_id: &str,
        provider: IntegrationProvider,
        name: &str,
        config: serde_json::Value,
        webhook_url: Option<&str>,
    ) -> Result<ProjectIntegration, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, ProjectIntegrationRow>(
            r#"
            INSERT INTO "ProjectIntegration" (id, project_id, provider, name, config, webhook_url, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(project_id)
        .bind(provider)
        .bind(name)
        .bind(config)
        .bind(webhook_url)
        .bind(true)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Update a project integration
    pub async fn update(
        pool: &PgPool,
        integration_id: &str,
        name: Option<&str>,
        config: Option<serde_json::Value>,
        webhook_url: Option<&str>,
        is_active: Option<bool>,
    ) -> Result<ProjectIntegration, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"ProjectIntegration\" SET ");

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
        if let Some(webhook_url) = webhook_url {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("webhook_url = ");
            query_builder.push_bind(webhook_url);
            has_updates = true;
        }
        if let Some(is_active) = is_active {
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("is_active = ");
            query_builder.push_bind(is_active);
            has_updates = true;
        }

        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("updated_at = ");
        query_builder.push_bind(Utc::now());

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(integration_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<ProjectIntegrationRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Delete a project integration
    pub async fn delete(pool: &PgPool, integration_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"ProjectIntegration\" WHERE id = $1")
            .bind(integration_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}

// ============================================
// IntegrationAuditEvent
// ============================================

#[derive(Debug, FromRow)]
struct IntegrationAuditEventRow {
    id: String,
    integration_id: String,
    event_type: String,
    payload: serde_json::Value,
    status: String,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<IntegrationAuditEventRow> for IntegrationAuditEvent {
    fn from(row: IntegrationAuditEventRow) -> Self {
        Self {
            id: row.id,
            integration_id: row.integration_id,
            event_type: row.event_type,
            payload: row.payload,
            status: row.status,
            error_message: row.error_message,
            created_at: row.created_at,
        }
    }
}

impl From<IntegrationAuditEventRow> for IntegrationAuditEventResponse {
    fn from(row: IntegrationAuditEventRow) -> Self {
        Self {
            id: row.id,
            integration_id: row.integration_id,
            event_type: row.event_type,
            payload: row.payload,
            status: row.status,
            error_message: row.error_message,
            created_at: row.created_at,
        }
    }
}

/// IntegrationAuditEvent queries
pub struct IntegrationAuditEventQueries;

impl IntegrationAuditEventQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        event_id: &str,
    ) -> Result<Option<IntegrationAuditEvent>, AppError> {
        let row = sqlx::query_as::<_, IntegrationAuditEventRow>(
            "SELECT * FROM \"IntegrationAuditEvent\" WHERE id = $1",
        )
        .bind(event_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_integration(
        pool: &PgPool,
        integration_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<IntegrationAuditEvent>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"IntegrationAuditEvent\" WHERE integration_id = ");
        query_builder.push_bind(integration_id);

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
            .build_query_as::<IntegrationAuditEventRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn create(
        pool: &PgPool,
        integration_id: &str,
        event_type: &str,
        payload: serde_json::Value,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<IntegrationAuditEvent, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, IntegrationAuditEventRow>(
            r#"
            INSERT INTO "IntegrationAuditEvent" (id, integration_id, event_type, payload, status, error_message, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(integration_id)
        .bind(event_type)
        .bind(payload)
        .bind(status)
        .bind(error_message)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }
}

// ============================================
// IntegrationConfirmation
// ============================================

#[derive(Debug, FromRow)]
struct IntegrationConfirmationRow {
    id: String,
    integration_id: String,
    confirmation_token: String,
    confirmed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<IntegrationConfirmationRow> for IntegrationConfirmation {
    fn from(row: IntegrationConfirmationRow) -> Self {
        Self {
            id: row.id,
            integration_id: row.integration_id,
            confirmation_token: row.confirmation_token,
            confirmed_at: row.confirmed_at,
            created_at: row.created_at,
        }
    }
}

impl From<IntegrationConfirmationRow> for IntegrationConfirmationResponse {
    fn from(row: IntegrationConfirmationRow) -> Self {
        Self {
            id: row.id,
            integration_id: row.integration_id,
            confirmed_at: row.confirmed_at,
            created_at: row.created_at,
        }
    }
}

/// IntegrationConfirmation queries
pub struct IntegrationConfirmationQueries;

impl IntegrationConfirmationQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        confirmation_id: &str,
    ) -> Result<Option<IntegrationConfirmation>, AppError> {
        let row = sqlx::query_as::<_, IntegrationConfirmationRow>(
            "SELECT * FROM \"IntegrationConfirmation\" WHERE id = $1",
        )
        .bind(confirmation_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_token(
        pool: &PgPool,
        token: &str,
    ) -> Result<Option<IntegrationConfirmation>, AppError> {
        let row = sqlx::query_as::<_, IntegrationConfirmationRow>(
            "SELECT * FROM \"IntegrationConfirmation\" WHERE confirmation_token = $1",
        )
        .bind(token)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn create(
        pool: &PgPool,
        integration_id: &str,
        confirmation_token: &str,
    ) -> Result<IntegrationConfirmation, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, IntegrationConfirmationRow>(
            r#"
            INSERT INTO "IntegrationConfirmation" (id, integration_id, confirmation_token, created_at)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(integration_id)
        .bind(confirmation_token)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn confirm(
        pool: &PgPool,
        confirmation_id: &str,
    ) -> Result<IntegrationConfirmation, AppError> {
        let row = sqlx::query_as::<_, IntegrationConfirmationRow>(
            "UPDATE \"IntegrationConfirmation\" SET confirmed_at = $1 WHERE id = $2 RETURNING *",
        )
        .bind(Utc::now())
        .bind(confirmation_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }
}

// ============================================
// IntegrationWebhookDelivery
// ============================================

#[derive(Debug, FromRow)]
struct IntegrationWebhookDeliveryRow {
    id: String,
    integration_id: String,
    event_type: String,
    payload: serde_json::Value,
    status: String,
    response_status: Option<i32>,
    error_message: Option<String>,
    attempts: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<IntegrationWebhookDeliveryRow> for IntegrationWebhookDelivery {
    fn from(row: IntegrationWebhookDeliveryRow) -> Self {
        Self {
            id: row.id,
            integration_id: row.integration_id,
            event_type: row.event_type,
            payload: row.payload,
            status: row.status,
            response_status: row.response_status,
            error_message: row.error_message,
            attempts: row.attempts,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<IntegrationWebhookDeliveryRow> for IntegrationWebhookDeliveryResponse {
    fn from(row: IntegrationWebhookDeliveryRow) -> Self {
        Self {
            id: row.id,
            integration_id: row.integration_id,
            event_type: row.event_type,
            payload: row.payload,
            status: row.status,
            response_status: row.response_status,
            error_message: row.error_message,
            attempts: row.attempts,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// IntegrationWebhookDelivery queries
pub struct IntegrationWebhookDeliveryQueries;

impl IntegrationWebhookDeliveryQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        delivery_id: &str,
    ) -> Result<Option<IntegrationWebhookDelivery>, AppError> {
        let row = sqlx::query_as::<_, IntegrationWebhookDeliveryRow>(
            "SELECT * FROM \"IntegrationWebhookDelivery\" WHERE id = $1",
        )
        .bind(delivery_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn find_pending(
        pool: &PgPool,
        limit: Option<i64>,
    ) -> Result<Vec<IntegrationWebhookDelivery>, AppError> {
        let rows = if let Some(limit) = limit {
            sqlx::query_as::<_, IntegrationWebhookDeliveryRow>(
                r#"SELECT * FROM "IntegrationWebhookDelivery" WHERE status = 'PENDING' ORDER BY created_at ASC LIMIT $1"#
            )
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?
        } else {
            sqlx::query_as::<_, IntegrationWebhookDeliveryRow>(
                r#"SELECT * FROM "IntegrationWebhookDelivery" WHERE status = 'PENDING' ORDER BY created_at ASC"#
            )
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?
        };

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn create(
        pool: &PgPool,
        integration_id: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<IntegrationWebhookDelivery, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, IntegrationWebhookDeliveryRow>(
            r#"
            INSERT INTO "IntegrationWebhookDelivery" (id, integration_id, event_type, payload, status, attempts, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(integration_id)
        .bind(event_type)
        .bind(payload)
        .bind("PENDING")
        .bind(0)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn update_status(
        pool: &PgPool,
        delivery_id: &str,
        status: &str,
        response_status: Option<i32>,
        error_message: Option<&str>,
    ) -> Result<IntegrationWebhookDelivery, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"IntegrationWebhookDelivery\" SET status = ");
        query_builder.push_bind(status);

        if let Some(response_status) = response_status {
            query_builder.push(", response_status = ");
            query_builder.push_bind(response_status);
        }

        if let Some(error_message) = error_message {
            query_builder.push(", error_message = ");
            query_builder.push_bind(error_message);
        }

        query_builder.push(", attempts = attempts + 1, updated_at = ");
        query_builder.push_bind(Utc::now());
        query_builder.push(" WHERE id = ");
        query_builder.push_bind(delivery_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<IntegrationWebhookDeliveryRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }
}

// ============================================
// IntegrationIdempotencyRecord
// ============================================

#[derive(Debug, FromRow)]
struct IntegrationIdempotencyRecordRow {
    id: String,
    integration_id: String,
    request_id: String,
    processed_at: DateTime<Utc>,
}

impl From<IntegrationIdempotencyRecordRow> for IntegrationIdempotencyRecord {
    fn from(row: IntegrationIdempotencyRecordRow) -> Self {
        Self {
            id: row.id,
            integration_id: row.integration_id,
            request_id: row.request_id,
            processed_at: row.processed_at,
        }
    }
}

impl From<IntegrationIdempotencyRecordRow> for IntegrationIdempotencyRecordResponse {
    fn from(row: IntegrationIdempotencyRecordRow) -> Self {
        Self {
            id: row.id,
            integration_id: row.integration_id,
            request_id: row.request_id,
            processed_at: row.processed_at,
        }
    }
}

/// IntegrationIdempotencyRecord queries
pub struct IntegrationIdempotencyRecordQueries;

impl IntegrationIdempotencyRecordQueries {
    pub async fn get_by_request_id(
        pool: &PgPool,
        integration_id: &str,
        request_id: &str,
    ) -> Result<Option<IntegrationIdempotencyRecord>, AppError> {
        let row = sqlx::query_as::<_, IntegrationIdempotencyRecordRow>(
            "SELECT * FROM \"IntegrationIdempotencyRecord\" WHERE integration_id = $1 AND \
             request_id = $2",
        )
        .bind(integration_id)
        .bind(request_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn create(
        pool: &PgPool,
        integration_id: &str,
        request_id: &str,
    ) -> Result<IntegrationIdempotencyRecord, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, IntegrationIdempotencyRecordRow>(
            r#"
            INSERT INTO "IntegrationIdempotencyRecord" (id, integration_id, request_id, processed_at)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(integration_id)
        .bind(request_id)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }
}
