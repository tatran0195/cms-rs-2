//! Reader access handlers
//!
//! This module contains the actual implementation of reader access handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::reader_access::ReaderAccessService;
use cms_entity::{
    common::Id,
    reader_access::{
        AudienceGrantResponse, AudienceResponse, CreateAudienceGrantRequest, CreateAudienceRequest,
        CreateReaderInvitationRequest, ReaderInvitationResponse, ReaderResponse,
        ReaderSessionResponse, UpdateAudienceRequest,
    },
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// Create a new audience
///
/// Creates a new audience for reader access to a project.
#[utoipa::path(
    post,
    path = "/reader-access/audiences",
    tag = "reader-access",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreateAudienceRequest,
    responses(
        (status = 200, description = "Audience created successfully", body = AudienceResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_audience_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateAudienceRequest>,
) -> Result<Json<AudienceResponse>, AppError> {
    let project_id = request.project_id.clone();
    let audience = ReaderAccessService::create_audience(
        &state.biz_context,
        &auth.user.id,
        &project_id,
        request,
    )
    .await?;

    Ok(Json(audience))
}

/// List audiences for a project
///
/// Returns all audiences for a specific project.
#[utoipa::path(
    get,
    path = "/reader-access/audiences/project/{project_id}",
    tag = "reader-access",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Path, description = "The ID of the project"),
    ),
    responses(
        (status = 200, description = "List of audiences", body = Vec<AudienceResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Project not found"),
    )
)]
pub async fn list_audiences_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<Id>,
) -> Result<Json<Vec<AudienceResponse>>, AppError> {
    let audiences =
        ReaderAccessService::list_audiences(&state.biz_context, &auth.user.id, &project_id).await?;

    Ok(Json(audiences))
}

/// Get a specific audience by ID
///
/// Retrieves an audience by its unique identifier.
#[utoipa::path(
    get,
    path = "/reader-access/audiences/{audience_id}",
    tag = "reader-access",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("audience_id", Path, description = "The ID of the audience to retrieve"),
    ),
    responses(
        (status = 200, description = "Audience found", body = AudienceResponse),
        (status = 404, description = "Audience not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_audience_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(audience_id): Path<Id>,
) -> Result<Json<AudienceResponse>, AppError> {
    let audience =
        ReaderAccessService::get_audience(&state.biz_context, &auth.user.id, &audience_id).await?;

    Ok(Json(audience))
}

/// Update an existing audience
///
/// Updates an audience by its ID with the provided fields.
#[utoipa::path(
    put,
    path = "/reader-access/audiences/{audience_id}",
    tag = "reader-access",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("audience_id", Path, description = "The ID of the audience to update"),
    ),
    request_body = UpdateAudienceRequest,
    responses(
        (status = 200, description = "Audience updated successfully", body = AudienceResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Audience not found"),
    )
)]
pub async fn update_audience_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(audience_id): Path<Id>,
    Json(request): Json<UpdateAudienceRequest>,
) -> Result<Json<AudienceResponse>, AppError> {
    let audience = ReaderAccessService::update_audience(
        &state.biz_context,
        &auth.user.id,
        &audience_id,
        request.name.as_deref(),
        request.description.as_deref(),
    )
    .await?;

    Ok(Json(audience))
}

/// Delete an audience
///
/// Permanently deletes an audience by its ID.
#[utoipa::path(
    delete,
    path = "/reader-access/audiences/{audience_id}",
    tag = "reader-access",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("audience_id", Path, description = "The ID of the audience to delete"),
    ),
    responses(
        (status = 200, description = "Audience deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Audience not found"),
    )
)]
pub async fn delete_audience_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(audience_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    ReaderAccessService::delete_audience(&state.biz_context, &auth.user.id, &audience_id).await?;

    Ok(Json(
        serde_json::json!({"success": true, "id": audience_id}),
    ))
}

/// Create an audience grant
///
/// Creates a new grant for an audience, allowing specific access.
#[utoipa::path(
    post,
    path = "/reader-access/grants",
    tag = "reader-access",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreateAudienceGrantRequest,
    responses(
        (status = 200, description = "Audience grant created successfully", body = AudienceGrantResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_audience_grant_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateAudienceGrantRequest>,
) -> Result<Json<AudienceGrantResponse>, AppError> {
    let grant = ReaderAccessService::create_audience_grant(
        &state.biz_context,
        &auth.user.id,
        &request.audience_id,
        &request.project_id,
        request.branch_id.as_deref(),
        request.language_id.as_deref(),
    )
    .await?;

    Ok(Json(grant))
}

/// List audience grants
///
/// Returns all grants for a specific audience.
#[utoipa::path(
    get,
    path = "/reader-access/grants/audience/{audience_id}",
    tag = "reader-access",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("audience_id", Path, description = "The ID of the audience"),
    ),
    responses(
        (status = 200, description = "List of audience grants", body = Vec<AudienceGrantResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Audience not found"),
    )
)]
pub async fn list_audience_grants_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(audience_id): Path<Id>,
) -> Result<Json<Vec<AudienceGrantResponse>>, AppError> {
    let grants =
        ReaderAccessService::list_audience_grants(&state.biz_context, &auth.user.id, &audience_id)
            .await?;

    Ok(Json(grants))
}

/// Delete an audience grant
///
/// Permanently deletes an audience grant by its ID.
#[utoipa::path(
    delete,
    path = "/reader-access/grants/{grant_id}",
    tag = "reader-access",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("grant_id", Path, description = "The ID of the audience grant to delete"),
    ),
    responses(
        (status = 200, description = "Audience grant deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Audience grant not found"),
    )
)]
pub async fn delete_audience_grant_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(grant_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    ReaderAccessService::delete_audience_grant(&state.biz_context, &auth.user.id, &grant_id)
        .await?;

    Ok(Json(serde_json::json!({"success": true, "id": grant_id})))
}

/// Create a reader invitation
///
/// Creates a new invitation for a reader to access a project.
#[utoipa::path(
    post,
    path = "/reader-access/invitations",
    tag = "reader-access",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreateReaderInvitationRequest,
    responses(
        (status = 200, description = "Reader invitation created successfully", body = ReaderInvitationResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_reader_invitation_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateReaderInvitationRequest>,
) -> Result<Json<ReaderInvitationResponse>, AppError> {
    let invitation = ReaderAccessService::create_reader_invitation(
        &state.biz_context,
        &auth.user.id,
        &request.audience_id,
        cms_entity::reader_access::CreateInvitationRequest {
            email: request.email,
            project_id: "".to_string(),
            audience_id: request.audience_id.clone(),
            expires_in_days: None,
        },
    )
    .await?;

    Ok(Json(invitation))
}

/// Accept a reader invitation
///
/// Accepts a reader invitation and creates a reader session.
/// This endpoint does not require authentication.
#[utoipa::path(
    post,
    path = "/reader-access/invitations/accept",
    tag = "reader-access",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Reader session created", body = ReaderSessionResponse),
        (status = 400, description = "Bad request - token is required"),
        (status = 404, description = "Invitation not found or expired"),
    )
)]
pub async fn accept_reader_invitation_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<ReaderSessionResponse>, AppError> {
    let token: String = serde_json::from_value(
        request
            .get("token")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|_| AppError::BadRequest("Invalid token".to_string()))?;

    let audience =
        ReaderAccessService::accept_reader_invitation(&state.biz_context, &token).await?;

    Ok(Json(ReaderSessionResponse {
        id: audience.id,
        reader_id: audience.reader_id,
        expires_at: chrono::Utc::now() + chrono::Duration::days(7),
        created_at: chrono::Utc::now(),
    }))
}
