//! Comment handlers
//!
//! This module contains the actual implementation of comment handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::comment::CommentService;
use cms_entity::{
    comment::{
        CommentResponse, CommentWithReplies, CreateCommentRequest, ListCommentsQuery,
        ResolveCommentRequest, UpdateCommentRequest,
    },
    common::{Id, PaginatedResponse},
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List comments
///
/// Returns a paginated list of comments filtered by page, parent, and resolved status.
#[utoipa::path(
    get,
    path = "/comments",
    tag = "comments",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("page_id", Query, description = "Filter by page ID"),
        ("parent_id", Query, description = "Filter by parent comment ID"),
        ("resolved", Query, description = "Filter by resolved status"),
        ("limit", Query, description = "Number of items per page"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of comments", body = PaginatedResponse<CommentResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_comments_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListCommentsQuery>,
) -> Result<Json<PaginatedResponse<CommentResponse>>, AppError> {
    let page_id = query.page_id.as_deref().unwrap_or("");
    let result = CommentService::list_comments(
        &state.biz_context,
        &auth.user.id,
        page_id,
        query.parent_id.as_deref(),
        query.resolved,
        query.limit.unwrap_or(1) as u64,
        query.offset.unwrap_or(20) as u64,
    )
    .await?;

    Ok(Json(result))
}

/// Create a new comment
///
/// Creates a new comment on a page.
#[utoipa::path(
    post,
    path = "/comments",
    tag = "comments",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreateCommentRequest,
    responses(
        (status = 200, description = "Comment created successfully", body = CommentResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_comment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateCommentRequest>,
) -> Result<Json<CommentResponse>, AppError> {
    let page_id = request.page_id.clone();
    let comment =
        CommentService::create_comment(&state.biz_context, &auth.user.id, &page_id, request)
            .await?;

    Ok(Json(comment))
}

/// Get a comment with its replies
///
/// Retrieves a comment and all its nested replies.
#[utoipa::path(
    get,
    path = "/comments/{comment_id}",
    tag = "comments",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("comment_id", Path, description = "The ID of the comment to retrieve"),
    ),
    responses(
        (status = 200, description = "Comment found with replies", body = CommentWithReplies),
        (status = 404, description = "Comment not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_comment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(comment_id): Path<Id>,
) -> Result<Json<CommentWithReplies>, AppError> {
    let comment =
        CommentService::get_comment_with_replies(&state.biz_context, &auth.user.id, &comment_id)
            .await?;

    Ok(Json(comment))
}

/// Update a comment
///
/// Updates a comment by its ID with the provided content.
#[utoipa::path(
    put,
    path = "/comments/{comment_id}",
    tag = "comments",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("comment_id", Path, description = "The ID of the comment to update"),
    ),
    request_body = UpdateCommentRequest,
    responses(
        (status = 200, description = "Comment updated successfully", body = CommentResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission or is not the comment author"),
        (status = 404, description = "Comment not found"),
    )
)]
pub async fn update_comment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(comment_id): Path<Id>,
    Json(request): Json<UpdateCommentRequest>,
) -> Result<Json<CommentResponse>, AppError> {
    let comment =
        CommentService::update_comment(&state.biz_context, &auth.user.id, &comment_id, request)
            .await?;

    Ok(Json(comment))
}

/// Delete a comment
///
/// Permanently deletes a comment by its ID.
#[utoipa::path(
    delete,
    path = "/comments/{comment_id}",
    tag = "comments",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("comment_id", Path, description = "The ID of the comment to delete"),
    ),
    responses(
        (status = 200, description = "Comment deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission or is not the comment author"),
        (status = 404, description = "Comment not found"),
    )
)]
pub async fn delete_comment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(comment_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    CommentService::delete_comment(&state.biz_context, &auth.user.id, &comment_id).await?;

    Ok(Json(serde_json::json!({"success": true, "id": comment_id})))
}

/// Resolve or unresolve a comment
///
/// Marks a comment as resolved or unresolved.
#[utoipa::path(
    patch,
    path = "/comments/{comment_id}/resolve",
    tag = "comments",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("comment_id", Path, description = "The ID of the comment to resolve/unresolve"),
    ),
    request_body = ResolveCommentRequest,
    responses(
        (status = 200, description = "Comment resolved status updated", body = CommentResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Comment not found"),
    )
)]
pub async fn resolve_comment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(comment_id): Path<Id>,
    Json(request): Json<ResolveCommentRequest>,
) -> Result<Json<CommentResponse>, AppError> {
    let comment = if request.resolved {
        CommentService::resolve_comment(&state.biz_context, &auth.user.id, &comment_id).await?
    } else {
        CommentService::unresolve_comment(&state.biz_context, &auth.user.id, &comment_id).await?
    };

    Ok(Json(comment))
}

/// List comments for a specific page
///
/// Returns a paginated list of comments for a specific page.
#[utoipa::path(
    get,
    path = "/comments/page/{page_id}",
    tag = "comments",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("page_id", Path, description = "The ID of the page"),
        ("parent_id", Query, description = "Filter by parent comment ID"),
        ("resolved", Query, description = "Filter by resolved status"),
        ("limit", Query, description = "Number of items per page"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of comments for page", body = PaginatedResponse<CommentResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Page not found"),
    )
)]
pub async fn list_page_comments_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(page_id): Path<Id>,
    Query(query): Query<ListCommentsQuery>,
) -> Result<Json<PaginatedResponse<CommentResponse>>, AppError> {
    let result = CommentService::list_comments(
        &state.biz_context,
        &auth.user.id,
        &page_id,
        query.parent_id.as_deref(),
        query.resolved,
        query.limit.unwrap_or(1) as u64,
        query.offset.unwrap_or(20) as u64,
    )
    .await?;

    Ok(Json(result))
}
