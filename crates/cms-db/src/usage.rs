//! Usage and Billing database queries

use chrono::{DateTime, Utc};
use cms_entity::usage::{
    AnalyticsEvent, OrganizationUsagePlan, UsageCheckpoint, UsageEntitlement, UsageMeter,
    UsagePlan, UsagePlanMeter,
};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

// ============================================
// UsagePlan
// ============================================

#[derive(Debug, FromRow)]
struct UsagePlanRow {
    id: String,
    name: String,
    description: Option<String>,
    price: i64,
    billing_period: String,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<UsagePlanRow> for UsagePlan {
    fn from(row: UsagePlanRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            price: row.price,
            billing_period: row.billing_period,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// UsagePlan queries
pub struct UsagePlanQueries;

impl UsagePlanQueries {
    pub async fn get_by_id(pool: &PgPool, plan_id: &str) -> Result<Option<UsagePlan>, AppError> {
        let row = sqlx::query_as::<_, UsagePlanRow>("SELECT * FROM \"UsagePlan\" WHERE id = $1")
            .bind(plan_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_all(
        pool: &PgPool,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<UsagePlan>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"UsagePlan\"");

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
            .build_query_as::<UsagePlanRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn count(pool: &PgPool) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM \"UsagePlan\"")
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(count)
    }

    pub async fn create(
        pool: &PgPool,
        name: &str,
        description: Option<&str>,
        price: i64,
        billing_period: &str,
    ) -> Result<UsagePlan, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, UsagePlanRow>(
            r#"
            INSERT INTO "UsagePlan" (id, name, description, price, billing_period, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(price)
        .bind(billing_period)
        .bind(true)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }
}

// ============================================
// UsageMeter
// ============================================

#[derive(Debug, FromRow)]
struct UsageMeterRow {
    id: String,
    code: String,
    name: String,
    description: Option<String>,
    unit: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<UsageMeterRow> for UsageMeter {
    fn from(row: UsageMeterRow) -> Self {
        Self {
            id: row.id,
            code: row.code,
            name: row.name,
            description: row.description,
            unit: row.unit,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// UsageMeter queries
pub struct UsageMeterQueries;

impl UsageMeterQueries {
    pub async fn get_by_id(pool: &PgPool, meter_id: &str) -> Result<Option<UsageMeter>, AppError> {
        let row = sqlx::query_as::<_, UsageMeterRow>("SELECT * FROM \"UsageMeter\" WHERE id = $1")
            .bind(meter_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_all(
        pool: &PgPool,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<UsageMeter>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"UsageMeter\"");

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
            .build_query_as::<UsageMeterRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn count(pool: &PgPool) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM \"UsageMeter\"")
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(count)
    }

    pub async fn create(
        pool: &PgPool,
        code: &str,
        name: &str,
        description: Option<&str>,
        unit: &str,
    ) -> Result<UsageMeter, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, UsageMeterRow>(
            r#"
            INSERT INTO "UsageMeter" (id, code, name, description, unit, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(code)
        .bind(name)
        .bind(description)
        .bind(unit)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }
}

// ============================================
// UsagePlanMeter
// ============================================

#[derive(Debug, FromRow)]
struct UsagePlanMeterRow {
    id: String,
    usage_plan_id: String,
    usage_meter_id: String,
    limit: i64,
    created_at: DateTime<Utc>,
}

impl From<UsagePlanMeterRow> for UsagePlanMeter {
    fn from(row: UsagePlanMeterRow) -> Self {
        Self {
            id: row.id,
            usage_plan_id: row.usage_plan_id,
            usage_meter_id: row.usage_meter_id,
            limit: row.limit,
            created_at: row.created_at,
        }
    }
}

/// UsagePlanMeter queries
pub struct UsagePlanMeterQueries;

impl UsagePlanMeterQueries {
    pub async fn get_by_plan(
        pool: &PgPool,
        plan_id: &str,
    ) -> Result<Vec<UsagePlanMeter>, AppError> {
        let rows = sqlx::query_as::<_, UsagePlanMeterRow>(
            "SELECT * FROM \"UsagePlanMeter\" WHERE usage_plan_id = $1",
        )
        .bind(plan_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn create(
        pool: &PgPool,
        usage_plan_id: &str,
        usage_meter_id: &str,
        limit: i64,
    ) -> Result<UsagePlanMeter, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, UsagePlanMeterRow>(
            r#"
            INSERT INTO "UsagePlanMeter" (id, usage_plan_id, usage_meter_id, "limit", created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(usage_plan_id)
        .bind(usage_meter_id)
        .bind(limit)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }
}

// ============================================
// UsageEntitlement
// ============================================

#[derive(Debug, FromRow)]
struct UsageEntitlementRow {
    id: String,
    usage_meter_id: String,
    name: String,
    description: Option<String>,
    is_enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<UsageEntitlementRow> for UsageEntitlement {
    fn from(row: UsageEntitlementRow) -> Self {
        Self {
            id: row.id,
            usage_meter_id: row.usage_meter_id,
            name: row.name,
            description: row.description,
            is_enabled: row.is_enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// UsageEntitlement queries
pub struct UsageEntitlementQueries;

impl UsageEntitlementQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        entitlement_id: &str,
    ) -> Result<Option<UsageEntitlement>, AppError> {
        let row = sqlx::query_as::<_, UsageEntitlementRow>(
            "SELECT * FROM \"UsageEntitlement\" WHERE id = $1",
        )
        .bind(entitlement_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_code(
        pool: &PgPool,
        code: &str,
    ) -> Result<Option<UsageEntitlement>, AppError> {
        let row = sqlx::query_as::<_, UsageEntitlementRow>(
            "SELECT * FROM \"UsageEntitlement\" WHERE name = $1",
        )
        .bind(code)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_all(
        pool: &PgPool,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<UsageEntitlement>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"UsageEntitlement\"");

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
            .build_query_as::<UsageEntitlementRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn update(
        pool: &PgPool,
        entitlement_id: &str,
        name: Option<&str>,
        description: Option<&str>,
        is_enabled: Option<bool>,
    ) -> Result<UsageEntitlement, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE \"UsageEntitlement\" SET ");

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
        query_builder.push_bind(entitlement_id);
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build_query_as::<UsageEntitlementRow>()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn delete(pool: &PgPool, entitlement_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM \"UsageEntitlement\" WHERE id = $1")
            .bind(entitlement_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Create a new usage entitlement
    pub async fn create(
        pool: &PgPool,
        usage_meter_id: &str,
        name: &str,
        description: Option<&str>,
        is_enabled: bool,
    ) -> Result<UsageEntitlement, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, UsageEntitlementRow>(
            r#"
            INSERT INTO "UsageEntitlement" (id, usage_meter_id, name, description, is_enabled, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(usage_meter_id)
        .bind(name)
        .bind(description)
        .bind(is_enabled)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// Create or update usage entitlement by name
    pub async fn create_or_update(
        pool: &PgPool,
        usage_meter_id: &str,
        name: &str,
        description: Option<&str>,
        is_enabled: bool,
    ) -> Result<UsageEntitlement, AppError> {
        let existing = Self::get_by_code(pool, name).await?;
        if let Some(ent) = existing {
            Self::update(pool, &ent.id, Some(name), description, Some(is_enabled)).await
        } else {
            Self::create(pool, usage_meter_id, name, description, is_enabled).await
        }
    }
}

// ============================================
// OrganizationUsagePlan
// ============================================

#[derive(Debug, FromRow)]
struct OrganizationUsagePlanRow {
    id: String,
    organization_id: String,
    usage_plan_id: String,
    starts_at: DateTime<Utc>,
    ends_at: Option<DateTime<Utc>>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<OrganizationUsagePlanRow> for OrganizationUsagePlan {
    fn from(row: OrganizationUsagePlanRow) -> Self {
        Self {
            id: row.id,
            organization_id: row.organization_id,
            usage_plan_id: row.usage_plan_id,
            starts_at: row.starts_at,
            ends_at: row.ends_at,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// OrganizationUsagePlan queries
pub struct OrganizationUsagePlanQueries;

impl OrganizationUsagePlanQueries {
    pub async fn get_by_id(
        pool: &PgPool,
        org_plan_id: &str,
    ) -> Result<Option<OrganizationUsagePlan>, AppError> {
        let row = sqlx::query_as::<_, OrganizationUsagePlanRow>(
            "SELECT * FROM \"OrganizationUsagePlan\" WHERE id = $1",
        )
        .bind(org_plan_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_by_organization(
        pool: &PgPool,
        org_id: &str,
    ) -> Result<Option<OrganizationUsagePlan>, AppError> {
        let row = sqlx::query_as::<_, OrganizationUsagePlanRow>(
            "SELECT * FROM \"OrganizationUsagePlan\" WHERE organization_id = $1 LIMIT 1",
        )
        .bind(org_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn create(
        pool: &PgPool,
        organization_id: &str,
        usage_plan_id: &str,
        starts_at: DateTime<Utc>,
        ends_at: Option<DateTime<Utc>>,
    ) -> Result<OrganizationUsagePlan, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, OrganizationUsagePlanRow>(
            r#"
            INSERT INTO "OrganizationUsagePlan" (id, organization_id, usage_plan_id, starts_at, ends_at, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(organization_id)
        .bind(usage_plan_id)
        .bind(starts_at)
        .bind(ends_at)
        .bind("ACTIVE")
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn update_plan(
        pool: &PgPool,
        organization_id: &str,
        usage_plan_id: &str,
    ) -> Result<OrganizationUsagePlan, AppError> {
        let now = Utc::now();
        let row = sqlx::query_as::<_, OrganizationUsagePlanRow>(
            r#"
            UPDATE "OrganizationUsagePlan"
            SET usage_plan_id = $1, updated_at = $2
            WHERE organization_id = $3
            RETURNING *
            "#,
        )
        .bind(usage_plan_id)
        .bind(now)
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }
}

// ============================================
// AnalyticsEvent
// ============================================

#[derive(Debug, FromRow)]
struct AnalyticsEventRow {
    id: String,
    organization_id: Option<String>,
    project_id: Option<String>,
    user_id: Option<String>,
    event_type: String,
    metadata: serde_json::Value,
    ip_address: Option<String>,
    user_agent: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<AnalyticsEventRow> for AnalyticsEvent {
    fn from(row: AnalyticsEventRow) -> Self {
        Self {
            id: row.id,
            organization_id: row.organization_id,
            project_id: row.project_id,
            user_id: row.user_id,
            event_type: row.event_type,
            metadata: row.metadata,
            ip_address: row.ip_address,
            user_agent: row.user_agent,
            created_at: row.created_at,
        }
    }
}

/// AnalyticsEvent queries
pub struct AnalyticsEventQueries;

impl AnalyticsEventQueries {
    pub async fn create(
        pool: &PgPool,
        organization_id: Option<&str>,
        project_id: Option<&str>,
        user_id: Option<&str>,
        event_type: &str,
        metadata: serde_json::Value,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<AnalyticsEvent, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, AnalyticsEventRow>(
            r#"
            INSERT INTO "AnalyticsEvent" (id, organization_id, project_id, user_id, event_type, metadata, ip_address, user_agent, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(organization_id)
        .bind(project_id)
        .bind(user_id)
        .bind(event_type)
        .bind(metadata)
        .bind(ip_address)
        .bind(user_agent)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    pub async fn query(
        pool: &PgPool,
        organization_id: Option<&str>,
        project_id: Option<&str>,
        user_id: Option<&str>,
        event_type: Option<&str>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<AnalyticsEvent>, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM \"AnalyticsEvent\" WHERE 1=1");

        if let Some(org_id) = organization_id {
            query_builder.push(" AND organization_id = ");
            query_builder.push_bind(org_id);
        }

        if let Some(pid) = project_id {
            query_builder.push(" AND project_id = ");
            query_builder.push_bind(pid);
        }

        if let Some(uid) = user_id {
            query_builder.push(" AND user_id = ");
            query_builder.push_bind(uid);
        }

        if let Some(et) = event_type {
            query_builder.push(" AND event_type = ");
            query_builder.push_bind(et);
        }

        if let Some(start) = start_date {
            query_builder.push(" AND created_at >= ");
            query_builder.push_bind(start);
        }

        if let Some(end) = end_date {
            query_builder.push(" AND created_at <= ");
            query_builder.push_bind(end);
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
            .build_query_as::<AnalyticsEventRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn count(
        pool: &PgPool,
        organization_id: Option<&str>,
        project_id: Option<&str>,
        user_id: Option<&str>,
        event_type: Option<&str>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<i64, AppError> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM \"AnalyticsEvent\" WHERE 1=1");

        if let Some(org_id) = organization_id {
            query_builder.push(" AND organization_id = ");
            query_builder.push_bind(org_id);
        }

        if let Some(pid) = project_id {
            query_builder.push(" AND project_id = ");
            query_builder.push_bind(pid);
        }

        if let Some(uid) = user_id {
            query_builder.push(" AND user_id = ");
            query_builder.push_bind(uid);
        }

        if let Some(et) = event_type {
            query_builder.push(" AND event_type = ");
            query_builder.push_bind(et);
        }

        if let Some(start) = start_date {
            query_builder.push(" AND created_at >= ");
            query_builder.push_bind(start);
        }

        if let Some(end) = end_date {
            query_builder.push(" AND created_at <= ");
            query_builder.push_bind(end);
        }

        let count: i64 = query_builder
            .build()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?
            .get::<i64, _>(0);

        Ok(count)
    }
}

// ============================================
// UsageCheckpoint
// ============================================

#[derive(Debug, FromRow)]
struct UsageCheckpointRow {
    id: String,
    event_type: String,
    entity_id: String,
    period_start: DateTime<Utc>,
    processed_at: DateTime<Utc>,
}

impl From<UsageCheckpointRow> for UsageCheckpoint {
    fn from(row: UsageCheckpointRow) -> Self {
        Self {
            id: row.id,
            event_type: row.event_type,
            entity_id: row.entity_id,
            period_start: row.period_start,
            processed_at: row.processed_at,
        }
    }
}

/// UsageCheckpoint queries
pub struct UsageCheckpointQueries;

impl UsageCheckpointQueries {
    pub async fn create(
        pool: &PgPool,
        event_type: &str,
        entity_id: &str,
        period_start: DateTime<Utc>,
    ) -> Result<UsageCheckpoint, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Check if checkpoint already exists
        let existing = sqlx::query_scalar::<_, Option<String>>(
            r#"
            SELECT id FROM "UsageCheckpoint" 
            WHERE event_type = $1 AND entity_id = $2 AND period_start = $3
            "#,
        )
        .bind(event_type)
        .bind(entity_id)
        .bind(period_start)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        if existing.is_some() {
            // Checkpoint already exists - return it
            let row = sqlx::query_as::<_, UsageCheckpointRow>(
                "SELECT * FROM \"UsageCheckpoint\" WHERE id = $1",
            )
            .bind(existing.unwrap())
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

            return Ok(row.into());
        }

        let row = sqlx::query_as::<_, UsageCheckpointRow>(
            r#"
            INSERT INTO "UsageCheckpoint" (id, event_type, entity_id, period_start, processed_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(event_type)
        .bind(entity_id)
        .bind(period_start)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }
}
