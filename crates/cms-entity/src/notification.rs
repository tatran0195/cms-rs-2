//! Notification entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, Timestamp};

/// Notification type
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "\"NotificationType\"", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationType {
    Comment,
    Invitation,
    Mention,
    System,
    ExportComplete,
    DeploymentComplete,
}

/// Notification status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum NotificationStatus {
    Unread,
    Read,
    Archived,
    // Uppercase aliases for backward compatibility
    UNREAD,
    READ,
    ARCHIVED,
}

impl sqlx::Type<sqlx::Postgres> for NotificationStatus {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("\"NotificationStatus\"")
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for NotificationStatus {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        match s {
            "UNREAD" | "unread" => Ok(NotificationStatus::Unread),
            "READ" | "read" => Ok(NotificationStatus::Read),
            "ARCHIVED" | "archived" => Ok(NotificationStatus::Archived),
            other => Err(format!("unknown NotificationStatus: {}", other).into()),
        }
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Postgres> for NotificationStatus {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = match self {
            NotificationStatus::Unread | NotificationStatus::UNREAD => "UNREAD",
            NotificationStatus::Read | NotificationStatus::READ => "READ",
            NotificationStatus::Archived | NotificationStatus::ARCHIVED => "ARCHIVED",
        };
        <&str as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&s, buf)
    }
}

/// Notification entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Notification {
    pub id: Id,
    pub user_id: Id,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub data: serde_json::Value,
    pub status: NotificationStatus,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Notification response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NotificationResponse {
    pub id: Id,
    pub user_id: Id,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub data: serde_json::Value,
    pub status: NotificationStatus,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Notification> for NotificationResponse {
    fn from(notification: Notification) -> Self {
        Self {
            id: notification.id,
            user_id: notification.user_id,
            notification_type: notification.notification_type,
            title: notification.title,
            message: notification.message,
            data: notification.data,
            status: notification.status,
            read_at: notification.read_at,
            created_at: notification.created_at,
            updated_at: notification.updated_at,
        }
    }
}

/// Create notification request (internal use, not from API)
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateNotificationRequest {
    pub user_id: Id,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Mark notification as read request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct MarkNotificationReadRequest {
    #[serde(default)]
    pub notification_ids: Vec<Id>,
}

/// Mark all notifications as read request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct MarkAllNotificationsReadRequest {
    #[serde(default)]
    pub notification_type: Option<NotificationType>,
}

/// Archive notification request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ArchiveNotificationRequest {
    pub notification_id: Id,
}

/// List notifications query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListNotificationsQuery {
    #[serde(default)]
    pub status: Option<NotificationStatus>,
    #[serde(default)]
    pub notification_type: Option<NotificationType>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

/// Notification count response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NotificationCountResponse {
    pub total: i64,
    pub unread: i64,
    pub by_type: std::collections::HashMap<String, i64>,
}
