//! Usage entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, Timestamp};

/// Usage plan entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UsagePlan {
    pub id: Id,
    pub name: String,
    pub description: Option<String>,
    pub price: i64,
    pub billing_period: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Usage plan response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UsagePlanResponse {
    pub id: Id,
    pub name: String,
    pub description: Option<String>,
    pub price: i64,
    pub billing_period: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<UsagePlan> for UsagePlanResponse {
    fn from(plan: UsagePlan) -> Self {
        Self {
            id: plan.id,
            name: plan.name,
            description: plan.description,
            price: plan.price,
            billing_period: plan.billing_period,
            is_active: plan.is_active,
            created_at: plan.created_at,
            updated_at: plan.updated_at,
        }
    }
}

/// Usage meter entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UsageMeter {
    pub id: Id,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub unit: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Usage meter response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UsageMeterResponse {
    pub id: Id,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub unit: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<UsageMeter> for UsageMeterResponse {
    fn from(meter: UsageMeter) -> Self {
        Self {
            id: meter.id,
            code: meter.code,
            name: meter.name,
            description: meter.description,
            unit: meter.unit,
            created_at: meter.created_at,
            updated_at: meter.updated_at,
        }
    }
}

/// Usage plan meter entity (junction table)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UsagePlanMeter {
    pub id: Id,
    pub usage_plan_id: Id,
    pub usage_meter_id: Id,
    pub limit: i64,
    pub created_at: DateTime<Utc>,
}

/// Usage plan meter response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UsagePlanMeterResponse {
    pub id: Id,
    pub usage_plan_id: Id,
    pub usage_meter_id: Id,
    pub limit: i64,
    pub created_at: DateTime<Utc>,
}

impl From<UsagePlanMeter> for UsagePlanMeterResponse {
    fn from(pm: UsagePlanMeter) -> Self {
        Self {
            id: pm.id,
            usage_plan_id: pm.usage_plan_id,
            usage_meter_id: pm.usage_meter_id,
            limit: pm.limit,
            created_at: pm.created_at,
        }
    }
}

/// Usage entitlement entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UsageEntitlement {
    pub id: Id,
    pub usage_meter_id: Id,
    pub name: String,
    pub description: Option<String>,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Usage entitlement response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UsageEntitlementResponse {
    pub id: Id,
    pub usage_meter_id: Id,
    pub name: String,
    pub description: Option<String>,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<UsageEntitlement> for UsageEntitlementResponse {
    fn from(entitlement: UsageEntitlement) -> Self {
        Self {
            id: entitlement.id,
            usage_meter_id: entitlement.usage_meter_id,
            name: entitlement.name,
            description: entitlement.description,
            is_enabled: entitlement.is_enabled,
            created_at: entitlement.created_at,
            updated_at: entitlement.updated_at,
        }
    }
}

/// Organization usage plan entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationUsagePlan {
    pub id: Id,
    pub organization_id: Id,
    pub usage_plan_id: Id,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Organization usage plan response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationUsagePlanResponse {
    pub id: Id,
    pub organization_id: Id,
    pub usage_plan_id: Id,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<OrganizationUsagePlan> for OrganizationUsagePlanResponse {
    fn from(oup: OrganizationUsagePlan) -> Self {
        Self {
            id: oup.id,
            organization_id: oup.organization_id,
            usage_plan_id: oup.usage_plan_id,
            starts_at: oup.starts_at,
            ends_at: oup.ends_at,
            status: oup.status,
            created_at: oup.created_at,
            updated_at: oup.updated_at,
        }
    }
}

/// Analytics event entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AnalyticsEvent {
    pub id: Id,
    pub organization_id: Option<Id>,
    pub project_id: Option<Id>,
    pub user_id: Option<Id>,
    pub event_type: String,
    pub metadata: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Analytics event response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AnalyticsEventResponse {
    pub id: Id,
    pub organization_id: Option<Id>,
    pub project_id: Option<Id>,
    pub user_id: Option<Id>,
    pub event_type: String,
    pub metadata: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<AnalyticsEvent> for AnalyticsEventResponse {
    fn from(event: AnalyticsEvent) -> Self {
        Self {
            id: event.id,
            organization_id: event.organization_id,
            project_id: event.project_id,
            user_id: event.user_id,
            event_type: event.event_type,
            metadata: event.metadata,
            ip_address: event.ip_address,
            user_agent: event.user_agent,
            created_at: event.created_at,
        }
    }
}

/// Usage checkpoint entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UsageCheckpoint {
    pub id: Id,
    pub event_type: String,
    pub entity_id: String,
    pub period_start: DateTime<Utc>,
    pub processed_at: DateTime<Utc>,
}

/// Create usage plan request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateUsagePlanRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub price: i64,
    #[serde(default = "default_monthly")]
    pub billing_period: String,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_monthly() -> String {
    "MONTHLY".to_string()
}

fn default_true() -> bool {
    true
}

/// Create usage meter request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateUsageMeterRequest {
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub unit: String,
}

/// Track analytics event request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct TrackAnalyticsEventRequest {
    pub event_type: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub ip_address: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
}

/// List analytics events query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListAnalyticsEventsQuery {
    #[serde(default)]
    pub organization_id: Option<Id>,
    #[serde(default)]
    pub project_id: Option<Id>,
    #[serde(default)]
    pub user_id: Option<Id>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub start_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub end_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}
