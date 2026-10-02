//! Analytics entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, Timestamp};

/// Track analytics event request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct TrackAnalyticsEventRequest {
    pub organization_id: Option<Id>,
    pub project_id: Option<Id>,
    pub user_id: Option<Id>,
    pub event_type: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// List analytics events query
pub type ListAnalyticsEventsQuery = AnalyticsQueryRequest;

/// Analytics event entity (duplicated from usage.rs for clarity, but we'll keep it here)
/// Note: This is a simplified version for the analytics crate
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

/// Analytics query request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct AnalyticsQueryRequest {
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
    pub group_by: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub page_size: Option<i64>,
}

/// Analytics result item
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AnalyticsResultItem {
    pub date: Option<String>,
    pub group_value: Option<String>,
    pub count: i64,
    pub event_type: String,
}

/// Analytics query response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AnalyticsQueryResponse {
    pub query: AnalyticsQueryRequest,
    pub results: Vec<AnalyticsResultItem>,
    pub total: i64,
    /// Events list (for paginated results)
    #[serde(default)]
    pub events: Vec<AnalyticsEventResponse>,
    /// Current page (1-based)
    #[serde(default)]
    pub page: i64,
    /// Items per page
    #[serde(default)]
    pub page_size: i64,
}

/// Page view analytics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PageViewAnalytics {
    pub page_id: Id,
    pub project_id: Id,
    pub view_count: i64,
    pub unique_visitors: i64,
    pub last_viewed_at: Option<DateTime<Utc>>,
}

/// Project analytics summary
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProjectAnalyticsSummary {
    pub project_id: Id,
    pub total_views: i64,
    pub unique_visitors: i64,
    pub total_pages: i64,
    pub most_viewed_pages: Vec<PageViewAnalytics>,
    pub recent_activity: Vec<AnalyticsEventResponse>,
}

/// Track page view request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct TrackPageViewRequest {
    pub page_id: Id,
    pub project_id: Id,
    #[serde(default)]
    pub referrer: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub ip_address: Option<String>,
}

/// Time series analytics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TimeSeriesAnalytics {
    pub date: String,
    pub count: i64,
}

/// Analytics dashboard response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AnalyticsDashboardResponse {
    pub total_events: i64,
    pub events_by_type: std::collections::HashMap<String, i64>,
    pub time_series: Vec<TimeSeriesAnalytics>,
}
