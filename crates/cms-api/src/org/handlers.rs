//! Org handlers
//!
//! This module contains the actual implementation of organization handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::org::OrgService;
use cms_entity::{
    common::{Id, PaginatedResponse},
    org::{
        CreateInvitationRequest, CreateOrganizationRequest, InvitationResponse,
        ListInvitationsResponse, ListMembersQuery, ListMembersResponse, OrganizationResponse,
        UpdateOrganizationRequest,
    },
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List all organizations for the authenticated user
///
/// Returns a list of organizations that the user is a member of.
#[utoipa::path(
    get,
    path = "/orgs",
    tag = "orgs",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    responses(
        (status = 200, description = "List of organizations", body = Vec<OrganizationResponse>),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn list_orgs_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<Vec<OrganizationResponse>>, AppError> {
    let orgs = OrgService::list_organizations_for_user(&state.biz_context, &auth.user.id).await?;

    Ok(Json(orgs))
}

/// Create a new organization
///
/// Creates a new organization with the authenticated user as the owner.
#[utoipa::path(
    post,
    path = "/orgs",
    tag = "orgs",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreateOrganizationRequest,
    responses(
        (status = 200, description = "Organization created successfully", body = OrganizationResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn create_org_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateOrganizationRequest>,
) -> Result<Json<OrganizationResponse>, AppError> {
    let org = OrgService::create_organization(&state.biz_context, &auth.user.id, request).await?;

    Ok(Json(org))
}

/// Get organization handler
pub async fn get_org_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(org_id): Path<Id>,
) -> Result<Json<OrganizationResponse>, AppError> {
    // Check access
    state
        .biz_context
        .authz
        .require_org_member(&auth.user.id, &org_id)
        .await?;

    let org = OrgService::get_organization(&state.biz_context, &org_id).await?;

    Ok(Json(org))
}

/// Update organization handler
pub async fn update_org_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(org_id): Path<Id>,
    Json(request): Json<UpdateOrganizationRequest>,
) -> Result<Json<OrganizationResponse>, AppError> {
    let org = OrgService::update_organization(&state.biz_context, &auth.user.id, &org_id, request)
        .await?;

    Ok(Json(org))
}

/// Delete organization handler
pub async fn delete_org_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(org_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    OrgService::delete_organization(&state.biz_context, &auth.user.id, &org_id).await?;

    Ok(Json(serde_json::json!({"success": true, "id": org_id})))
}

/// List members handler
pub async fn list_members_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(org_id): Path<Id>,
    Query(query): Query<ListMembersQuery>,
) -> Result<Json<ListMembersResponse>, AppError> {
    let result =
        OrgService::list_members(&state.biz_context, &auth.user.id, &org_id, query, 1, 100).await?;

    Ok(Json(result))
}

/// Add member handler
pub async fn add_member_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(org_id): Path<Id>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let new_user_id: String = serde_json::from_value(
        request
            .get("user_id")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|_| AppError::BadRequest("Invalid user_id".to_string()))?;

    let role: String = serde_json::from_value(
        request
            .get("role")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .unwrap_or_else(|_| "MEMBER".to_string());

    let role = match role.to_uppercase().as_str() {
        "OWNER" => cms_entity::common::MemberRole::Owner,
        "ADMIN" => cms_entity::common::MemberRole::Admin,
        "MEMBER" => cms_entity::common::MemberRole::Member,
        "GUEST" => cms_entity::common::MemberRole::Guest,
        _ => cms_entity::common::MemberRole::Member,
    };

    let member = OrgService::add_member(
        &state.biz_context,
        &auth.user.id,
        &org_id,
        &new_user_id,
        role,
    )
    .await?;

    Ok(Json(serde_json::json!(member)))
}

/// Update member role handler
pub async fn update_member_role_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((org_id, member_id)): Path<(Id, Id)>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let new_role: String = serde_json::from_value(
        request
            .get("role")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|_| AppError::BadRequest("Invalid role".to_string()))?;

    let role = match new_role.to_uppercase().as_str() {
        "OWNER" => cms_entity::common::MemberRole::Owner,
        "ADMIN" => cms_entity::common::MemberRole::Admin,
        "MEMBER" => cms_entity::common::MemberRole::Member,
        "GUEST" => cms_entity::common::MemberRole::Guest,
        _ => return Err(AppError::BadRequest("Invalid role value".to_string())),
    };

    let member = OrgService::update_member_role(
        &state.biz_context,
        &auth.user.id,
        &org_id,
        &member_id,
        role,
    )
    .await?;

    Ok(Json(serde_json::json!(member)))
}

/// Remove member handler
pub async fn remove_member_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((org_id, member_id)): Path<(Id, Id)>,
) -> Result<Json<serde_json::Value>, AppError> {
    OrgService::remove_member(&state.biz_context, &auth.user.id, &org_id, &member_id).await?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// Create invitation handler
pub async fn create_invitation_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(org_id): Path<Id>,
    Json(request): Json<CreateInvitationRequest>,
) -> Result<Json<InvitationResponse>, AppError> {
    let invitation =
        OrgService::create_invitation(&state.biz_context, &auth.user.id, &org_id, request).await?;

    Ok(Json(invitation))
}

/// List invitations handler
pub async fn list_invitations_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(org_id): Path<Id>,
) -> Result<Json<ListInvitationsResponse>, AppError> {
    let result =
        OrgService::list_invitations(&state.biz_context, &auth.user.id, &org_id, 1, 100).await?;

    Ok(Json(result))
}

/// Revoke invitation handler
pub async fn revoke_invitation_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((org_id, invitation_id)): Path<(Id, Id)>,
) -> Result<Json<serde_json::Value>, AppError> {
    OrgService::revoke_invitation(&state.biz_context, &auth.user.id, &org_id, &invitation_id)
        .await?;

    Ok(Json(serde_json::json!({"success": true})))
}
