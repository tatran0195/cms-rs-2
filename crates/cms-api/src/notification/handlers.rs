//! Notification handlers
//!
//! This module contains the actual implementation of notification handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::notification::NotificationService;
use cms_entity::{
    common::{Id, PaginatedResponse},
    notification::{
        ArchiveNotificationRequest, ListNotificationsQuery, MarkNotificationReadRequest,
        NotificationCountResponse, NotificationResponse,
    },
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List notifications for the authenticated user
///
/// Returns a paginated list of notifications filtered by status and type.
#[utoipa::path(
    get,
    path = "/notifications",
    tag = "notifications",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("status", Query, description = "Filter by notification status"),
        ("notification_type", Query, description = "Filter by notification type"),
        ("limit", Query, description = "Number of items per page"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of notifications", body = PaginatedResponse<NotificationResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_notifications_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListNotificationsQuery>,
) -> Result<Json<PaginatedResponse<NotificationResponse>>, AppError> {
    let result = NotificationService::list_notifications(
        &state.biz_context,
        &auth.user.id,
        query.status,
        query.limit.unwrap_or(1) as u64,
        query.offset.unwrap_or(20) as u64,
    )
    .await?;

    Ok(Json(result))
}

/// Get a specific notification by ID
///
/// Retrieves a notification by its unique identifier.
#[utoipa::path(
    get,
    path = "/notifications/{notification_id}",
    tag = "notifications",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("notification_id", Path, description = "The ID of the notification to retrieve"),
    ),
    responses(
        (status = 200, description = "Notification found", body = NotificationResponse),
        (status = 404, description = "Notification not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_notification_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(notification_id): Path<Id>,
) -> Result<Json<NotificationResponse>, AppError> {
    let notification =
        NotificationService::get_notification(&state.biz_context, &auth.user.id, &notification_id)
            .await?;

    Ok(Json(notification))
}

/// Mark notifications as read
///
/// Marks one or more notifications as read.
#[utoipa::path(
    post,
    path = "/notifications/mark-read",
    tag = "notifications",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = MarkNotificationReadRequest,
    responses(
        (status = 200, description = "Notifications marked as read", body = serde_json::Value),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn mark_notification_read_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<MarkNotificationReadRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    NotificationService::mark_notifications_read(
        &state.biz_context,
        &auth.user.id,
        &request.notification_ids,
    )
    .await?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// Mark all notifications as read
///
/// Marks all notifications for the authenticated user as read.
#[utoipa::path(
    post,
    path = "/notifications/mark-all-read",
    tag = "notifications",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    responses(
        (status = 200, description = "All notifications marked as read", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn mark_all_notifications_read_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<serde_json::Value>, AppError> {
    NotificationService::mark_all_notifications_read(&state.biz_context, &auth.user.id).await?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// Archive a notification
///
/// Archives a notification, removing it from the active list.
#[utoipa::path(
    post,
    path = "/notifications/{notification_id}/archive",
    tag = "notifications",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("notification_id", Path, description = "The ID of the notification to archive"),
    ),
    request_body = ArchiveNotificationRequest,
    responses(
        (status = 200, description = "Notification archived", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Notification not found"),
    )
)]
pub async fn archive_notification_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(notification_id): Path<Id>,
    Json(request): Json<ArchiveNotificationRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    NotificationService::archive_notification(&state.biz_context, &auth.user.id, &notification_id)
        .await?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// Get notification count
///
/// Returns the count of unread notifications for the authenticated user.
#[utoipa::path(
    get,
    path = "/notifications/count",
    tag = "notifications",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    responses(
        (status = 200, description = "Notification count", body = NotificationCountResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_notification_count_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<NotificationCountResponse>, AppError> {
    let count =
        NotificationService::get_notification_count(&state.biz_context, &auth.user.id).await?;

    Ok(Json(count))
}
