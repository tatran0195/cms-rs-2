//! Page handlers
//!
//! This module contains the actual implementation of page handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::page::PageService;
use cms_entity::{
    common::{Id, PaginatedResponse},
    page::{CreatePageRequest, ListPagesQuery, PageListItem, PageResponse, UpdatePageRequest},
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List pages for a project and branch
///
/// Returns a paginated list of pages filtered by project, branch, and optional search criteria.
#[utoipa::path(
    get,
    path = "/pages",
    tag = "pages",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Query, description = "Filter by project ID"),
        ("branch_id", Query, description = "Filter by branch ID"),
        ("parent_id", Query, description = "Filter by parent page ID"),
        ("search", Query, description = "Search term for page title or content"),
        ("limit", Query, description = "Number of items per page"),
        ("offset", Query, description = "Pagination offset"),
        ("tree", Query, description = "Return as tree structure"),
    ),
    responses(
        (status = 200, description = "List of pages", body = PaginatedResponse<PageListItem>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_pages_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListPagesQuery>,
) -> Result<Json<PaginatedResponse<PageListItem>>, AppError> {
    let result = PageService::list_pages(&state.biz_context, &auth.user.id, query, 1, 20).await?;

    Ok(Json(result))
}

/// Create a new page
///
/// Creates a new page within a project and branch.
#[utoipa::path(
    post,
    path = "/pages",
    tag = "pages",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreatePageRequest,
    responses(
        (status = 200, description = "Page created successfully", body = PageResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_page_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreatePageRequest>,
) -> Result<Json<PageResponse>, AppError> {
    let project_id = request.project_id.clone();
    let branch_id = request.branch_id.clone();
    let page = PageService::create_page(
        &state.biz_context,
        &auth.user.id,
        &project_id,
        &branch_id,
        request,
    )
    .await?;

    Ok(Json(page))
}

/// Get a specific page by ID
///
/// Retrieves a page by its unique identifier.
#[utoipa::path(
    get,
    path = "/pages/{page_id}",
    tag = "pages",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("page_id", Path, description = "The ID of the page to retrieve"),
    ),
    responses(
        (status = 200, description = "Page found", body = PageResponse),
        (status = 404, description = "Page not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_page_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(page_id): Path<Id>,
) -> Result<Json<PageResponse>, AppError> {
    let page = PageService::get_page(&state.biz_context, &auth.user.id, &page_id).await?;

    Ok(Json(page))
}

/// Update an existing page
///
/// Updates a page by its ID with the provided fields.
#[utoipa::path(
    put,
    path = "/pages/{page_id}",
    tag = "pages",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("page_id", Path, description = "The ID of the page to update"),
    ),
    request_body = UpdatePageRequest,
    responses(
        (status = 200, description = "Page updated successfully", body = PageResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Page not found"),
    )
)]
pub async fn update_page_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(page_id): Path<Id>,
    Json(request): Json<UpdatePageRequest>,
) -> Result<Json<PageResponse>, AppError> {
    let page =
        PageService::update_page(&state.biz_context, &auth.user.id, &page_id, request).await?;

    Ok(Json(page))
}

/// Delete a page
///
/// Permanently deletes a page by its ID.
#[utoipa::path(
    delete,
    path = "/pages/{page_id}",
    tag = "pages",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("page_id", Path, description = "The ID of the page to delete"),
    ),
    responses(
        (status = 200, description = "Page deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Page not found"),
    )
)]
pub async fn delete_page_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(page_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    PageService::delete_page(&state.biz_context, &auth.user.id, &page_id).await?;

    Ok(Json(serde_json::json!({"success": true, "id": page_id})))
}
