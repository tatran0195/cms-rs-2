//! Asset entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, Timestamp};

/// Asset entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Asset {
    pub id: Id,
    pub project_id: Id,
    pub page_id: Option<Id>,
    pub storage_key: String,
    pub file_name: String,
    pub content_type: String,
    pub file_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub alt_text: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Asset response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AssetResponse {
    pub id: Id,
    pub project_id: Id,
    pub page_id: Option<Id>,
    pub file_name: String,
    pub content_type: String,
    pub file_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub alt_text: Option<String>,
    pub download_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Asset> for AssetResponse {
    fn from(asset: Asset) -> Self {
        Self {
            id: asset.id,
            project_id: asset.project_id,
            page_id: asset.page_id,
            file_name: asset.file_name,
            content_type: asset.content_type,
            file_size: asset.file_size,
            width: asset.width,
            height: asset.height,
            alt_text: asset.alt_text,
            download_url: format!("/api/assets/{}/download", asset.storage_key),
            created_at: asset.created_at,
            updated_at: asset.updated_at,
        }
    }
}

/// Upload asset request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UploadAssetRequest {
    pub project_id: Id,
    #[serde(default)]
    pub page_id: Option<Id>,
    pub file_name: String,
    pub content_type: String,
    #[serde(default)]
    pub alt_text: Option<String>,
    /// File size in bytes (populated after upload)
    #[serde(default)]
    pub file_size: Option<i64>,
    /// Image width in pixels (if applicable)
    #[serde(default)]
    pub width: Option<i32>,
    /// Image height in pixels (if applicable)
    #[serde(default)]
    pub height: Option<i32>,
}

/// Update asset request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateAssetRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<String>,
}

/// Asset upload response (with presigned URL for direct upload)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AssetUploadResponse {
    pub asset_id: Id,
    pub storage_key: String,
    pub upload_url: String,
    pub download_url: String,
}

/// List assets query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListAssetsQuery {
    #[serde(default)]
    pub project_id: Option<Id>,
    #[serde(default)]
    pub page_id: Option<Id>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

/// Delete asset response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DeleteAssetResponse {
    pub success: bool,
    pub asset_id: Id,
}

/// Create asset request (alias for UploadAssetRequest for API compatibility)
pub type CreateAssetRequest = UploadAssetRequest;
