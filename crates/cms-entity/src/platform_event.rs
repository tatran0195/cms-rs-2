//! Platform event entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, Timestamp};

/// Platform event entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PlatformEvent {
    pub id: Id,
    pub organization_id: Option<Id>,
    pub user_id: Option<Id>,
    pub event_type: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Platform event response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PlatformEventResponse {
    pub id: Id,
    pub organization_id: Option<Id>,
    pub user_id: Option<Id>,
    pub event_type: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl From<PlatformEvent> for PlatformEventResponse {
    fn from(event: PlatformEvent) -> Self {
        Self {
            id: event.id,
            organization_id: event.organization_id,
            user_id: event.user_id,
            event_type: event.event_type,
            metadata: event.metadata,
            created_at: event.created_at,
        }
    }
}

/// Create platform event request (internal use)
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreatePlatformEventRequest {
    #[serde(default)]
    pub organization_id: Option<Id>,
    #[serde(default)]
    pub user_id: Option<Id>,
    pub event_type: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// List platform events query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListPlatformEventsQuery {
    #[serde(default)]
    pub organization_id: Option<Id>,
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

/// Platform event type constants
pub mod event_types {
    pub const ORG_CREATED: &str = "organization.created";
    pub const ORG_DELETED: &str = "organization.deleted";
    pub const PROJECT_CREATED: &str = "project.created";
    pub const PROJECT_DELETED: &str = "project.deleted";
    pub const USER_INVITED: &str = "user.invited";
    pub const USER_JOINED: &str = "user.joined";
    pub const USER_LEFT: &str = "user.left";
    pub const DEPLOYMENT_CREATED: &str = "deployment.created";
    pub const DEPLOYMENT_COMPLETED: &str = "deployment.completed";
    pub const DEPLOYMENT_FAILED: &str = "deployment.failed";
    pub const EXPORT_CREATED: &str = "export.created";
    pub const EXPORT_COMPLETED: &str = "export.completed";
    pub const GIT_SYNC_STARTED: &str = "git.sync.started";
    pub const GIT_SYNC_COMPLETED: &str = "git.sync.completed";
    pub const INTEGRATION_CREATED: &str = "integration.created";
    pub const INTEGRATION_DELETED: &str = "integration.deleted";
}
