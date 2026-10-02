//! Admin handlers
//!
//! This module contains the actual implementation of admin handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::org::OrgService;
use cms_entity::{
    common::{Id, PaginatedResponse},
    org::OrganizationResponse,
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List all organizations (admin only)
///
/// Returns a paginated list of all organizations in the system.
/// Requires admin privileges.
#[utoipa::path(
    get,
    path = "/admin/organizations",
    tag = "admin",
    security(
        ("bearerAuth" = ["admin"]),
        ("apiKeyAuth" = ["admin"]),
        ("cookieAuth" = ["admin"]),
    ),
    params(
        ("page", Query, description = "Page number"),
        ("page_size", Query, description = "Number of items per page"),
    ),
    responses(
        (status = 200, description = "List of all organizations", body = PaginatedResponse<OrganizationResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privileges required"),
    )
)]
pub async fn list_all_organizations_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<PaginatedResponse<OrganizationResponse>>, AppError> {
    state
        .biz_context
        .authz
        .require_system_admin(&auth.user.id)
        .await?;

    let page = query.get("page").and_then(|v| v.as_u64()).unwrap_or(1);
    let page_size = query
        .get("page_size")
        .and_then(|v| v.as_u64())
        .unwrap_or(20);

    let orgs =
        OrgService::list_all_organizations(&state.biz_context, &auth.user.id, page, page_size)
            .await?;

    Ok(Json(orgs))
}

/// Get organization statistics (admin only)
///
/// Returns statistics for a specific organization.
/// Requires admin privileges.
#[utoipa::path(
    get,
    path = "/admin/organizations/{org_id}/stats",
    tag = "admin",
    security(
        ("bearerAuth" = ["admin"]),
        ("apiKeyAuth" = ["admin"]),
        ("cookieAuth" = ["admin"]),
    ),
    params(
        ("org_id", Path, description = "The ID of the organization"),
    ),
    responses(
        (status = 200, description = "Organization statistics", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privileges required"),
        (status = 404, description = "Organization not found"),
    )
)]
pub async fn get_organization_stats_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(org_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .biz_context
        .authz
        .require_system_admin(&auth.user.id)
        .await?;

    let stats =
        cms_biz::analytics::AnalyticsService::get_organization_stats(&state.biz_context, &org_id)
            .await?;

    Ok(Json(serde_json::json!(stats)))
}

/// Get system statistics (admin only)
///
/// Returns overall system statistics.
/// Requires admin privileges.
#[utoipa::path(
    get,
    path = "/admin/system/stats",
    tag = "admin",
    security(
        ("bearerAuth" = ["admin"]),
        ("apiKeyAuth" = ["admin"]),
        ("cookieAuth" = ["admin"]),
    ),
    responses(
        (status = 200, description = "System statistics", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privileges required"),
    )
)]
pub async fn get_system_stats_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .biz_context
        .authz
        .require_system_admin(&auth.user.id)
        .await?;

    let stats = cms_biz::analytics::AnalyticsService::get_system_stats(&state.biz_context).await?;

    Ok(Json(serde_json::json!(stats)))
}

/// Get system health (admin only)
///
/// Returns the current health status of the system.
/// Requires admin privileges.
#[utoipa::path(
    get,
    path = "/admin/system/health",
    tag = "admin",
    security(
        ("bearerAuth" = ["admin"]),
        ("apiKeyAuth" = ["admin"]),
        ("cookieAuth" = ["admin"]),
    ),
    responses(
        (status = 200, description = "System health status", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privileges required"),
    )
)]
pub async fn get_system_health_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .biz_context
        .authz
        .require_system_admin(&auth.user.id)
        .await?;

    let health =
        cms_biz::platform_event::PlatformEventService::get_system_health(&state.biz_context)
            .await?;

    Ok(Json(serde_json::json!(health)))
}
