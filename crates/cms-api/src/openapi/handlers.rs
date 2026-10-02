//! OpenAPI handlers
//!
//! This module contains the actual implementation of OpenAPI handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::openapi::OpenApiService;
use cms_entity::{
    common::{Id, PaginatedResponse},
    openapi::{
        CreateOpenApiDocumentRequest, ListOpenApiDocumentsQuery, OpenApiDocumentResponse,
        OpenApiParsingResult, ParseOpenApiDocumentRequest, UpdateOpenApiDocumentRequest,
    },
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List OpenAPI documents
///
/// Returns a paginated list of OpenAPI documents filtered by project and error status.
#[utoipa::path(
    get,
    path = "/openapi/documents",
    tag = "openapi",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Query, description = "Filter by project ID"),
        ("has_error", Query, description = "Filter by error status"),
        ("limit", Query, description = "Number of items per page"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of OpenAPI documents", body = PaginatedResponse<OpenApiDocumentResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_openapi_documents_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListOpenApiDocumentsQuery>,
) -> Result<Json<PaginatedResponse<OpenApiDocumentResponse>>, AppError> {
    let project_id = query.project_id.as_deref().unwrap_or("");
    let documents =
        OpenApiService::list_documents(&state.biz_context, &auth.user.id, project_id).await?;
    let total = documents.len() as u64;

    Ok(Json(PaginatedResponse::new(documents, total, 1, 20)))
}

/// Create a new OpenAPI document
///
/// Creates a new OpenAPI document for a project.
#[utoipa::path(
    post,
    path = "/openapi/documents",
    tag = "openapi",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreateOpenApiDocumentRequest,
    responses(
        (status = 200, description = "OpenAPI document created successfully", body = OpenApiDocumentResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_openapi_document_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateOpenApiDocumentRequest>,
) -> Result<Json<OpenApiDocumentResponse>, AppError> {
    let document =
        OpenApiService::create_document(&state.biz_context, &auth.user.id, request).await?;

    Ok(Json(document))
}

/// Get a specific OpenAPI document
///
/// Retrieves an OpenAPI document by its unique identifier.
#[utoipa::path(
    get,
    path = "/openapi/documents/{document_id}",
    tag = "openapi",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("document_id", Path, description = "The ID of the OpenAPI document to retrieve"),
    ),
    responses(
        (status = 200, description = "OpenAPI document found", body = OpenApiDocumentResponse),
        (status = 404, description = "OpenAPI document not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_openapi_document_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(document_id): Path<Id>,
) -> Result<Json<OpenApiDocumentResponse>, AppError> {
    let document =
        OpenApiService::get_document(&state.biz_context, &auth.user.id, &document_id).await?;

    Ok(Json(document))
}

/// Update an OpenAPI document
///
/// Updates an OpenAPI document by its ID.
#[utoipa::path(
    put,
    path = "/openapi/documents/{document_id}",
    tag = "openapi",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("document_id", Path, description = "The ID of the OpenAPI document to update"),
    ),
    request_body = UpdateOpenApiDocumentRequest,
    responses(
        (status = 200, description = "OpenAPI document updated successfully", body = OpenApiDocumentResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "OpenAPI document not found"),
    )
)]
pub async fn update_openapi_document_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(document_id): Path<Id>,
    Json(request): Json<UpdateOpenApiDocumentRequest>,
) -> Result<Json<OpenApiDocumentResponse>, AppError> {
    let document =
        OpenApiService::update_document(&state.biz_context, &auth.user.id, &document_id, request)
            .await?;

    Ok(Json(document))
}

/// Delete an OpenAPI document
///
/// Permanently deletes an OpenAPI document by its ID.
#[utoipa::path(
    delete,
    path = "/openapi/documents/{document_id}",
    tag = "openapi",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("document_id", Path, description = "The ID of the OpenAPI document to delete"),
    ),
    responses(
        (status = 200, description = "OpenAPI document deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "OpenAPI document not found"),
    )
)]
pub async fn delete_openapi_document_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(document_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    OpenApiService::delete_document(&state.biz_context, &auth.user.id, &document_id).await?;

    Ok(Json(
        serde_json::json!({"success": true, "id": document_id}),
    ))
}

/// Parse an OpenAPI document
///
/// Parses an OpenAPI document and validates it.
#[utoipa::path(
    post,
    path = "/openapi/documents/{document_id}/parse",
    tag = "openapi",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("document_id", Path, description = "The ID of the OpenAPI document to parse"),
    ),
    request_body = ParseOpenApiDocumentRequest,
    responses(
        (status = 200, description = "OpenAPI document parsing result", body = OpenApiParsingResult),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "OpenAPI document not found"),
    )
)]
pub async fn parse_openapi_document_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(document_id): Path<Id>,
    Json(request): Json<ParseOpenApiDocumentRequest>,
) -> Result<Json<OpenApiParsingResult>, AppError> {
    let result = OpenApiService::parse_document(&state.biz_context, &auth.user.id, request).await?;

    Ok(Json(result))
}

/// Get OpenAPI document content
///
/// Retrieves the raw content of an OpenAPI document.
#[utoipa::path(
    get,
    path = "/openapi/documents/{document_id}/content",
    tag = "openapi",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("document_id", Path, description = "The ID of the OpenAPI document"),
    ),
    responses(
        (status = 200, description = "OpenAPI document content", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "OpenAPI document not found"),
    )
)]
pub async fn get_openapi_content_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(document_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    let content =
        OpenApiService::get_document_content(&state.biz_context, &auth.user.id, &document_id)
            .await?;

    Ok(Json(serde_json::json!({"content": content})))
}
