//! Integration entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, Timestamp};

/// Integration provider types
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "\"IntegrationProvider\"", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IntegrationProvider {
    Slack,
    Discord,
    MicrosoftTeams,
    Zapier,
    Make,
    Webhook,
}

/// Integration event status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "IntegrationEventStatus", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IntegrationEventStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// Project integration entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProjectIntegration {
    pub id: Id,
    pub project_id: Id,
    pub provider: IntegrationProvider,
    pub name: String,
    pub config: serde_json::Value,
    pub webhook_url: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Project integration response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProjectIntegrationResponse {
    pub id: Id,
    pub project_id: Id,
    pub provider: IntegrationProvider,
    pub name: String,
    pub config: serde_json::Value,
    pub webhook_url: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ProjectIntegration> for ProjectIntegrationResponse {
    fn from(integration: ProjectIntegration) -> Self {
        Self {
            id: integration.id,
            project_id: integration.project_id,
            provider: integration.provider,
            name: integration.name,
            config: integration.config,
            webhook_url: integration.webhook_url,
            is_active: integration.is_active,
            created_at: integration.created_at,
            updated_at: integration.updated_at,
        }
    }
}

/// Create project integration request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateProjectIntegrationRequest {
    pub project_id: Id,
    pub provider: IntegrationProvider,
    pub name: String,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

/// Update project integration request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateProjectIntegrationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

/// Integration audit event entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntegrationAuditEvent {
    pub id: Id,
    pub integration_id: Id,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Integration audit event response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntegrationAuditEventResponse {
    pub id: Id,
    pub integration_id: Id,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<IntegrationAuditEvent> for IntegrationAuditEventResponse {
    fn from(event: IntegrationAuditEvent) -> Self {
        Self {
            id: event.id,
            integration_id: event.integration_id,
            event_type: event.event_type,
            payload: event.payload,
            status: event.status,
            error_message: event.error_message,
            created_at: event.created_at,
        }
    }
}

/// Integration confirmation entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntegrationConfirmation {
    pub id: Id,
    pub integration_id: Id,
    pub confirmation_token: String,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Integration confirmation response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntegrationConfirmationResponse {
    pub id: Id,
    pub integration_id: Id,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<IntegrationConfirmation> for IntegrationConfirmationResponse {
    fn from(confirmation: IntegrationConfirmation) -> Self {
        Self {
            id: confirmation.id,
            integration_id: confirmation.integration_id,
            confirmed_at: confirmation.confirmed_at,
            created_at: confirmation.created_at,
        }
    }
}

/// Integration webhook delivery entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntegrationWebhookDelivery {
    pub id: Id,
    pub integration_id: Id,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub response_status: Option<i32>,
    pub error_message: Option<String>,
    pub attempts: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Integration webhook delivery response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntegrationWebhookDeliveryResponse {
    pub id: Id,
    pub integration_id: Id,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub response_status: Option<i32>,
    pub error_message: Option<String>,
    pub attempts: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<IntegrationWebhookDelivery> for IntegrationWebhookDeliveryResponse {
    fn from(delivery: IntegrationWebhookDelivery) -> Self {
        Self {
            id: delivery.id,
            integration_id: delivery.integration_id,
            event_type: delivery.event_type,
            payload: delivery.payload,
            status: delivery.status,
            response_status: delivery.response_status,
            error_message: delivery.error_message,
            attempts: delivery.attempts,
            created_at: delivery.created_at,
            updated_at: delivery.updated_at,
        }
    }
}

/// Integration idempotency record entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntegrationIdempotencyRecord {
    pub id: Id,
    pub integration_id: Id,
    pub request_id: String,
    pub processed_at: DateTime<Utc>,
}

/// Integration idempotency record response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntegrationIdempotencyRecordResponse {
    pub id: Id,
    pub integration_id: Id,
    pub request_id: String,
    pub processed_at: DateTime<Utc>,
}

impl From<IntegrationIdempotencyRecord> for IntegrationIdempotencyRecordResponse {
    fn from(record: IntegrationIdempotencyRecord) -> Self {
        Self {
            id: record.id,
            integration_id: record.integration_id,
            request_id: record.request_id,
            processed_at: record.processed_at,
        }
    }
}

/// List integrations query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListIntegrationsQuery {
    #[serde(default)]
    pub project_id: Option<Id>,
    #[serde(default)]
    pub provider: Option<IntegrationProvider>,
    #[serde(default)]
    pub is_active: Option<bool>,
}
