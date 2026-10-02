//! Theme Business Logic
//!
//! This module contains business logic for theme management.

use chrono::Utc;
use cms_db::{project::ProjectQueries, theme::ThemeQueries};
use cms_entity::{
    common::{Id, MemberRole, PaginatedResponse},
    theme::{CreateThemeRequest, Theme, ThemeResponse, UpdateThemeRequest},
};
use uuid::Uuid;

use crate::{AppError, BizContext};

/// Theme service
pub struct ThemeService;

impl ThemeService {
    /// Create a theme
    pub async fn create_theme(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        request: CreateThemeRequest,
    ) -> Result<ThemeResponse, AppError> {
        // Verify project exists
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Admin)
            .await?;

        let theme = ThemeQueries::create(
            &ctx.pool,
            project_id,
            &request.name,
            request.config.clone().unwrap_or_default(),
            request.is_global.unwrap_or(false),
        )
        .await?;

        Ok(theme.into())
    }

    /// Get a theme
    pub async fn get_theme(
        ctx: &BizContext,
        user_id: &str,
        theme_id: &str,
    ) -> Result<ThemeResponse, AppError> {
        let theme = ThemeQueries::get_by_id(&ctx.pool, theme_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Theme not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &theme.project_id, MemberRole::Viewer)
            .await?;

        Ok(theme.into())
    }

    /// List themes for a project
    pub async fn list_themes(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<Vec<ThemeResponse>, AppError> {
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;

        let themes = ThemeQueries::get_by_project(&ctx.pool, project_id).await?;

        Ok(themes.into_iter().map(|t| t.into()).collect())
    }

    /// Update a theme
    pub async fn update_theme(
        ctx: &BizContext,
        user_id: &str,
        theme_id: &str,
        request: UpdateThemeRequest,
    ) -> Result<ThemeResponse, AppError> {
        let theme = ThemeQueries::get_by_id(&ctx.pool, theme_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Theme not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &theme.project_id, MemberRole::Admin)
            .await?;

        let updated = ThemeQueries::update(
            &ctx.pool,
            theme_id,
            request.name.as_deref(),
            request.config.as_ref(),
            request.is_global,
        )
        .await?;

        Ok(updated.into())
    }

    /// Delete a theme
    pub async fn delete_theme(
        ctx: &BizContext,
        user_id: &str,
        theme_id: &str,
    ) -> Result<bool, AppError> {
        let theme = ThemeQueries::get_by_id(&ctx.pool, theme_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Theme not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &theme.project_id, MemberRole::Admin)
            .await?;

        // Cannot delete global theme
        if theme.is_global {
            return Err(AppError::AccessDenied(
                "Cannot delete global theme".to_string(),
            ));
        }

        ThemeQueries::delete(&ctx.pool, theme_id).await
    }

    /// Get theme CSS variables
    pub async fn get_theme_css(
        ctx: &BizContext,
        user_id: &str,
        theme_id: &str,
    ) -> Result<cms_entity::theme::ThemeCssVariables, AppError> {
        let theme = ThemeQueries::get_by_id(&ctx.pool, theme_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Theme not found".to_string()))?;

        ctx.authz
            .require_project_role(user_id, &theme.project_id, MemberRole::Viewer)
            .await?;

        Ok(theme.into())
    }

    /// Set project theme
    pub async fn set_project_theme(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        theme_id: &str,
    ) -> Result<ThemeResponse, AppError> {
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Admin)
            .await?;

        let theme = Self::get_theme(ctx, user_id, theme_id).await?;

        cms_db::project::ProjectSettingsQueries::upsert(
            &ctx.pool,
            project_id,
            Some(theme_id),
            None,
            None,
            None,
            None,
        )
        .await?;

        Ok(theme)
    }
}
