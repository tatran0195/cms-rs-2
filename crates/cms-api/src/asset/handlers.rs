//! Asset handlers
//!
//! This module contains the actual implementation of asset handlers.

use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, Query, State},
    Json,
};
use cms_biz::asset::AssetService;
use cms_entity::{
    asset::{
        AssetResponse, DeleteAssetResponse, ListAssetsQuery, UpdateAssetRequest, UploadAssetRequest,
    },
    common::{Id, PaginatedResponse},
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;

use crate::auth::AuthExtractor;

/// List assets
///
/// Returns a list of assets filtered by project and page.
#[utoipa::path(
    get,
    path = "/assets",
    tag = "assets",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Query, description = "Filter by project ID"),
        ("page_id", Query, description = "Filter by page ID"),
        ("limit", Query, description = "Number of items to return"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of assets", body = PaginatedResponse<AssetResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_assets_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListAssetsQuery>,
) -> Result<Json<PaginatedResponse<AssetResponse>>, AppError> {
    let project_id = query.project_id.as_deref().unwrap_or("");
    let assets = AssetService::list_assets(
        &state.biz_context,
        &auth.user.id,
        project_id,
        query.limit.unwrap_or(1) as u64,
        query.offset.unwrap_or(20) as u64,
    )
    .await?;

    Ok(Json(assets))
}

/// Request direct upload URL
///
/// Creates an asset record and returns a presigned URL for direct upload.
#[utoipa::path(
    post,
    path = "/assets/upload",
    tag = "assets",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = UploadAssetRequest,
    responses(
        (status = 200, description = "Upload URL generated", body = AssetResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn upload_asset_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<UploadAssetRequest>,
) -> Result<Json<AssetResponse>, AppError> {
    let asset = AssetService::create_asset(
        &state.biz_context,
        &auth.user.id,
        &request.project_id,
        request.page_id.as_deref(),
        &request.file_name,
        &request.content_type,
        request.alt_text.as_deref(),
    )
    .await?;

    Ok(Json(asset))
}

/// Upload asset via multipart
///
/// Uploads a file via multipart/form-data.
#[utoipa::path(
    post,
    path = "/assets/upload-multipart/{project_id}",
    tag = "assets",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Path, description = "The ID of the project to upload to"),
    ),
    responses(
        (status = 200, description = "Asset uploaded successfully", body = AssetResponse),
        (status = 400, description = "Bad request - no file provided"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn upload_asset_multipart_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<AssetResponse>, AppError> {
    let mut file_name = None;
    let mut content_type = None;
    let mut file_data = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if field.name() == Some("file") {
            file_name = field.file_name().map(|s| s.to_string());
            content_type = field.content_type().map(|s| s.to_string());
            file_data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?
                .to_vec();
        }
    }

    let file_name =
        file_name.ok_or_else(|| AppError::BadRequest("No file provided".to_string()))?;
    let content_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    let asset = AssetService::upload_asset_bytes(
        &state.biz_context,
        &state.storage,
        &auth.user.id,
        &project_id,
        &file_name,
        &content_type,
        &file_data,
    )
    .await?;

    Ok(Json(asset))
}

/// Get a specific asset by ID
///
/// Retrieves an asset by its unique identifier.
#[utoipa::path(
    get,
    path = "/assets/{asset_id}",
    tag = "assets",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("asset_id", Path, description = "The ID of the asset to retrieve"),
    ),
    responses(
        (status = 200, description = "Asset found", body = AssetResponse),
        (status = 404, description = "Asset not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_asset_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(asset_id): Path<Id>,
) -> Result<Json<AssetResponse>, AppError> {
    let asset = AssetService::get_asset(&state.biz_context, &auth.user.id, &asset_id).await?;

    Ok(Json(asset))
}

/// Update an asset
///
/// Updates an asset by its ID with the provided fields.
#[utoipa::path(
    put,
    path = "/assets/{asset_id}",
    tag = "assets",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("asset_id", Path, description = "The ID of the asset to update"),
    ),
    request_body = UpdateAssetRequest,
    responses(
        (status = 200, description = "Asset updated successfully", body = AssetResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Asset not found"),
    )
)]
pub async fn update_asset_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(asset_id): Path<Id>,
    Json(request): Json<UpdateAssetRequest>,
) -> Result<Json<AssetResponse>, AppError> {
    let asset = AssetService::update_asset(
        &state.biz_context,
        &auth.user.id,
        &asset_id,
        request.alt_text.as_deref(),
    )
    .await?;

    Ok(Json(asset))
}

/// Delete an asset
///
/// Permanently deletes an asset by its ID.
#[utoipa::path(
    delete,
    path = "/assets/{asset_id}",
    tag = "assets",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("asset_id", Path, description = "The ID of the asset to delete"),
    ),
    responses(
        (status = 200, description = "Asset deleted successfully", body = DeleteAssetResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Asset not found"),
    )
)]
pub async fn delete_asset_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(asset_id): Path<Id>,
) -> Result<Json<DeleteAssetResponse>, AppError> {
    let _ = AssetService::delete_asset(
        &state.biz_context,
        &auth.user.id,
        state.storage.clone(),
        &asset_id,
    )
    .await?;

    Ok(Json(DeleteAssetResponse {
        success: true,
        asset_id,
    }))
}
