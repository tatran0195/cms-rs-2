//! Usage handlers
//!
//! This module contains the actual implementation of usage handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::usage::UsageService;
use cms_entity::{
    common::Id,
    usage::{
        OrganizationUsagePlanResponse, TrackAnalyticsEventRequest, UsageEntitlementResponse,
        UsageMeterResponse, UsagePlanResponse,
    },
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List usage plans
///
/// Returns a list of all available usage plans.
#[utoipa::path(
    get,
    path = "/usage/plans",
    tag = "usage",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    responses(
        (status = 200, description = "List of usage plans", body = Vec<UsagePlanResponse>),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn list_usage_plans_handler(
    State(state): State<Arc<AppState>>,
    _auth: AuthExtractor,
) -> Result<Json<Vec<UsagePlanResponse>>, AppError> {
    let plans = UsageService::list_usage_plans(&state.biz_context, 1, 100).await?;

    Ok(Json(plans.data))
}

/// Get a specific usage plan
///
/// Retrieves a usage plan by its unique identifier.
#[utoipa::path(
    get,
    path = "/usage/plans/{plan_id}",
    tag = "usage",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("plan_id", Path, description = "The ID of the usage plan to retrieve"),
    ),
    responses(
        (status = 200, description = "Usage plan found", body = UsagePlanResponse),
        (status = 404, description = "Usage plan not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_usage_plan_handler(
    State(state): State<Arc<AppState>>,
    _auth: AuthExtractor,
    Path(plan_id): Path<Id>,
) -> Result<Json<UsagePlanResponse>, AppError> {
    let plan = UsageService::get_usage_plan(&state.biz_context, &plan_id).await?;

    Ok(Json(plan))
}

/// List usage meters
///
/// Returns a list of all usage meters.
#[utoipa::path(
    get,
    path = "/usage/meters",
    tag = "usage",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    responses(
        (status = 200, description = "List of usage meters", body = Vec<UsageMeterResponse>),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn list_usage_meters_handler(
    State(state): State<Arc<AppState>>,
    _auth: AuthExtractor,
) -> Result<Json<Vec<UsageMeterResponse>>, AppError> {
    let meters = UsageService::list_usage_meters(&state.biz_context, 1, 100).await?;

    Ok(Json(meters.data))
}

/// Get a specific usage meter
///
/// Retrieves a usage meter by its unique identifier.
#[utoipa::path(
    get,
    path = "/usage/meters/{meter_id}",
    tag = "usage",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("meter_id", Path, description = "The ID of the usage meter to retrieve"),
    ),
    responses(
        (status = 200, description = "Usage meter found", body = UsageMeterResponse),
        (status = 404, description = "Usage meter not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_usage_meter_handler(
    State(state): State<Arc<AppState>>,
    _auth: AuthExtractor,
    Path(meter_id): Path<Id>,
) -> Result<Json<UsageMeterResponse>, AppError> {
    let meter = UsageService::get_usage_meter(&state.biz_context, &meter_id).await?;

    Ok(Json(meter))
}

/// List usage entitlements
///
/// Returns a list of all usage entitlements for the authenticated user.
#[utoipa::path(
    get,
    path = "/usage/entitlements",
    tag = "usage",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    responses(
        (status = 200, description = "List of usage entitlements", body = Vec<UsageEntitlementResponse>),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn list_usage_entitlements_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<Vec<UsageEntitlementResponse>>, AppError> {
    let entitlements =
        UsageService::list_usage_entitlements(&state.biz_context, &auth.user.id).await?;

    Ok(Json(entitlements))
}

/// Get organization usage plan
///
/// Returns the usage plan for a specific organization.
#[utoipa::path(
    get,
    path = "/usage/organizations/{org_id}/plan",
    tag = "usage",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("org_id", Path, description = "The ID of the organization"),
    ),
    responses(
        (status = 200, description = "Organization usage plan", body = OrganizationUsagePlanResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Organization not found"),
    )
)]
pub async fn get_organization_usage_plan_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(org_id): Path<Id>,
) -> Result<Json<OrganizationUsagePlanResponse>, AppError> {
    let plan =
        UsageService::get_organization_usage_plan(&state.biz_context, &auth.user.id, &org_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Organization usage plan not found".to_string()))?;

    Ok(Json(plan))
}

/// Update organization usage plan
///
/// Updates the usage plan for a specific organization.
#[utoipa::path(
    put,
    path = "/usage/organizations/{org_id}/plan",
    tag = "usage",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("org_id", Path, description = "The ID of the organization"),
    ),
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Organization usage plan updated", body = OrganizationUsagePlanResponse),
        (status = 400, description = "Bad request - usage_plan_id is required"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Organization not found"),
    )
)]
pub async fn update_organization_usage_plan_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(org_id): Path<Id>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<OrganizationUsagePlanResponse>, AppError> {
    let usage_plan_id: String = serde_json::from_value(
        request
            .get("usage_plan_id")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|_| AppError::BadRequest("Invalid usage_plan_id".to_string()))?;

    let plan = UsageService::update_organization_usage_plan(
        &state.biz_context,
        &auth.user.id,
        &org_id,
        &usage_plan_id,
    )
    .await?;

    Ok(Json(plan))
}

/// Track a usage event
///
/// Tracks a usage event for metering and billing purposes.
#[utoipa::path(
    post,
    path = "/usage/track",
    tag = "usage",
    request_body = TrackAnalyticsEventRequest,
    responses(
        (status = 200, description = "Usage event tracked", body = serde_json::Value),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn track_usage_event_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TrackAnalyticsEventRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    UsageService::track_usage_event(&state.biz_context, request).await?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// Get usage summary for an organization
///
/// Returns a summary of usage for a specific organization.
#[utoipa::path(
    get,
    path = "/usage/organizations/{org_id}/summary",
    tag = "usage",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("org_id", Path, description = "The ID of the organization"),
    ),
    responses(
        (status = 200, description = "Usage summary", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Organization not found"),
    )
)]
pub async fn get_usage_summary_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(org_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    let summary =
        UsageService::get_usage_summary(&state.biz_context, &auth.user.id, &org_id).await?;

    Ok(Json(serde_json::json!(summary)))
}
