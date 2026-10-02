//! Workspace and Members handlers
//!
//! Handlers for organization-level workspace settings, workspace analytics,
//! and workspace member management.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;

use crate::auth::AuthExtractor;

/// Get workspace settings
pub async fn get_workspace_settings_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<serde_json::Value>, AppError> {
    let members =
        cms_db::org::MemberQueries::get_by_user(&state.biz_context.pool, &auth.user.id).await?;
    let org_id = members
        .first()
        .map(|m| m.organization_id.clone())
        .unwrap_or_default();

    let org = if !org_id.is_empty() {
        cms_db::org::OrganizationQueries::get_by_id(&state.biz_context.pool, &org_id).await?
    } else {
        None
    };

    let (name, slug) = org
        .map(|o| (o.name, o.slug))
        .unwrap_or_else(|| ("Workspace".to_string(), "workspace".to_string()));

    let project_count = if !org_id.is_empty() {
        cms_db::project::ProjectQueries::count_by_organization(&state.biz_context.pool, &org_id, None, None)
            .await
            .unwrap_or(0)
    } else {
        0
    };

    let member_count = if !org_id.is_empty() {
        cms_db::org::MemberQueries::count_by_organization(&state.biz_context.pool, &org_id, None, None)
            .await
            .unwrap_or(1)
    } else {
        1
    };

    Ok(Json(serde_json::json!({
        "data": {
            "name": name,
            "slug": slug,
            "plan": "free",
            "notifications": {},
            "integrations": {},
            "git": {},
            "projectCount": project_count,
            "memberCount": member_count,
        }
    })))
}

/// Update workspace settings
pub async fn update_workspace_settings_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::org::OrganizationQueries;

    let members =
        cms_db::org::MemberQueries::get_by_user(&state.biz_context.pool, &auth.user.id).await?;
    let org_id = members
        .first()
        .map(|m| m.organization_id.clone())
        .ok_or_else(|| AppError::NotFound("No workspace organization found".to_string()))?;

    // Persist the editable workspace fields onto the owning organization.
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty());
    let logo = body
        .get("logo")
        .and_then(|v| v.as_str())
        .or_else(|| body.get("logoUrl").and_then(|v| v.as_str()));
    let description = body.get("description").and_then(|v| v.as_str());

    if name.is_some() || logo.is_some() || description.is_some() {
        let org = OrganizationQueries::update(
            &state.biz_context.pool,
            &org_id,
            name,
            description,
            logo,
        )
        .await?;
        let _ = org;
    }

    // Return the updated workspace settings so the SPA reflects the change.
    get_workspace_settings_handler(State(state), auth).await
}

/// Get workspace analytics
///
/// Populates the dashboard shape from the workspace's real analytics store. The
/// richer breakdown fields (top pages/referrers, AI tokens, search terms) stay
/// empty because the event store only aggregates counts; `totalViews` reflects the
/// number of recorded events for the organization.
pub async fn get_workspace_analytics_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<serde_json::Value>, AppError> {
    let org_id = resolve_workspace_org(&state, &auth.user.id).await?;
    let stats = cms_biz::analytics::AnalyticsService::get_organization_stats(
        &state.biz_context,
        &org_id,
    )
    .await
    .unwrap_or_else(|_| serde_json::json!({}));

    let total_views = stats
        .get("events")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .max(0);

    Ok(Json(serde_json::json!({
        "data": {
            "availability": "available",
            "totalViews": total_views,
            "uniqueVisitors": total_views,
            "viewsPreviousPeriod": 0,
            "visitorsPreviousPeriod": 0,
            "viewsChangePct": 0,
            "visitorsChangePct": 0,
            "avgDurationSeconds": 0,
            "timeseries": [],
            "topPages": [],
            "topReferrers": [],
            "topCountries": [],
            "topSearches": [],
            "referrers": [],
            "languages": [],
            "devices": [],
            "engagement": { "engagedViews": null, "averageEngagementMs": null },
            "searches": {
                "total": 0,
                "zeroResults": null,
                "clickedResults": null,
                "averageLatencyMs": null,
                "queryTerms": "legacy",
                "topTerms": []
            },
            "ai": {
                "answersCompleted": null,
                "answersFailed": null,
                "promptTokens": null,
                "completionTokens": null,
                "costMicros": null,
                "averageLatencyMs": null
            },
            "noAnswerReasons": []
        }
    })))
}

