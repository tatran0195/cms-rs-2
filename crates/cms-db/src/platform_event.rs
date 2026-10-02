//! Platform Event database queries

use chrono::{DateTime, Utc};
use cms_entity::platform_event::{PlatformEvent, PlatformEventResponse};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

/// Database representation of a platform event row
#[derive(Debug, FromRow)]
struct PlatformEventRow {
    id: String,
    organization_id: Option<String>,
    user_id: Option<String>,
    event_type: String,
    metadata: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl From<PlatformEventRow> for PlatformEvent {
    fn from(row: PlatformEventRow) -> Self {
        Self {
            id: row.id,
            organization_id: row.organization_id,
            user_id: row.user_id,
            event_type: row.event_type,
            metadata: row.metadata,
            created_at: row.created_at,
        }
    }
}

impl From<PlatformEventRow> for PlatformEventResponse {
    fn from(row: PlatformEventRow) -> Self {
        Self {
            id: row.id,
            organization_id: row.organization_id,
            user_id: row.user_id,
            event_type: row.event_type,
            metadata: row.metadata,
            created_at: row.created_at,
        }
    }
}

/// PlatformEvent queries
pub struct PlatformEventQueries;

impl PlatformEventQueries {
    /// Get a platform event by ID
    pub async fn get_by_id(
        pool: &PgPool,
        event_id: &str,
    ) -> Result<Option<PlatformEvent>, AppError> {
        let row =
            sqlx::query_as::<_, PlatformEventRow>("SELECT * FROM \"PlatformEvent\" WHERE id = $1")
                .bind(event_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    /// Get platform events by organization
    pub async fn get_by_organization(
        pool: &PgPool,
        org_id: Option<&str>,
        event_type: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<PlatformEvent>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"PlatformEvent\" WHERE 1=1");

        if let Some(org_id) = org_id {
            query_builder.push(" AND organization_id = ");
            query_builder.push_bind(org_id);
        }

        if let Some(event_type) = event_type {
            query_builder.push(" AND event_type = ");
            query_builder.push_bind(event_type);
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
            .build_query_as::<PlatformEventRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Count platform events by organization
    pub async fn count_by_organization(
        pool: &PgPool,
        org_id: Option<&str>,
        event_type: Option<&str>,
    ) -> Result<i64, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM \"PlatformEvent\" WHERE 1=1");

        if let Some(org_id) = org_id {
            query_builder.push(" AND organization_id = ");
            query_builder.push_bind(org_id);
        }

        if let Some(event_type) = event_type {
            query_builder.push(" AND event_type = ");
            query_builder.push_bind(event_type);
        }

        let count: i64 = query_builder
            .build()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?
            .get::<i64, _>(0);

        Ok(count)
    }

    /// Create a new platform event
    pub async fn create(
        pool: &PgPool,
        organization_id: Option<&str>,
        user_id: Option<&str>,
        event_type: &str,
        metadata: serde_json::Value,
    ) -> Result<PlatformEvent, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, PlatformEventRow>(
            r#"
            INSERT INTO "PlatformEvent" (id, organization_id, user_id, event_type, metadata, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(organization_id)
        .bind(user_id)
        .bind(event_type)
        .bind(metadata)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Delete a platform event
    pub async fn delete(pool: &PgPool, event_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"PlatformEvent\" WHERE id = $1")
            .bind(event_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }
}
