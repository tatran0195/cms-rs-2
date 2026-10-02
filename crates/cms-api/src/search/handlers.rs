//! Search handlers
//!
//! This module contains the actual implementation of search handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::search::SearchService;
use cms_entity::{
    common::{Id, PaginatedResponse},
    search::{
        ListSearchIndexRunsQuery, ReindexRequest, SearchIndexRunResponse, SearchRequest,
        SearchResponse,
    },
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// Perform a search
///
/// Searches across pages and content in projects the user has access to.
#[utoipa::path(
    get,
    path = "/search",
    tag = "search",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("q", Query, description = "Search query"),
        ("project_id", Query, description = "Filter by project ID"),
        ("branch_id", Query, description = "Filter by branch ID"),
        ("limit", Query, description = "Number of results to return"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn search_handler(
    State(state): State<Arc<AppState>>,
    _auth: AuthExtractor,
    Query(request): Query<SearchRequest>,
) -> Result<Json<SearchResponse>, AppError> {
    let project_id = request.project_id.clone();
    let result = SearchService::search(
        &state.biz_context,
        state.search_engine.clone(),
        &project_id,
        request,
    )
    .await?;

    Ok(Json(result))
}

/// Trigger a reindex
///
/// Triggers a reindexing of search content for projects.
#[utoipa::path(
    post,
    path = "/search/reindex",
    tag = "search",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = ReindexRequest,
    responses(
        (status = 200, description = "Reindexing started", body = serde_json::Value),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn reindex_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<ReindexRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    SearchService::reindex(
        &state.biz_context,
        state.search_engine.clone(),
        &auth.user.id,
        request,
    )
    .await?;

    Ok(Json(
        serde_json::json!({"success": true, "message": "Reindexing started"}),
    ))
}

/// List search index runs
///
/// Returns a paginated list of search index runs.
#[utoipa::path(
    get,
    path = "/search/index-runs",
    tag = "search",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Query, description = "Filter by project ID"),
        ("status", Query, description = "Filter by run status"),
        ("limit", Query, description = "Number of items per page"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of search index runs", body = PaginatedResponse<SearchIndexRunResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_search_index_runs_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListSearchIndexRunsQuery>,
) -> Result<Json<PaginatedResponse<SearchIndexRunResponse>>, AppError> {
    let result = SearchService::list_index_runs(&state.biz_context, &auth.user.id, query).await?;

    Ok(Json(result))
}

/// Get a specific search index run
///
/// Retrieves a search index run by its unique identifier.
#[utoipa::path(
    get,
    path = "/search/index-runs/{run_id}",
    tag = "search",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("run_id", Path, description = "The ID of the search index run to retrieve"),
    ),
    responses(
        (status = 200, description = "Search index run found", body = SearchIndexRunResponse),
        (status = 404, description = "Search index run not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_search_index_run_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(run_id): Path<Id>,
) -> Result<Json<SearchIndexRunResponse>, AppError> {
    let run = SearchService::get_index_run(&state.biz_context, &auth.user.id, &run_id).await?;

    Ok(Json(run))
}

/// Get search status for a project
///
/// Returns the current search status for a specific project.
#[utoipa::path(
    get,
    path = "/search/status/{project_id}",
    tag = "search",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Path, description = "The ID of the project"),
    ),
    responses(
        (status = 200, description = "Search status", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Project not found"),
    )
)]
pub async fn get_search_status_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    let status =
        SearchService::get_search_status(&state.biz_context, &auth.user.id, &project_id).await?;

    Ok(Json(status))
}
