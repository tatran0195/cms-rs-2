//! Language handlers
//!
//! This module contains the actual implementation of language handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::language::LanguageService;
use cms_entity::{
    common::{Id, PaginatedResponse},
    language::{
        CreateLanguageRequest, LanguageResponse, ListLanguagesQuery, UpdateLanguageRequest,
    },
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List languages for a project
///
/// Returns a paginated list of languages filtered by project and optional language code.
#[utoipa::path(
    get,
    path = "/languages",
    tag = "languages",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Query, description = "Filter by project ID"),
        ("code", Query, description = "Filter by language code (e.g., 'en', 'ja')"),
        ("limit", Query, description = "Number of items per page"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of languages", body = PaginatedResponse<LanguageResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_languages_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListLanguagesQuery>,
) -> Result<Json<PaginatedResponse<LanguageResponse>>, AppError> {
    let result =
        LanguageService::list_languages(&state.biz_context, &auth.user.id, query, 1, 20).await?;

    Ok(Json(result))
}

/// Create a new language for a project
///
/// Adds a new language to a project's localization setup.
#[utoipa::path(
    post,
    path = "/languages",
    tag = "languages",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreateLanguageRequest,
    responses(
        (status = 200, description = "Language created successfully", body = LanguageResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_language_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateLanguageRequest>,
) -> Result<Json<LanguageResponse>, AppError> {
    let language =
        LanguageService::create_language(&state.biz_context, &auth.user.id, request).await?;

    Ok(Json(language))
}

/// Get a specific language by ID
///
/// Retrieves a language by its unique identifier.
#[utoipa::path(
    get,
    path = "/languages/{language_id}",
    tag = "languages",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("language_id", Path, description = "The ID of the language to retrieve"),
    ),
    responses(
        (status = 200, description = "Language found", body = LanguageResponse),
        (status = 404, description = "Language not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_language_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(language_id): Path<Id>,
) -> Result<Json<LanguageResponse>, AppError> {
    let language =
        LanguageService::get_language(&state.biz_context, &auth.user.id, &language_id).await?;

    Ok(Json(language))
}

/// Update an existing language
///
/// Updates a language by its ID with the provided fields.
#[utoipa::path(
    put,
    path = "/languages/{language_id}",
    tag = "languages",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("language_id", Path, description = "The ID of the language to update"),
    ),
    request_body = UpdateLanguageRequest,
    responses(
        (status = 200, description = "Language updated successfully", body = LanguageResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Language not found"),
    )
)]
pub async fn update_language_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(language_id): Path<Id>,
    Json(request): Json<UpdateLanguageRequest>,
) -> Result<Json<LanguageResponse>, AppError> {
    let language =
        LanguageService::update_language(&state.biz_context, &auth.user.id, &language_id, request)
            .await?;

    Ok(Json(language))
}

/// Delete a language
///
/// Permanently deletes a language by its ID.
#[utoipa::path(
    delete,
    path = "/languages/{language_id}",
    tag = "languages",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("language_id", Path, description = "The ID of the language to delete"),
    ),
    responses(
        (status = 200, description = "Language deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Language not found"),
    )
)]
pub async fn delete_language_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(language_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    LanguageService::delete_language(&state.biz_context, &auth.user.id, &language_id).await?;

    Ok(Json(
        serde_json::json!({"success": true, "id": language_id}),
    ))
}

/// Set the default language for a project
///
/// Sets the specified language as the default for a project.
#[utoipa::path(
    post,
    path = "/languages/projects/{project_id}/set-default",
    tag = "languages",
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
        (status = 200, description = "Default language set successfully", body = serde_json::Value),
        (status = 400, description = "Bad request - language_id is required"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Project not found"),
    )
)]
pub async fn set_default_language_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<Id>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let language_id: String = serde_json::from_value(
        request
            .get("language_id")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|_| AppError::BadRequest("Invalid language_id".to_string()))?;

    LanguageService::set_default_language(
        &state.biz_context,
        &auth.user.id,
        cms_entity::language::SetDefaultLanguageRequest { language_id },
    )
    .await?;

    Ok(Json(serde_json::json!({"success": true})))
}