/// List workspace members
pub async fn list_workspace_members_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<serde_json::Value>, AppError> {
    let members =
        cms_db::org::MemberQueries::get_by_user(&state.biz_context.pool, &auth.user.id).await?;
    let org_id = members
        .first()
        .map(|m| m.organization_id.clone())
        .unwrap_or_default();

    if !org_id.is_empty() {
        let org_members = cms_db::org::MemberQueries::get_by_organization(
            &state.biz_context.pool,
            &org_id,
            None,
            None,
            Some(100),
            None,
        )
        .await?;

        let user_ids: Vec<&str> = org_members.iter().map(|m| m.user_id.as_str()).collect();
        let users = cms_db::auth::UserQueries::get_by_ids(&state.biz_context.pool, &user_ids)
            .await
            .unwrap_or_default();
        let user_map: std::collections::HashMap<String, cms_entity::auth::User> = users
            .into_iter()
            .map(|u| (u.id.clone(), u))
            .collect();

        let items: Vec<serde_json::Value> = org_members
            .into_iter()
            .map(|m| {
                let (user_name, user_email, user_image) = if let Some(u) = user_map.get(&m.user_id) {
                    (u.name.clone(), u.email.clone(), u.image.clone())
                } else {
                    (auth.user.name.clone(), auth.user.email.clone(), auth.user.image.clone())
                };
                serde_json::json!({
                    "id": m.id,
                    "organizationId": m.organization_id,
                    "userId": m.user_id,
                    "role": format!("{:?}", m.role).to_lowercase(),
                    "createdAt": m.created_at,
                    "user": {
                        "id": m.user_id,
                        "name": user_name,
                        "email": user_email,
                        "image": user_image,
                    }
                })
            })
            .collect();

        let invitations = cms_db::org::InvitationQueries::list_by_org(
            &state.biz_context.pool,
            &org_id,
        )
        .await
        .unwrap_or_default();

        return Ok(Json(serde_json::json!({
            "data": {
                "members": items,
                "invitations": invitations
            }
        })));
    }

    Ok(Json(serde_json::json!({
        "data": {
            "members": [],
            "invitations": []
        }
    })))
}

/// Resolve the caller's workspace organization (the first org they belong to).
async fn resolve_workspace_org(state: &Arc<AppState>, user_id: &str) -> Result<String, AppError> {
    let members = cms_db::org::MemberQueries::get_by_user(&state.biz_context.pool, user_id).await?;
    members
        .first()
        .map(|m| m.organization_id.clone())
        .ok_or_else(|| AppError::NotFound("No workspace organization found".to_string()))
}

/// Map an SPA workspace role string to the internal MemberRole.
fn parse_workspace_role(role: Option<&str>) -> cms_entity::common::MemberRole {
    use cms_entity::common::MemberRole;
    match role {
        Some("owner") => MemberRole::Owner,
        Some("admin") => MemberRole::Admin,
        Some("member") | Some("editor") => MemberRole::Member,
        _ => MemberRole::Member,
    }
}

/// Invite workspace member
pub async fn invite_workspace_member_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let org_id = resolve_workspace_org(&state, &auth.user.id).await?;

    let email = body
        .get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("email is required".to_string()))?
        .to_string();
    let role = parse_workspace_role(body.get("role").and_then(|v| v.as_str()));

    let request = cms_entity::org::CreateInvitationRequest { email, role };
    let invitation =
        cms_biz::org::OrgService::create_invitation(&state.biz_context, &auth.user.id, &org_id, request)
            .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": invitation.id,
            "organizationId": invitation.organization_id,
            "email": invitation.email,
            "role": format!("{:?}", invitation.role).to_lowercase(),
            "expiresAt": invitation.expires_at.to_rfc3339(),
            "createdAt": invitation.created_at.to_rfc3339(),
        }
    })))
}

/// Update workspace member role
pub async fn update_workspace_member_role_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let org_id = resolve_workspace_org(&state, &auth.user.id).await?;
    let role = parse_workspace_role(body.get("role").and_then(|v| v.as_str()));
    let member = cms_biz::org::OrgService::update_member_role(
        &state.biz_context,
        &auth.user.id,
        &org_id,
        &id,
        role,
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": member })))
}

/// Remove workspace member
pub async fn remove_workspace_member_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let org_id = resolve_workspace_org(&state, &auth.user.id).await?;
    cms_biz::org::OrgService::remove_member(&state.biz_context, &auth.user.id, &org_id, &id)
        .await?;
    Ok(Json(serde_json::json!({
        "data": { "success": true, "id": id }
    })))
}

/// Cancel workspace invitation
pub async fn cancel_workspace_invitation_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let org_id = resolve_workspace_org(&state, &auth.user.id).await?;
    cms_biz::org::OrgService::revoke_invitation(&state.biz_context, &auth.user.id, &org_id, &id)
        .await?;
    Ok(Json(serde_json::json!({
        "data": { "success": true, "id": id }
    })))
}
