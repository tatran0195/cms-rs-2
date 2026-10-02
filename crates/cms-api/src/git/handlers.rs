//! Git handlers
//!
//! This module contains the actual implementation of Git-related handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::git::GitService;
use cms_entity::{
    common::{Id, PaginatedResponse},
    git::{
        CreateGitConnectionRequest, GitConnectionResponse, GitSyncOperationResponse,
        ListGitSyncOperationsQuery, UpdateGitConnectionRequest,
    },
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// Create a new Git connection
///
/// Sets up a connection to a Git repository for syncing content.
#[utoipa::path(
    post,
    path = "/git/connections",
    tag = "git",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreateGitConnectionRequest,
    responses(
        (status = 200, description = "Git connection created successfully", body = GitConnectionResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_git_connection_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateGitConnectionRequest>,
) -> Result<Json<GitConnectionResponse>, AppError> {
    let project_id = request.project_id.clone();
    let connection =
        GitService::create_connection(&state.biz_context, &auth.user.id, &project_id, request)
            .await?;

    Ok(Json(connection))
}

/// Get a Git connection by ID
///
/// Retrieves a Git connection by its unique identifier.
#[utoipa::path(
    get,
    path = "/git/connections/{connection_id}",
    tag = "git",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("connection_id", Path, description = "The ID of the Git connection to retrieve"),
    ),
    responses(
        (status = 200, description = "Git connection found", body = GitConnectionResponse),
        (status = 404, description = "Git connection not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_git_connection_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(connection_id): Path<Id>,
) -> Result<Json<GitConnectionResponse>, AppError> {
    let connection = GitService::get_connection(&state.biz_context, &auth.user.id, &connection_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Git connection not found".to_string()))?;

    Ok(Json(connection))
}

/// List Git connections for a project
///
/// Returns all Git connections for a specific project.
#[utoipa::path(
    get,
    path = "/git/connections/project/{project_id}",
    tag = "git",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Path, description = "The ID of the project"),
    ),
    responses(
        (status = 200, description = "List of Git connections", body = Vec<GitConnectionResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Project not found"),
    )
)]
pub async fn list_git_connections_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<Id>,
) -> Result<Json<Vec<GitConnectionResponse>>, AppError> {
    let connections =
        GitService::list_connections(&state.biz_context, &auth.user.id, &project_id).await?;

    Ok(Json(connections))
}

/// Update a Git connection
///
/// Updates a Git connection by its ID with the provided fields.
#[utoipa::path(
    put,
    path = "/git/connections/{connection_id}",
    tag = "git",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("connection_id", Path, description = "The ID of the Git connection to update"),
    ),
    request_body = UpdateGitConnectionRequest,
    responses(
        (status = 200, description = "Git connection updated successfully", body = GitConnectionResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Git connection not found"),
    )
)]
pub async fn update_git_connection_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(connection_id): Path<Id>,
    Json(request): Json<UpdateGitConnectionRequest>,
) -> Result<Json<GitConnectionResponse>, AppError> {
    let connection =
        GitService::update_connection(&state.biz_context, &auth.user.id, &connection_id, request)
            .await?;

    Ok(Json(connection))
}

/// Delete a Git connection
///
/// Deletes a Git connection by its ID.
#[utoipa::path(
    delete,
    path = "/git/connections/{connection_id}",
    tag = "git",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("connection_id", Path, description = "The ID of the Git connection to delete"),
    ),
    responses(
        (status = 200, description = "Git connection deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Git connection not found"),
    )
)]
pub async fn delete_git_connection_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(connection_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    GitService::delete_connection(&state.biz_context, &auth.user.id, &connection_id).await?;

    Ok(Json(
        serde_json::json!({"success": true, "message": "Git connection deleted"}),
    ))
}

/// Trigger a Git sync operation
///
/// Initiates a synchronization operation for a Git connection.
#[utoipa::path(
    post,
    path = "/git/connections/{connection_id}/sync",
    tag = "git",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("connection_id", Path, description = "The ID of the Git connection to sync"),
    ),
    responses(
        (status = 200, description = "Git sync operation started", body = GitSyncOperationResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Git connection not found"),
    )
)]
pub async fn trigger_git_sync_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(connection_id): Path<Id>,
) -> Result<Json<GitSyncOperationResponse>, AppError> {
    let operation = GitService::trigger_sync(
        &state.biz_context,
        &auth.user.id,
        &connection_id,
        cms_entity::git::GitSyncOperationType::Manual,
    )
    .await?;

    Ok(Json(operation))
}

/// List Git sync operations for a connection
///
/// Returns a paginated list of sync operations for a specific Git connection.
#[utoipa::path(
    get,
    path = "/git/connections/{connection_id}/operations",
    tag = "git",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("connection_id", Path, description = "The ID of the Git connection"),
        ("status", Query, description = "Filter by operation status"),
        ("limit", Query, description = "Number of items per page"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of Git sync operations", body = PaginatedResponse<GitSyncOperationResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Git connection not found"),
    )
)]
pub async fn list_git_sync_operations_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(connection_id): Path<Id>,
    Query(query): Query<ListGitSyncOperationsQuery>,
) -> Result<Json<PaginatedResponse<GitSyncOperationResponse>>, AppError> {
    let result = GitService::list_sync_operations(
        &state.biz_context,
        &auth.user.id,
        &connection_id,
        query.limit.unwrap_or(1) as u64,
        query.offset.unwrap_or(20) as u64,
    )
    .await?;

    Ok(Json(result))
}

/// Get a specific Git sync operation
///
/// Retrieves a Git sync operation by its unique identifier.
#[utoipa::path(
    get,
    path = "/git/operations/{operation_id}",
    tag = "git",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("operation_id", Path, description = "The ID of the Git sync operation to retrieve"),
    ),
    responses(
        (status = 200, description = "Git sync operation found", body = GitSyncOperationResponse),
        (status = 404, description = "Git sync operation not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_git_sync_operation_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(operation_id): Path<Id>,
) -> Result<Json<GitSyncOperationResponse>, AppError> {
    let operation =
        GitService::get_sync_operation(&state.biz_context, &auth.user.id, &operation_id).await?;

    Ok(Json(operation))
}

/// Get Git sync status for a connection
///
/// Returns the current sync status for a Git connection.
#[utoipa::path(
    get,
    path = "/git/connections/{connection_id}/status",
    tag = "git",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("connection_id", Path, description = "The ID of the Git connection"),
    ),
    responses(
        (status = 200, description = "Git sync status", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Git connection not found"),
    )
)]
pub async fn get_git_sync_status_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(connection_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    let status =
        GitService::get_sync_status(&state.biz_context, &auth.user.id, &connection_id).await?;

    Ok(Json(status))
}
