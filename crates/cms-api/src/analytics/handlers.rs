//! Analytics handlers
//!
//! This module contains the actual implementation of analytics handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::analytics::AnalyticsService;
use cms_entity::{
    analytics::{
        AnalyticsDashboardResponse, AnalyticsEventResponse, AnalyticsQueryRequest,
        AnalyticsQueryResponse, ListAnalyticsEventsQuery, TrackAnalyticsEventRequest,
    },
    common::{Id, PaginatedResponse},
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;

use crate::auth::AuthExtractor;

/// Track an analytics event
///
/// Tracks a user interaction or page view event.
/// This endpoint can be called without authentication for public pages.
#[utoipa::path(
    post,
    path = "/analytics/track",
    tag = "analytics",
    request_body = TrackAnalyticsEventRequest,
    responses(
        (status = 200, description = "Event tracked successfully", body = AnalyticsEventResponse),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn track_event_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TrackAnalyticsEventRequest>,
) -> Result<Json<AnalyticsEventResponse>, AppError> {
    let event = AnalyticsService::track_event(&state.biz_context, request).await?;

    Ok(Json(event))
}

/// List analytics events
///
/// Returns a paginated list of analytics events for the authenticated user.
#[utoipa::path(
    get,
    path = "/analytics/events",
    tag = "analytics",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Query, description = "Filter by project ID"),
        ("event_type", Query, description = "Filter by event type"),
        ("start_date", Query, description = "Filter by start date"),
        ("end_date", Query, description = "Filter by end date"),
        ("limit", Query, description = "Number of items per page"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of analytics events", body = PaginatedResponse<AnalyticsEventResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_analytics_events_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListAnalyticsEventsQuery>,
) -> Result<Json<PaginatedResponse<AnalyticsEventResponse>>, AppError> {
    let result = AnalyticsService::list_events(&state.biz_context, &auth.user.id, query).await?;

    Ok(Json(result))
}

/// Query analytics
///
/// Executes a custom analytics query.
#[utoipa::path(
    post,
    path = "/analytics/query",
    tag = "analytics",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = AnalyticsQueryRequest,
    responses(
        (status = 200, description = "Query results", body = AnalyticsQueryResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn query_analytics_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<AnalyticsQueryRequest>,
) -> Result<Json<AnalyticsQueryResponse>, AppError> {
    let result =
        AnalyticsService::query_analytics(&state.biz_context, &auth.user.id, request).await?;

    Ok(Json(result))
}

/// Get analytics dashboard for a project
///
/// Returns a pre-aggregated analytics dashboard for a project.
#[utoipa::path(
    get,
    path = "/analytics/dashboard/{project_id}",
    tag = "analytics",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Path, description = "The ID of the project"),
    ),
    responses(
        (status = 200, description = "Analytics dashboard", body = AnalyticsDashboardResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Project not found"),
    )
)]
pub async fn get_analytics_dashboard_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<Id>,
) -> Result<Json<AnalyticsDashboardResponse>, AppError> {
    let dashboard =
        AnalyticsService::get_dashboard(&state.biz_context, &auth.user.id, &project_id).await?;

    Ok(Json(dashboard))
}

/// Get page views for a page
///
/// Returns the number of views for a specific page.
#[utoipa::path(
    get,
    path = "/analytics/page-views/{page_id}",
    tag = "analytics",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("page_id", Path, description = "The ID of the page"),
    ),
    responses(
        (status = 200, description = "Page view count", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Page not found"),
    )
)]
pub async fn get_page_views_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(page_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    let views =
        AnalyticsService::get_page_views(&state.biz_context, &auth.user.id, &page_id).await?;

    Ok(Json(serde_json::json!(views)))
}
