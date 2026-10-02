//! Asset Business Logic
//!
//! This module contains business logic for asset management (images, files, etc.).

use std::sync::Arc;

use bytes::Bytes;
use chrono::Utc;
use cms_db::{asset::AssetQueries, page::PageQueries, project::ProjectQueries, PgPool};
use cms_entity::{
    asset::{Asset, AssetResponse, CreateAssetRequest},
    common::{Id, MemberRole, PaginatedResponse},
};
use cms_storage::Storage;
use uuid::Uuid;

use crate::{AppError, BizContext};

/// Asset service
pub struct AssetService;

impl AssetService {
    /// Upload an asset
    pub async fn upload_asset(
        ctx: &BizContext,
        user_id: &str,
        storage: Arc<dyn Storage>,
        project_id: &str,
        page_id: Option<&str>,
        request: CreateAssetRequest,
        content: Bytes,
    ) -> Result<AssetResponse, AppError> {
        // Verify project exists
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Verify page exists (if specified)
        if let Some(page_id) = page_id {
            let _page = PageQueries::get_by_id(&ctx.pool, page_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;
        }

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Editor)
            .await?;

        // Generate storage key
        let storage_key = format!(
            "assets/{}/{}/{}",
            project_id,
            Utc::now().timestamp(),
            request.file_name
        );

        let file_size = request.file_size.unwrap_or(content.len() as i64);

        // Store the file
        storage
            .put(&storage_key, content, &request.content_type)
            .await?;

        // Create asset record
        let asset = AssetQueries::create(
            &ctx.pool,
            project_id,
            page_id,
            &storage_key,
            &request.file_name,
            &request.content_type,
            file_size,
            request.width,
            request.height,
            request.alt_text.as_deref(),
        )
        .await?;

        Ok(asset.into())
    }

    /// Get an asset
    pub async fn get_asset(
        ctx: &BizContext,
        user_id: &str,
        asset_id: &str,
    ) -> Result<AssetResponse, AppError> {
        let asset = AssetQueries::get_by_id(&ctx.pool, asset_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Asset not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &asset.project_id, MemberRole::Viewer)
            .await?;

        Ok(asset.into())
    }

    /// List assets for a project
    pub async fn list_assets(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResponse<AssetResponse>, AppError> {
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;

        let assets = AssetQueries::get_by_project(
            &ctx.pool,
            project_id,
            Some(page as i64),
            Some(page_size as i64),
        )
        .await?;

        let total = AssetQueries::count_by_project(&ctx.pool, project_id).await?;

        Ok(PaginatedResponse::new(
            assets.into_iter().map(|a| a.into()).collect(),
            total as u64,
            page,
            page_size,
        ))
    }

    /// List assets for a page
    pub async fn list_assets_for_page(
        ctx: &BizContext,
        user_id: &str,
        page_id: &str,
    ) -> Result<Vec<AssetResponse>, AppError> {
        let page = PageQueries::get_by_id(&ctx.pool, page_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &page.project_id, MemberRole::Viewer)
            .await?;

        let assets = AssetQueries::get_by_page(&ctx.pool, page_id).await?;

        Ok(assets.into_iter().map(|a| a.into()).collect())
    }

    /// Update an asset
    pub async fn update_asset(
        ctx: &BizContext,
        user_id: &str,
        asset_id: &str,
        alt_text: Option<&str>,
    ) -> Result<AssetResponse, AppError> {
        let asset = AssetQueries::get_by_id(&ctx.pool, asset_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Asset not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &asset.project_id, MemberRole::Editor)
            .await?;

        let updated = AssetQueries::update(&ctx.pool, asset_id, alt_text).await?;

        Ok(updated.into())
    }

    /// Delete an asset
    pub async fn delete_asset(
        ctx: &BizContext,
        user_id: &str,
        storage: Arc<dyn Storage>,
        asset_id: &str,
    ) -> Result<bool, AppError> {
        let asset = AssetQueries::get_by_id(&ctx.pool, asset_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Asset not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &asset.project_id, MemberRole::Editor)
            .await?;

        // Delete from storage
        storage.delete(&asset.storage_key).await?;

        // Delete from database
        AssetQueries::delete(&ctx.pool, asset_id).await
    }

    /// Create asset record
    pub async fn create_asset(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        page_id: Option<&str>,
        file_name: &str,
        content_type: &str,
        alt_text: Option<&str>,
    ) -> Result<AssetResponse, AppError> {
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Editor)
            .await?;
        let storage_key = format!(
            "assets/{}/{}/{}",
            project_id,
            Utc::now().timestamp(),
            file_name
        );
        let asset = AssetQueries::create(
            &ctx.pool,
            project_id,
            page_id,
            &storage_key,
            file_name,
            content_type,
            0,
            None,
            None,
            alt_text,
        )
        .await?;
        Ok(asset.into())
    }

    /// Upload asset bytes
    pub async fn upload_asset_bytes(
        ctx: &BizContext,
        storage: &Arc<dyn Storage>,
        user_id: &str,
        project_id: &str,
        file_name: &str,
        content_type: &str,
        file_data: &[u8],
    ) -> Result<AssetResponse, AppError> {
        Self::upload_asset(
            ctx,
            user_id,
            storage.clone(),
            project_id,
            None,
            CreateAssetRequest {
                project_id: project_id.to_string(),
                page_id: None,
                file_name: file_name.to_string(),
                content_type: content_type.to_string(),
                file_size: Some(file_data.len() as i64),
                width: None,
                height: None,
                alt_text: None,
            },
            Bytes::copy_from_slice(file_data),
        )
        .await
    }
}
