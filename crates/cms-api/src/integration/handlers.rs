//! Integration handlers
//!
//! This module contains the actual implementation of integration handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::integration::IntegrationService;
use cms_entity::{
    common::Id,
    integration::{
        CreateProjectIntegrationRequest, ListIntegrationsQuery, ProjectIntegrationResponse,
        UpdateProjectIntegrationRequest,
    },
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List integrations for a project
///
/// Returns all integrations filtered by project, provider, and active status.
#[utoipa::path(
    get,
    path = "/integrations",
    tag = "integrations",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Query, description = "Filter by project ID"),
        ("provider", Query, description = "Filter by provider name"),
        ("is_active", Query, description = "Filter by active status"),
    ),
    responses(
        (status = 200, description = "List of integrations", body = Vec<ProjectIntegrationResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_integrations_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListIntegrationsQuery>,
) -> Result<Json<Vec<ProjectIntegrationResponse>>, AppError> {
    let project_id = query.project_id.as_deref().unwrap_or("");
    let integrations =
        IntegrationService::list_integrations(&state.biz_context, &auth.user.id, project_id)
            .await?;

    Ok(Json(integrations))
}

/// Create a new integration
///
/// Creates a new third-party integration for a project.
#[utoipa::path(
    post,
    path = "/integrations",
    tag = "integrations",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreateProjectIntegrationRequest,
    responses(
        (status = 200, description = "Integration created successfully", body = ProjectIntegrationResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_integration_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateProjectIntegrationRequest>,
) -> Result<Json<ProjectIntegrationResponse>, AppError> {
    let integration =
        IntegrationService::create_integration(&state.biz_context, &auth.user.id, request).await?;

    Ok(Json(integration))
}

/// Get an integration by ID
///
/// Retrieves an integration by its unique identifier.
#[utoipa::path(
    get,
    path = "/integrations/{integration_id}",
    tag = "integrations",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("integration_id", Path, description = "The ID of the integration to retrieve"),
    ),
    responses(
        (status = 200, description = "Integration found", body = ProjectIntegrationResponse),
        (status = 404, description = "Integration not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_integration_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(integration_id): Path<Id>,
) -> Result<Json<ProjectIntegrationResponse>, AppError> {
    let integration =
        IntegrationService::get_integration(&state.biz_context, &auth.user.id, &integration_id)
            .await?;

    Ok(Json(integration))
}

/// Update an existing integration
///
/// Updates an integration by its ID with the provided configuration.
#[utoipa::path(
    put,
    path = "/integrations/{integration_id}",
    tag = "integrations",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("integration_id", Path, description = "The ID of the integration to update"),
    ),
    request_body = UpdateProjectIntegrationRequest,
    responses(
        (status = 200, description = "Integration updated successfully", body = ProjectIntegrationResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Integration not found"),
    )
)]
pub async fn update_integration_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(integration_id): Path<Id>,
    Json(request): Json<UpdateProjectIntegrationRequest>,
) -> Result<Json<ProjectIntegrationResponse>, AppError> {
    let integration = IntegrationService::update_integration(
        &state.biz_context,
        &auth.user.id,
        &integration_id,
        request,
    )
    .await?;

    Ok(Json(integration))
}

/// Delete an integration
///
/// Permanently deletes an integration by its ID.
#[utoipa::path(
    delete,
    path = "/integrations/{integration_id}",
    tag = "integrations",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("integration_id", Path, description = "The ID of the integration to delete"),
    ),
    responses(
        (status = 200, description = "Integration deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Integration not found"),
    )
)]
pub async fn delete_integration_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(integration_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    IntegrationService::delete_integration(&state.biz_context, &auth.user.id, &integration_id)
        .await?;

    Ok(Json(
        serde_json::json!({"success": true, "id": integration_id}),
    ))
}

/// Enable an integration
///
/// Activates an integration that was previously disabled.
#[utoipa::path(
    post,
    path = "/integrations/{integration_id}/enable",
    tag = "integrations",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("integration_id", Path, description = "The ID of the integration to enable"),
    ),
    responses(
        (status = 200, description = "Integration enabled successfully", body = ProjectIntegrationResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Integration not found"),
    )
)]
pub async fn enable_integration_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(integration_id): Path<Id>,
) -> Result<Json<ProjectIntegrationResponse>, AppError> {
    let integration =
        IntegrationService::enable_integration(&state.biz_context, &auth.user.id, &integration_id)
            .await?;

    Ok(Json(integration))
}

/// Disable an integration
///
/// Deactivates an integration without deleting it.
#[utoipa::path(
    post,
    path = "/integrations/{integration_id}/disable",
    tag = "integrations",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("integration_id", Path, description = "The ID of the integration to disable"),
    ),
    responses(
        (status = 200, description = "Integration disabled successfully", body = ProjectIntegrationResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Integration not found"),
    )
)]
pub async fn disable_integration_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(integration_id): Path<Id>,
) -> Result<Json<ProjectIntegrationResponse>, AppError> {
    let integration =
        IntegrationService::disable_integration(&state.biz_context, &auth.user.id, &integration_id)
            .await?;

    Ok(Json(integration))
}

/// Test an integration connection
///
/// Tests the connection to a third-party integration.
#[utoipa::path(
    post,
    path = "/integrations/{integration_id}/test",
    tag = "integrations",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("integration_id", Path, description = "The ID of the integration to test"),
    ),
    responses(
        (status = 200, description = "Integration test result", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Integration not found"),
    )
)]
pub async fn test_integration_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(integration_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result =
        IntegrationService::test_integration(&state.biz_context, &auth.user.id, &integration_id)
            .await?;

    Ok(Json(serde_json::json!(result)))
}
