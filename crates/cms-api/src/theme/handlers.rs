//! Theme handlers
//!
//! This module contains the actual implementation of theme handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::theme::ThemeService;
use cms_entity::{
    common::Id,
    theme::{
        CreateThemeRequest, ListThemesQuery, ThemeCssVariables, ThemeResponse, UpdateThemeRequest,
    },
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List themes
///
/// Returns a list of themes filtered by project.
#[utoipa::path(
    get,
    path = "/themes",
    tag = "themes",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Query, description = "Filter by project ID"),
        ("limit", Query, description = "Number of items to return"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of themes", body = Vec<ThemeResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_themes_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListThemesQuery>,
) -> Result<Json<Vec<ThemeResponse>>, AppError> {
    let project_id = query.project_id.as_deref().unwrap_or("");
    let themes = ThemeService::list_themes(&state.biz_context, &auth.user.id, project_id).await?;

    Ok(Json(themes))
}

/// Create a new theme
///
/// Creates a new theme for a project.
#[utoipa::path(
    post,
    path = "/themes",
    tag = "themes",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreateThemeRequest,
    responses(
        (status = 200, description = "Theme created successfully", body = ThemeResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_theme_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateThemeRequest>,
) -> Result<Json<ThemeResponse>, AppError> {
    let project_id = request.project_id.clone();
    let theme =
        ThemeService::create_theme(&state.biz_context, &auth.user.id, &project_id, request).await?;

    Ok(Json(theme))
}

/// Get a specific theme by ID
///
/// Retrieves a theme by its unique identifier.
#[utoipa::path(
    get,
    path = "/themes/{theme_id}",
    tag = "themes",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("theme_id", Path, description = "The ID of the theme to retrieve"),
    ),
    responses(
        (status = 200, description = "Theme found", body = ThemeResponse),
        (status = 404, description = "Theme not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_theme_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(theme_id): Path<Id>,
) -> Result<Json<ThemeResponse>, AppError> {
    let theme = ThemeService::get_theme(&state.biz_context, &auth.user.id, &theme_id).await?;

    Ok(Json(theme))
}

/// Update an existing theme
///
/// Updates a theme by its ID with the provided fields.
#[utoipa::path(
    put,
    path = "/themes/{theme_id}",
    tag = "themes",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("theme_id", Path, description = "The ID of the theme to update"),
    ),
    request_body = UpdateThemeRequest,
    responses(
        (status = 200, description = "Theme updated successfully", body = ThemeResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Theme not found"),
    )
)]
pub async fn update_theme_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(theme_id): Path<Id>,
    Json(request): Json<UpdateThemeRequest>,
) -> Result<Json<ThemeResponse>, AppError> {
    let theme =
        ThemeService::update_theme(&state.biz_context, &auth.user.id, &theme_id, request).await?;

    Ok(Json(theme))
}

/// Delete a theme
///
/// Permanently deletes a theme by its ID.
#[utoipa::path(
    delete,
    path = "/themes/{theme_id}",
    tag = "themes",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("theme_id", Path, description = "The ID of the theme to delete"),
    ),
    responses(
        (status = 200, description = "Theme deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Theme not found"),
    )
)]
pub async fn delete_theme_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(theme_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    ThemeService::delete_theme(&state.biz_context, &auth.user.id, &theme_id).await?;

    Ok(Json(serde_json::json!({"success": true, "id": theme_id})))
}

/// Get theme CSS variables
///
/// Returns the CSS variables for a theme.
#[utoipa::path(
    get,
    path = "/themes/{theme_id}/css",
    tag = "themes",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("theme_id", Path, description = "The ID of the theme"),
    ),
    responses(
        (status = 200, description = "Theme CSS variables", body = ThemeCssVariables),
        (status = 404, description = "Theme not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_theme_css_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(theme_id): Path<Id>,
) -> Result<Json<ThemeCssVariables>, AppError> {
    let css_vars =
        ThemeService::get_theme_css(&state.biz_context, &auth.user.id, &theme_id).await?;

    Ok(Json(css_vars))
}

/// Set project theme
///
/// Sets the specified theme as the active theme for a project.
#[utoipa::path(
    post,
    path = "/themes/set-project-theme/{project_id}",
    tag = "themes",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Path, description = "The ID of the project"),
    ),
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Project theme set successfully", body = ThemeResponse),
        (status = 400, description = "Bad request - theme_id is required"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Project or theme not found"),
    )
)]
pub async fn set_project_theme_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<Id>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<ThemeResponse>, AppError> {
    let theme_id: String = serde_json::from_value(
        request
            .get("theme_id")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|_| AppError::BadRequest("Invalid theme_id".to_string()))?;

    let theme =
        ThemeService::set_project_theme(&state.biz_context, &auth.user.id, &project_id, &theme_id)
            .await?;

    Ok(Json(theme))
}
