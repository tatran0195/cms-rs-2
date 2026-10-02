//! Platform event handlers
//!
//! This module contains the actual implementation of platform event handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::platform_event::{FunnelEventType, PlatformEventService};
use cms_entity::{
    common::{Id, PaginatedResponse},
    platform_event::{ListPlatformEventsQuery, PlatformEventResponse},
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List platform events
///
/// Returns a paginated list of platform events filtered by various criteria.
#[utoipa::path(
    get,
    path = "/platform-events",
    tag = "platform-events",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("organization_id", Query, description = "Filter by organization ID"),
        ("user_id", Query, description = "Filter by user ID"),
        ("event_type", Query, description = "Filter by event type"),
        ("start_date", Query, description = "Filter by start date"),
        ("end_date", Query, description = "Filter by end date"),
        ("limit", Query, description = "Number of items per page"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of platform events", body = PaginatedResponse<PlatformEventResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_platform_events_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListPlatformEventsQuery>,
) -> Result<Json<PaginatedResponse<PlatformEventResponse>>, AppError> {
    let org_id = query.organization_id.as_deref().unwrap_or("");
    let result = PlatformEventService::list_events(
        &state.biz_context,
        &auth.user.id,
        org_id,
        query.event_type.as_deref(),
        query.limit.unwrap_or(1) as u64,
        query.offset.unwrap_or(20) as u64,
    )
    .await?;

    Ok(Json(result))
}

/// Create platform event (internal use)
///
/// Creates a new platform event. This is typically called internally.
#[utoipa::path(
    post,
    path = "/platform-events",
    tag = "platform-events",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Platform event created", body = PlatformEventResponse),
        (status = 400, description = "Bad request - event_type is required"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn create_platform_event_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<PlatformEventResponse>, AppError> {
    // Identity comes only from the authenticated session/API key; request-body
    // user IDs are intentionally ignored.
    let event_type: String = serde_json::from_value(
        request
            .get("event_type")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|_| AppError::BadRequest("Invalid event_type".to_string()))?;
    if event_type.is_empty()
        || event_type.len() > 100
        || !event_type
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(AppError::BadRequest("Invalid event_type".to_string()));
    }
    if FunnelEventType::is_reserved_event_type(&event_type) {
        return Err(AppError::Forbidden);
    }

    let organization_id: Option<String> = serde_json::from_value(
        request
            .get("organization_id")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|_| AppError::BadRequest("Invalid organization_id".to_string()))?;
    if let Some(organization_id) = organization_id.as_deref() {
        state
            .biz_context
            .authz
            .require_org_member(&auth.user.id, organization_id)
            .await?;
    }

    let metadata: serde_json::Value = request
        .get("metadata")
        .cloned()
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    if !metadata.is_object() || serde_json::to_vec(&metadata)?.len() > 16 * 1024 {
        return Err(AppError::BadRequest(
            "metadata must be an object no larger than 16 KiB".to_string(),
        ));
    }
    let user_id = auth.user.id.clone();

    let event = PlatformEventService::create_event(
        &state.biz_context,
        organization_id.as_deref(),
        Some(&user_id),
        cms_entity::platform_event::CreatePlatformEventRequest {
            organization_id: organization_id.clone(),
            user_id: Some(user_id.clone()),
            event_type,
            metadata,
        },
    )
    .await?;

    Ok(Json(event))
}

/// Get a specific platform event
///
/// Retrieves a platform event by its unique identifier.
#[utoipa::path(
    get,
    path = "/platform-events/{event_id}",
    tag = "platform-events",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("event_id", Path, description = "The ID of the platform event to retrieve"),
    ),
    responses(
        (status = 200, description = "Platform event found", body = PlatformEventResponse),
        (status = 404, description = "Platform event not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_platform_event_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(event_id): Path<Id>,
) -> Result<Json<PlatformEventResponse>, AppError> {
    let event =
        PlatformEventService::get_event(&state.biz_context, &auth.user.id, &event_id).await?;

    Ok(Json(event))
}
