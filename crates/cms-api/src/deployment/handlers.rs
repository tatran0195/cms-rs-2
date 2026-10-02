//! Deployment handlers
//!
//! This module contains the actual implementation of deployment handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::deployment::DeploymentService;
use cms_entity::{
    common::{Id, PaginatedResponse},
    deployment::{CreateDeploymentRequest, DeploymentResponse, ListDeploymentsQuery},
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List deployments for a project
///
/// Returns a paginated list of deployments filtered by project and status.
#[utoipa::path(
    get,
    path = "/deployments",
    tag = "deployments",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Query, description = "Filter by project ID"),
        ("status", Query, description = "Filter by deployment status"),
        ("limit", Query, description = "Number of items per page"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of deployments", body = PaginatedResponse<DeploymentResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_deployments_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListDeploymentsQuery>,
) -> Result<Json<PaginatedResponse<DeploymentResponse>>, AppError> {
    let project_id = query.project_id.as_deref().unwrap_or("");
    let limit = query.limit.unwrap_or(20) as u64;
    let offset = query.offset.unwrap_or(0) as u64;
    let page = offset.checked_div(limit).unwrap_or(0).saturating_add(1);
    let result = DeploymentService::list_deployments(
        &state.biz_context,
        &auth.user.id,
        project_id,
        page,
        limit,
    )
    .await?;

    Ok(Json(result))
}

/// Create a new deployment
///
/// Triggers a deployment for a project.
#[utoipa::path(
    post,
    path = "/deployments",
    tag = "deployments",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreateDeploymentRequest,
    responses(
        (status = 200, description = "Deployment created successfully", body = DeploymentResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_deployment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateDeploymentRequest>,
) -> Result<Json<DeploymentResponse>, AppError> {
    let project_id = request.project_id.clone();
    let deployment = DeploymentService::create_deployment(
        &state.biz_context,
        &auth.user.id,
        &project_id,
        request,
    )
    .await?;

    Ok(Json(deployment))
}

/// Get a specific deployment by ID
///
/// Retrieves a deployment by its unique identifier.
#[utoipa::path(
    get,
    path = "/deployments/{deployment_id}",
    tag = "deployments",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("deployment_id", Path, description = "The ID of the deployment to retrieve"),
    ),
    responses(
        (status = 200, description = "Deployment found", body = DeploymentResponse),
        (status = 404, description = "Deployment not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_deployment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(deployment_id): Path<Id>,
) -> Result<Json<DeploymentResponse>, AppError> {
    let deployment =
        DeploymentService::get_deployment(&state.biz_context, &auth.user.id, &deployment_id)
            .await?;

    Ok(Json(deployment))
}

/// Get deployment logs
///
/// Retrieves the logs for a specific deployment.
#[utoipa::path(
    get,
    path = "/deployments/{deployment_id}/logs",
    tag = "deployments",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("deployment_id", Path, description = "The ID of the deployment"),
    ),
    responses(
        (status = 200, description = "Deployment logs", body = serde_json::Value),
        (status = 404, description = "Deployment not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_deployment_logs_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(deployment_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    let logs =
        DeploymentService::get_deployment_logs(&state.biz_context, &auth.user.id, &deployment_id)
            .await?;

    Ok(Json(serde_json::json!({"logs": logs})))
}

/// Retry a deployment
///
/// Retries a failed or completed deployment.
#[utoipa::path(
    post,
    path = "/deployments/{deployment_id}/retry",
    tag = "deployments",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("deployment_id", Path, description = "The ID of the deployment to retry"),
    ),
    responses(
        (status = 200, description = "Deployment retry initiated", body = DeploymentResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Deployment not found"),
    )
)]
pub async fn retry_deployment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(deployment_id): Path<Id>,
) -> Result<Json<DeploymentResponse>, AppError> {
    let deployment =
        DeploymentService::retry_deployment(&state.biz_context, &auth.user.id, &deployment_id)
            .await?;

    Ok(Json(deployment))
}

/// Cancel a deployment
///
/// Cancels an in-progress deployment.
#[utoipa::path(
    post,
    path = "/deployments/{deployment_id}/cancel",
    tag = "deployments",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("deployment_id", Path, description = "The ID of the deployment to cancel"),
    ),
    responses(
        (status = 200, description = "Deployment cancelled", body = DeploymentResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Deployment not found"),
        (status = 400, description = "Cannot cancel a completed deployment"),
    )
)]
pub async fn cancel_deployment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(deployment_id): Path<Id>,
) -> Result<Json<DeploymentResponse>, AppError> {
    let deployment =
        DeploymentService::cancel_deployment(&state.biz_context, &auth.user.id, &deployment_id)
            .await?;

    Ok(Json(deployment))
}
