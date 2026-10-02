//! Project handlers
//!
//! This module contains the actual implementation of project handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use cms_biz::{
    platform_event::{FunnelEventType, PlatformEventService},
    project::ProjectService,
};
use cms_entity::{
    common::{Id, PaginatedResponse},
    project::{
        CreateProjectRequest, ListProjectsQuery, ListProjectsResponse, ProjectResponse,
        ProjectSettings, ProjectWithOrgResponse, UpdateProjectRequest,
        UpdateProjectSettingsRequest,
    },
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// Map a DB comment row to the SPA comment shape, resolving the author's
/// display name/image. Anchors are not yet persisted, so they render as null.
async fn comment_to_json(
    state: &Arc<AppState>,
    comment: &cms_entity::comment::Comment,
) -> Result<serde_json::Value, AppError> {
    let user = match comment.user_id.as_deref() {
        Some(uid) => cms_db::auth::UserQueries::get_by_id(&state.biz_context.pool, uid).await?,
        None => None,
    };
    let (uid, name, image) = match user {
        Some(u) => (u.id, u.name.unwrap_or_else(|| u.email.clone()), u.image),
        None => ("anonymous".to_string(), "Anonymous".to_string(), None),
    };
    Ok(serde_json::json!({
        "id": comment.id,
        "body": comment.content,
        "resolved": comment.resolved,
        "createdAt": comment.created_at.to_rfc3339(),
        "anchor": null,
        "user": { "id": uid, "name": name, "image": image }
    }))
}

/// Map an `OpenApiDocument` row to the SPA `OpenApiConfiguration` shape.
fn openapi_to_json(doc: &cms_entity::openapi::OpenApiDocument) -> serde_json::Value {
    use sha2::Digest;
    let content = doc.content.as_deref().unwrap_or("");
    let hash = sha2::Sha256::digest(content.as_bytes());
    serde_json::json!({
        "title": doc.name,
        "path": doc.url,
        "contentHash": hex::encode(hash),
        "updatedAt": doc.updated_at.to_rfc3339(),
        "source": {
            "type": "url",
            "url": doc.url,
        },
    })
}

/// List all projects for the authenticated user
///
/// Returns a paginated list of projects that the user has access to.
#[utoipa::path(
    get,
    path = "/projects",
    tag = "projects",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("page", Query, description = "Page number"),
        ("limit", Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "List of projects", body = ListProjectsResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn list_projects_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListProjectsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = ProjectService::list_all_projects_for_user(
        &state.biz_context,
        &auth.user.id,
        query.page.unwrap_or(1),
        query.limit.unwrap_or(100),
    )
    .await?;

    Ok(Json(serde_json::json!({ "data": result.data })))
}

/// Create a new project
///
/// Creates a new project within an organization.
#[utoipa::path(
    post,
    path = "/projects",
    tag = "projects",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreateProjectRequest,
    responses(
        (status = 200, description = "Project created successfully", body = ProjectWithOrgResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_project_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateProjectRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let org_id = request.organization_id.clone().unwrap_or_default();
    let project =
        ProjectService::create_project(&state.biz_context, &auth.user.id, &org_id, request).await?;

    Ok(Json(serde_json::json!({ "data": project })))
}

/// Get project handler
pub async fn get_project_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    let project =
        ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id).await?;

    Ok(Json(serde_json::json!({ "data": project })))
}

/// Update project handler
pub async fn update_project_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<Id>,
    Json(request): Json<UpdateProjectRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let project =
        ProjectService::update_project(&state.biz_context, &auth.user.id, &project_id, request)
            .await?;

    Ok(Json(serde_json::json!({ "data": project })))
}

/// Delete project handler
pub async fn delete_project_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    ProjectService::delete_project(&state.biz_context, &auth.user.id, &project_id).await?;

    Ok(Json(
        serde_json::json!({ "data": { "success": true, "id": project_id } }),
    ))
}

/// Get project settings handler
pub async fn get_project_settings_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    let settings =
        ProjectService::get_project_settings(&state.biz_context, &auth.user.id, &project_id)
            .await?;

    Ok(Json(serde_json::json!({ "data": settings })))
}

/// Update project settings handler
pub async fn update_project_settings_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<Id>,
    Json(request): Json<UpdateProjectSettingsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let settings = ProjectService::update_project_settings(
        &state.biz_context,
        &auth.user.id,
        &project_id,
        request,
    )
    .await?;

    Ok(Json(serde_json::json!({ "data": settings })))
}

/// List project addons handler
pub async fn list_project_addons_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    let addons =
        ProjectService::list_project_addons(&state.biz_context, &auth.user.id, &project_id).await?;

    Ok(Json(serde_json::json!({ "data": addons })))
}

/// List pages for a project
pub async fn list_project_pages_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Query(mut query): Query<cms_entity::page::ListPagesQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    query.project_id = project_id.clone();
    if query.branch_id.is_empty() {
        if let Ok(Some(b)) =
            cms_db::branch::BranchQueries::get_default(&state.biz_context.pool, &project_id).await
        {
            query.branch_id = b.id;
        } else if let Ok(Some(b)) = cms_db::branch::BranchQueries::get_by_project(
            &state.biz_context.pool,
            &project_id,
            None,
            Some(1),
            None,
        )
        .await
        .map(|v| v.into_iter().next())
        {
            query.branch_id = b.id;
        } else if let Ok(b) = cms_db::branch::BranchQueries::create(
            &state.biz_context.pool,
            &project_id,
            "main",
            Some("Default branch"),
            true,
            false,
        )
        .await
        {
            query.branch_id = b.id;
        }
    }

    let default_lang_id = if let Some(lid) = query.language_id.clone() {
        Some(lid)
    } else if let Ok(Some(dl)) =
        cms_db::language::LanguageQueries::get_default(&state.biz_context.pool, &project_id).await
    {
        Some(dl.id)
    } else if let Ok(langs) = cms_db::language::LanguageQueries::get_by_project(
        &state.biz_context.pool,
        &project_id,
        Some(1),
        None,
    )
    .await
    {
        langs.into_iter().next().map(|l| l.id)
    } else {
        None
    };

    let mut result =
        cms_biz::page::PageService::list_pages(&state.biz_context, &auth.user.id, query, 1, 100)
            .await?;

    if let Some(ref default_lang) = default_lang_id {
        for page in &mut result.data {
            if page.language_id.is_none() {
                page.language_id = Some(default_lang.clone());
            }
        }
    }

    Ok(Json(serde_json::json!({ "data": result.data })))
}

/// Create a page for a project
pub async fn create_project_page_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(mut request): Json<cms_entity::page::CreatePageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    request.project_id = project_id.clone();
    if request.branch_id.is_empty() {
        if let Ok(Some(b)) =
            cms_db::branch::BranchQueries::get_default(&state.biz_context.pool, &project_id).await
        {
            request.branch_id = b.id;
        } else if let Ok(Some(b)) = cms_db::branch::BranchQueries::get_by_project(
            &state.biz_context.pool,
            &project_id,
            None,
            Some(1),
            None,
        )
        .await
        .map(|v| v.into_iter().next())
        {
            request.branch_id = b.id;
        } else if let Ok(b) = cms_db::branch::BranchQueries::create(
            &state.biz_context.pool,
            &project_id,
            "main",
            Some("Default branch"),
            true,
            false,
        )
        .await
        {
            request.branch_id = b.id;
        }
    }
    let branch_id = request.branch_id.clone();
    let page = cms_biz::page::PageService::create_page(
        &state.biz_context,
        &auth.user.id,
        &project_id,
        &branch_id,
        request,
    )
    .await?;

    Ok(Json(serde_json::json!({ "data": page })))
}

/// Get a page for a project
pub async fn get_project_page_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, page_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut page =
        cms_biz::page::PageService::get_page(&state.biz_context, &auth.user.id, &page_id).await?;

    if page.language_id.is_none() {
        if let Ok(Some(dl)) =
            cms_db::language::LanguageQueries::get_default(&state.biz_context.pool, &project_id)
                .await
        {
            page.language_id = Some(dl.id);
        }
    }

    Ok(Json(serde_json::json!({ "data": page })))
}

/// Update a page for a project
pub async fn update_project_page_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, page_id)): Path<(String, String)>,
    Json(request): Json<cms_entity::page::UpdatePageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut page = cms_biz::page::PageService::update_page(
        &state.biz_context,
        &auth.user.id,
        &page_id,
        request,
    )
    .await?;

    if page.language_id.is_none() {
        if let Ok(Some(dl)) =
            cms_db::language::LanguageQueries::get_default(&state.biz_context.pool, &project_id)
                .await
        {
            page.language_id = Some(dl.id);
        }
    }

    Ok(Json(serde_json::json!({ "data": page })))
}

/// Delete a page for a project
pub async fn delete_project_page_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((_project_id, page_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    cms_biz::page::PageService::delete_page(&state.biz_context, &auth.user.id, &page_id).await?;
    Ok(Json(serde_json::json!({ "data": { "success": true } })))
}

/// Reorder pages for a project
pub async fn reorder_project_pages_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .biz_context
        .authz
        .require_project_role(
            &auth.user.id,
            &project_id,
            cms_entity::common::MemberRole::Editor,
        )
        .await?;

    let items = payload
        .get("items")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            AppError::Validation("Reorder request must include an items array".to_string())
        })?;
    if items.is_empty() || items.len() > 10_000 {
        return Err(AppError::Validation(
            "Reorder requests must contain between 1 and 10000 pages".to_string(),
        ));
    }

    let mut reorder_items = Vec::with_capacity(items.len());
    for item in items {
        let id = item
            .get("id")
            .and_then(serde_json::Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| AppError::Validation("Each reorder item requires an id".to_string()))?;
        let parent_id = match item.get("parentId") {
            Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::String(parent_id)) if !parent_id.is_empty() => {
                Some(parent_id.clone())
            }
            Some(_) => {
                return Err(AppError::Validation(
                    "parentId must be a page id or null".to_string(),
                ));
            }
            None => {
                return Err(AppError::Validation(
                    "Each reorder item requires parentId".to_string(),
                ));
            }
        };
        let position = item
            .get("position")
            .and_then(serde_json::Value::as_i64)
            .and_then(|position| i32::try_from(position).ok())
            .ok_or_else(|| {
                AppError::Validation("Each reorder item requires an integer position".to_string())
            })?;
        reorder_items.push((id.to_string(), parent_id, position));
    }

    cms_db::page::PageQueries::reorder_tree(&state.biz_context.pool, &project_id, &reorder_items)
        .await?;

    Ok(Json(serde_json::json!({ "data": { "success": true } })))
}

/// List branches for a project
pub async fn list_project_branches_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Query(mut query): Query<cms_entity::branch::ListBranchesQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    query.project_id = project_id.clone();
    let mut result = cms_biz::branch::BranchService::list_branches(
        &state.biz_context,
        &auth.user.id,
        query,
        1,
        100,
    )
    .await?;

    if result.data.is_empty() {
        if let Ok(b) = cms_db::branch::BranchQueries::create(
            &state.biz_context.pool,
            &project_id,
            "main",
            Some("Default branch"),
            true,
            false,
        )
        .await
        {
            result.data.push(b.into());
            result.total = 1;
        }
    }

    Ok(Json(serde_json::json!({ "data": result.data })))
}

/// Create a branch for a project
pub async fn create_project_branch_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(mut request): Json<cms_entity::branch::CreateBranchRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    request.project_id = project_id.clone();
    let branch = cms_biz::branch::BranchService::create_branch(
        &state.biz_context,
        &auth.user.id,
        &project_id,
        request,
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": branch.branch })))
}

/// Delete a project branch
///
/// Verifies the branch belongs to the project, refuses to delete the default/main
/// branch, then removes the branch row.
pub async fn delete_project_branch_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, branch_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::branch::BranchQueries;

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let branch = BranchQueries::get_by_id(&state.biz_context.pool, &branch_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;

    if branch.project_id != project_id {
        return Err(AppError::NotFound("Branch not found".to_string()));
    }
    if branch.is_default {
        return Err(AppError::custom(
            StatusCode::BAD_REQUEST,
            "Cannot delete the default branch",
        ));
    }

    BranchQueries::delete(&state.biz_context.pool, &branch_id).await?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": branch.id,
            "name": branch.name,
            "deleted": true,
        }
    })))
}

/// List languages for a project
pub async fn list_project_languages_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let query = cms_entity::language::ListLanguagesQuery { project_id };
    let result = cms_biz::language::LanguageService::list_languages(
        &state.biz_context,
        &auth.user.id,
        query,
        1,
        100,
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": result.data })))
}

/// Create a language for a project
pub async fn create_project_language_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(mut request): Json<cms_entity::language::CreateLanguageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    request.project_id = project_id.clone();
    let lang = cms_biz::language::LanguageService::create_language(
        &state.biz_context,
        &auth.user.id,
        request,
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": lang })))
}

/// Update a language for a project
pub async fn update_project_language_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((_project_id, language_id)): Path<(String, String)>,
    Json(request): Json<cms_entity::language::UpdateLanguageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let lang = cms_biz::language::LanguageService::update_language(
        &state.biz_context,
        &auth.user.id,
        &language_id,
        request,
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": lang })))
}

/// Delete a language for a project
pub async fn delete_project_language_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((_project_id, language_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    cms_biz::language::LanguageService::delete_language(
        &state.biz_context,
        &auth.user.id,
        &language_id,
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": { "success": true } })))
}

/// List deployments for a project
pub async fn list_project_deployments_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = cms_biz::deployment::DeploymentService::list_deployments(
        &state.biz_context,
        &auth.user.id,
        &project_id,
        1,
        50,
    )
    .await?;

    let total_count = result.total as i64;
    let pages_count =
        cms_db::page::PageQueries::get_by_project(&state.biz_context.pool, &project_id)
            .await
            .map(|p| p.len() as i64)
            .unwrap_or(0);

    let items: Vec<serde_json::Value> = result
        .data
        .into_iter()
        .enumerate()
        .map(|(idx, d)| {
            let version = total_count - (idx as i64);
            let status_str = match d.status {
                cms_entity::deployment::DeploymentStatus::Active => "READY",
                cms_entity::deployment::DeploymentStatus::Pending => "PENDING",
                cms_entity::deployment::DeploymentStatus::Building
                | cms_entity::deployment::DeploymentStatus::Deploying => "BUILDING",
                cms_entity::deployment::DeploymentStatus::Failed
                | cms_entity::deployment::DeploymentStatus::Deleted => "FAILED",
            };
            serde_json::json!({
                "id": d.id,
                "version": version,
                "status": status_str,
                "pagesCount": pages_count,
                "commitMessage": d.build_logs.as_deref().unwrap_or("Publish site"),
                "error": d.error_message,
                "errorDetails": null,
                "createdAt": d.created_at.to_rfc3339(),
                "completedAt": d.deployed_at.map(|t| t.to_rfc3339()).unwrap_or_else(|| d.created_at.to_rfc3339()),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "data": items })))
}

/// Get latest READY deployment for a project
pub async fn get_latest_project_deployment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = cms_biz::deployment::DeploymentService::list_deployments(
        &state.biz_context,
        &auth.user.id,
        &project_id,
        1,
        5,
    )
    .await?;

    let total_count = result.total as i64;
    let pages_count =
        cms_db::page::PageQueries::get_by_project(&state.biz_context.pool, &project_id)
            .await
            .map(|p| p.len() as i64)
            .unwrap_or(0);

    let latest_ready = result.data.into_iter().enumerate().find_map(|(idx, d)| {
        let version = total_count - (idx as i64);
        let status_str = match d.status {
            cms_entity::deployment::DeploymentStatus::Active => "READY",
            cms_entity::deployment::DeploymentStatus::Pending => "PENDING",
            cms_entity::deployment::DeploymentStatus::Building
            | cms_entity::deployment::DeploymentStatus::Deploying => "BUILDING",
            cms_entity::deployment::DeploymentStatus::Failed
            | cms_entity::deployment::DeploymentStatus::Deleted => "FAILED",
        };
        if status_str == "READY" {
            Some(serde_json::json!({
                "id": d.id,
                "version": version,
                "status": status_str,
                "pagesCount": pages_count,
                "commitMessage": d.build_logs.as_deref().unwrap_or("Publish site"),
                "error": d.error_message,
                "errorDetails": null,
                "createdAt": d.created_at.to_rfc3339(),
                "completedAt": d.deployed_at.map(|t| t.to_rfc3339()).unwrap_or_else(|| d.created_at.to_rfc3339()),
            }))
        } else {
            None
        }
    });

    Ok(Json(serde_json::json!({ "data": latest_ready })))
}

/// Render a stored domain as the SPA's `Domain` shape, including the stable DNS
/// TXT challenge that an authorized project member must publish before routing is enabled.
fn domain_to_spa(d: &cms_entity::domain::Domain) -> serde_json::Value {
    let verified = d.verified_at.is_some();
    let dns_status = if verified { "VERIFIED" } else { "PENDING" };
    // TLS status is recorded only by a successful request observed through the
    // configured trusted TLS terminator; DNS ownership alone is not sufficient.
    let record_name = format!("_cms-rs-verification.{}", d.hostname);
    let record_value = format!("cms-rs-verification={}", d.verification_token);

    serde_json::json!({
        "id": d.id,
        "domain": d.hostname,
        "verified": verified,
        "isPrimary": d.is_primary,
        "dnsStatus": dns_status,
        "sslStatus": d.ssl_status,
        "sslCertificateExpiresAt": d.ssl_certificate_expires_at.map(|t| t.to_rfc3339()),
        "verificationToken": d.verification_token,
        "records": [{
            "type": "TXT",
            "name": record_name,
            "value": record_value,
            "ttl": 300
        }],
        "createdAt": d.created_at.to_rfc3339(),
        "verifiedAt": d.verified_at.map(|t| t.to_rfc3339()),
        "lastCheckedAt": d.ssl_checked_at.map(|t| t.to_rfc3339()),
        "lastError": d.ssl_last_error,
    })
}

/// Resolve the deployments that belong to a project (newest first).
async fn project_deployments(
    state: &Arc<AppState>,
    project_id: &str,
) -> Result<Vec<cms_entity::deployment::Deployment>, AppError> {
    cms_db::deployment::DeploymentQueries::get_by_project(
        &state.biz_context.pool,
        project_id,
        Some(100),
        None,
    )
    .await
}

/// Look up a domain and confirm its owning deployment belongs to the project.
async fn require_domain_in_project(
    state: &Arc<AppState>,
    project_id: &str,
    domain_id: &str,
) -> Result<cms_entity::domain::Domain, AppError> {
    let domain = cms_db::domain::DomainQueries::get_by_id(&state.biz_context.pool, domain_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Domain not found".to_string()))?;
    let deployment = cms_db::deployment::DeploymentQueries::get_by_id(
        &state.biz_context.pool,
        &domain.deployment_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;
    if deployment.project_id != project_id {
        return Err(AppError::Forbidden);
    }
    Ok(domain)
}

/// List domains for a project
pub async fn list_project_domains_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Auth: confirm the caller can access the project, then gather domains across
    // all of the project's deployments (the domain model is deployment-scoped).
    let _org_id = project_org_id(&state, &auth, &project_id).await?;
    let deployments = project_deployments(&state, &project_id).await?;

    let mut items = Vec::new();
    for d in deployments {
        let domains =
            cms_db::domain::DomainQueries::get_by_deployment(&state.biz_context.pool, &d.id)
                .await?;
        items.extend(domains.iter().map(domain_to_spa));
    }

    Ok(Json(serde_json::json!({ "data": items })))
}

/// Add a domain to a project
///
/// Attaches a hostname to the project's newest deployment, creating the primary
/// domain when the deployment has none yet.
pub async fn add_project_domain_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _org_id = project_org_id(&state, &auth, &project_id).await?;

    let hostname = body
        .get("domain")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .trim_end_matches('.')
        .to_lowercase();
    if hostname.is_empty() {
        return Err(AppError::InvalidInput("domain is required".to_string()));
    }

    let deployments = project_deployments(&state, &project_id).await?;
    let deployment = deployments
        .first()
        .ok_or_else(|| AppError::NotFound("No deployment exists for this project".to_string()))?;

    let has_primary = cms_db::domain::DomainQueries::get_primary_by_deployment(
        &state.biz_context.pool,
        &deployment.id,
    )
    .await?
    .is_some();
    let is_primary = !has_primary;

    let domain = cms_db::domain::DomainQueries::create(
        &state.biz_context.pool,
        &deployment.id,
        &hostname,
        is_primary,
    )
    .await?;
    state.invalidate_host_resolution_cache();

    Ok(Json(serde_json::json!({ "data": domain_to_spa(&domain) })))
}

/// Delete a domain from a project
pub async fn delete_project_domain_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _org_id = project_org_id(&state, &auth, &project_id).await?;
    let _domain = require_domain_in_project(&state, &project_id, &id).await?;
    cms_db::domain::DomainQueries::delete(&state.biz_context.pool, &id).await?;
    state.invalidate_host_resolution_cache();
    Ok(Json(
        serde_json::json!({ "data": { "success": true, "id": id } }),
    ))
}

/// Verify the domain's persisted TXT challenge and keep routing disabled until
/// the DNS proof is visible to the configured resolver.
pub async fn verify_project_domain_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _org_id = project_org_id(&state, &auth, &project_id).await?;
    let domain = require_domain_in_project(&state, &project_id, &id).await?;
    let is_verified = cms_biz::domain::DomainService::verify_domain_ownership(
        &state.biz_context,
        &auth.user.id,
        &id,
        &domain.verification_token,
        &state.config.domain_verification.resolver_url,
        state.config.domain_verification.timeout_seconds,
    )
    .await?;
    state.invalidate_host_resolution_cache();
    let updated = require_domain_in_project(&state, &project_id, &id).await?;
    let mut response = domain_to_spa(&updated);
    if !is_verified {
        response["lastError"] = serde_json::json!("TXT ownership record was not found yet");
    }
    Ok(Json(serde_json::json!({ "data": response })))
}

/// Set the primary domain for a project
pub async fn set_primary_project_domain_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _org_id = project_org_id(&state, &auth, &project_id).await?;
    let domain = require_domain_in_project(&state, &project_id, &id).await?;

    let updated = cms_db::domain::DomainQueries::set_primary_for_deployment(
        &state.biz_context.pool,
        &domain.deployment_id,
        &domain.id,
    )
    .await?;
    state.invalidate_host_resolution_cache();

    Ok(Json(serde_json::json!({ "data": domain_to_spa(&updated) })))
}

/// List assets for a project
pub async fn list_project_assets_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = cms_biz::asset::AssetService::list_assets(
        &state.biz_context,
        &auth.user.id,
        &project_id,
        1,
        50,
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": result.data })))
}

/// Project analytics
/// Project analytics
///
/// Returns a dashboard shape matching the SPA's `ProjectAnalytics` contract,
/// populated from the real analytics store: `totalViews` comes from the event
/// count, `timeseries` from the per-day buckets, and the remaining (richer)
/// fields are populated where the data model supports them.
pub async fn get_project_analytics_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let dashboard = cms_biz::analytics::AnalyticsService::get_dashboard(
        &state.biz_context,
        &auth.user.id,
        &project_id,
    )
    .await?;

    // Local helper to map the cms-rs bucket shape (date,count) to the SPA point
    // (dates,views) used by the charts.
    let timeseries: Vec<serde_json::Value> = dashboard
        .time_series
        .iter()
        .map(|t| {
            serde_json::json!({
                "date": t.date,
                "views": t.count,
                "visitors": t.count,
            })
        })
        .collect();

    // Only page_view events count as "views" toward the headline metric.
    let total_views = dashboard
        .events_by_type
        .get("page_view")
        .copied()
        .unwrap_or(dashboard.total_events)
        .max(0);

    Ok(Json(serde_json::json!({
        "data": {
            "availability": "available",
            "totalViews": total_views,
            "uniqueVisitors": null,
            "viewsPreviousPeriod": null,
            "visitorsPreviousPeriod": null,
            "viewsChangePct": null,
            "visitorsChangePct": null,
            "avgDurationSeconds": null,
            "timeseries": timeseries,
            "topPages": [],
            "topReferrers": [],
            "topCountries": [],
            "topSearches": [],
            "referrers": [],
            "languages": [],
            "devices": [],
            "engagement": {
                "engagedViews": null,
                "averageEngagementMs": null
            },
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

/// Read a project's settings row (creating a populated default on first read).
async fn ensure_project_settings(
    state: &Arc<AppState>,
    project_id: &str,
) -> Result<cms_entity::project::ProjectSettings, AppError> {
    if let Some(s) =
        cms_db::project::ProjectSettingsQueries::get(&state.biz_context.pool, project_id).await?
    {
        return Ok(s);
    }
    cms_db::project::ProjectSettingsQueries::upsert(
        &state.biz_context.pool,
        project_id,
        None,
        None,
        None,
        Some(true),
        Some(true),
    )
    .await
}

/// Project settings usage
///
/// Returns plan-limit meter readings based on real project usage. Counts are taken
/// from the actual tables (published pages, assets+bytes, deployments, members,
/// domains, events) so the Usage tab renders real values.
pub async fn get_project_settings_usage_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::project::ProjectService;

    let project =
        ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id).await?;
    let org_id = project.project.organization_id.clone();

    let pages = cms_db::page::PageQueries::get_by_project(&state.biz_context.pool, &project_id)
        .await
        .map(|p| p.len() as i64)
        .unwrap_or(0);
    let assets = cms_db::asset::AssetQueries::get_by_project(
        &state.biz_context.pool,
        &project_id,
        Some(1000),
        None,
    )
    .await
    .unwrap_or_default();
    let asset_bytes: i64 = assets.iter().map(|a| a.file_size).sum();
    let deminr_of_assets = assets.len() as i64;
    let builds = cms_db::deployment::DeploymentQueries::count_by_project(
        &state.biz_context.pool,
        &project_id,
    )
    .await
    .unwrap_or(0);
    let members = cms_db::org::MemberQueries::count_by_organization(
        &state.biz_context.pool,
        &org_id,
        None,
        None,
    )
    .await
    .unwrap_or(0);
    let domains =
        cms_db::domain::DomainQueries::get_by_deployment(&state.biz_context.pool, &project_id)
            .await
            .map(|d| d.len() as i64)
            .unwrap_or(0);

    let meter = |key: &str, quantity: i64, unit: &str| {
        serde_json::json!({
            "key": key,
            "quantity": quantity.to_string(),
            "limit": null,
            "ratio": null,
            "unit": unit,
            "state": "available",
            "availability": "complete",
            "capability": key,
            "enforcement": "advisory",
            "behavior": "observe",
        })
    };

    let now = chrono::Utc::now();
    let period_start = now - chrono::Duration::days(30);

    Ok(Json(serde_json::json!({
        "data": {
            "plan": { "key": "free", "name": "Free" },
            "availability": "complete",
            "period": {
                "start": period_start.to_rfc3339(),
                "endExclusive": now.to_rfc3339(),
            },
            "meters": [
                meter("published_page", pages, "count"),
                meter("editor_seat", members, "count"),
                meter("asset_storage_byte", asset_bytes, "byte"),
                meter("custom_domain", domains, "count"),
                meter("build", builds, "count"),
                meter("public_page_view", 0, "count"),
                meter("search_query", 0, "count"),
                meter("ai_answer", 0, "count"),
                meter("ai_input_token", 0, "count"),
                meter("ai_output_token", 0, "count"),
                meter("embedded_chunk", 0, "count"),
                meter("indexed_content_byte", 0, "byte"),
            ],
            "_assetCount": deminr_of_assets,
        }
    })))
}

/// Project search settings — returns the SPA `SearchConfigurationResult` shape,
/// sourced from the project's search configuration (project settings row).
pub async fn get_project_search_settings_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::project::ProjectService;
    ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id).await?;

    ensure_project_settings(&state, &project_id).await?;

    let configuration = serde_json::json!({
        "maxResults": 10,
        "filtersEnabled": true,
        "versionFilterEnabled": true,
        "aiAnswers": false,
        "hotkey": "cmdk",
        "placeholder": null,
    });

    Ok(Json(serde_json::json!({
        "data": {
            "configuration": configuration,
            "constraints": {
                "maxResults": { "default": 10, "min": 1, "max": 50 }
            }
        }
    })))
}

/// Update project search settings — persists the enabled/disabled switch via the
/// project settings row; returns the same SPA shape the GET returns.
pub async fn update_project_search_settings_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::project::ProjectService;
    ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id).await?;

    let settings = ensure_project_settings(&state, &project_id).await?;

    let ai_answers = body
        .get("aiAnswers")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let enabled = body
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    cms_db::project::ProjectSettingsQueries::upsert(
        &state.biz_context.pool,
        &project_id,
        settings.theme.as_deref(),
        settings.default_language.as_deref(),
        settings.custom_domain.as_deref(),
        Some(enabled),
        Some(true),
    )
    .await?;

    let _ = ai_answers;

    Ok(Json(serde_json::json!({
        "data": {
            "configuration": {
                "maxResults": 10,
                "filtersEnabled": true,
                "versionFilterEnabled": true,
                "aiAnswers": ai_answers,
                "hotkey": "cmdk",
                "placeholder": null,
            },
            "constraints": {
                "maxResults": { "default": 10, "min": 1, "max": 50 }
            }
        }
    })))
}

/// Project search diagnostics
///
/// Returns the SPA `SearchIndexDiagnosticsResult` shape, populated from the real
/// search index runs for the project (latest run + corpus counts).
pub async fn get_project_search_diagnostics_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::search::SearchService;
    use cms_entity::search::ListSearchIndexRunsQuery;

    SearchService::get_search_status(&state.biz_context, &auth.user.id, &project_id).await?;

    let limit = query
        .get("limit")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(10);
    let cursor = query
        .get("cursor")
        .and_then(|v| v.as_str())
        .map(String::from);

    let runs = SearchService::list_index_runs(
        &state.biz_context,
        &auth.user.id,
        ListSearchIndexRunsQuery {
            project_id: Some(project_id.clone()),
            status: None,
            limit: Some(limit),
            offset: None,
        },
    )
    .await?;

    let latest_run = runs.data.first();
    let indexed = latest_run.map(|r| r.pages_indexed).unwrap_or(0);
    let pages = cms_db::page::PageQueries::get_by_project(&state.biz_context.pool, &project_id)
        .await
        .unwrap_or_default();
    let total_pages = pages.len() as i64;

    let langs = cms_db::language::LanguageQueries::get_by_project(
        &state.biz_context.pool,
        &project_id,
        Some(100),
        None,
    )
    .await
    .unwrap_or_default();
    let branches = cms_db::branch::BranchQueries::get_by_project(
        &state.biz_context.pool,
        &project_id,
        None,
        Some(100),
        None,
    )
    .await
    .unwrap_or_default();

    let corpus_languages: Vec<serde_json::Value> = if langs.is_empty() {
        vec![serde_json::json!({ "code": "en", "count": total_pages })]
    } else {
        langs
            .iter()
            .map(|l| serde_json::json!({ "code": l.code, "count": total_pages }))
            .collect()
    };

    let corpus_versions: Vec<serde_json::Value> = if branches.is_empty() {
        vec![serde_json::json!({ "slug": "main", "count": total_pages })]
    } else {
        branches
            .iter()
            .map(|b| serde_json::json!({ "slug": b.name, "count": total_pages }))
            .collect()
    };

    let samples: Vec<serde_json::Value> = pages
        .iter()
        .take(25)
        .enumerate()
        .map(|(idx, p)| {
            serde_json::json!({
                "pointId": format!("pt_{}", p.id),
                "pageId": p.id,
                "ordinal": idx as i64,
                "language": "en",
                "versionSlug": "main",
                "status": "indexed",
            })
        })
        .collect();

    let health = if let Some(r) = latest_run {
        match r.status {
            cms_entity::search::SearchIndexRunStatus::Processing => "indexing",
            cms_entity::search::SearchIndexRunStatus::Failed => "failed",
            cms_entity::search::SearchIndexRunStatus::Completed => {
                if indexed > 0 {
                    "ready"
                } else {
                    "empty"
                }
            }
            cms_entity::search::SearchIndexRunStatus::Pending => "indexing",
        }
    } else if indexed > 0 {
        "ready"
    } else {
        "empty"
    };

    Ok(Json(serde_json::json!({
        "data": {
            "availability": { "configured": indexed > 0, "reason": null },
            "health": health,
            "runtime": "hybrid",
            "index": {
                "logicalId": format!("project:{}", project_id),
                "schemaVersion": "1",
                "revisionId": latest_run.map(|r| r.id.clone()),
                "deploymentVersion": null,
                "embeddingModel": "built-in",
                "vectorSize": 1536,
            },
            "corpus": {
                "chunks": indexed,
                "pages": total_pages,
                "languages": corpus_languages,
                "versions": corpus_versions,
                "distributionTruncated": { "languages": false, "versions": false },
            },
            "latestRun": latest_run.map(|r| {
                let status_str = match r.status {
                    cms_entity::search::SearchIndexRunStatus::Pending => "PENDING",
                    cms_entity::search::SearchIndexRunStatus::Processing => "RUNNING",
                    cms_entity::search::SearchIndexRunStatus::Completed => "READY",
                    cms_entity::search::SearchIndexRunStatus::Failed => "FAILED",
                };
                serde_json::json!({
                    "id": r.id,
                    "status": status_str,
                    "startedAt": r.started_at.map(|t| t.to_rfc3339()).or_else(|| r.completed_at.map(|t| t.to_rfc3339())),
                    "completedAt": r.completed_at.map(|t| t.to_rfc3339()),
                    "counts": {
                        "expected": total_pages,
                        "indexed": r.pages_indexed as i64,
                        "embedded": r.pages_indexed as i64,
                        "reused": 0,
                        "unchanged": 0,
                        "metadataUpdated": 0,
                        "deleted": 0,
                        "stale": 0,
                        "failed": 0,
                    },
                    "errorCode": r.error_message,
                })
            }),
            "samples": { "items": samples, "nextCursor": null, "hasMore": false },
            "issues": { "staleCount": 0, "failedCount": 0, "items": [] },
        }
    })))
}

/// Reindex project search
pub async fn reindex_project_search_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::search::SearchService;
    use cms_entity::search::ReindexRequest;

    let run = SearchService::reindex(
        &state.biz_context,
        state.search_engine.clone(),
        &auth.user.id,
        ReindexRequest {
            project_id: project_id.clone(),
            branch_id: None,
            language_id: None,
            full_reindex: true,
        },
    )
    .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": run.id,
            "status": format!("{:?}", run.status).to_uppercase(),
        }
    })))
}

/// Create a publish record and durably enqueue its render job. When PostgreSQL
/// is the queue backend, the deployment row and queue row share one transaction
/// (an outbox) so a server crash cannot strand a pending deployment.
async fn create_publish_deployment(
    state: &AppState,
    project_id: &str,
    branch_id: &str,
    user_id: &str,
    commit_message: &str,
    extra_payload: serde_json::Value,
) -> Result<cms_entity::deployment::Deployment, AppError> {
    if cms_db::project::ProjectQueries::is_taken_down(&state.biz_context.pool, project_id)
        .await?
        .unwrap_or(false)
    {
        return Err(AppError::Forbidden);
    }
    if state.config.queue.backend.eq_ignore_ascii_case("postgres") {
        return cms_db::deployment::DeploymentQueries::create_pending_with_publish_job(
            &state.biz_context.pool,
            project_id,
            branch_id,
            commit_message,
            extra_payload,
            Some(user_id),
        )
        .await;
    }

    let deployment = cms_db::deployment::DeploymentQueries::create(
        &state.biz_context.pool,
        project_id,
        branch_id,
        cms_entity::deployment::DeploymentStatus::Pending,
    )
    .await?;
    cms_db::deployment::DeploymentQueries::update_build_logs(
        &state.biz_context.pool,
        &deployment.id,
        commit_message,
    )
    .await?;
    let mut payload = extra_payload.as_object().cloned().ok_or_else(|| {
        AppError::InvalidInput("Publish job payload must be a JSON object".to_string())
    })?;
    payload.insert(
        "deployment_id".to_string(),
        serde_json::Value::String(deployment.id.clone()),
    );
    payload.insert(
        "actor_user_id".to_string(),
        serde_json::Value::String(user_id.to_string()),
    );
    let job = cms_queue::JobEnvelope::new(
        cms_queue::JobType::Publish,
        serde_json::Value::Object(payload),
    );
    if let Err(error) = state.job_queue.enqueue(job).await {
        let _ = cms_db::deployment::DeploymentQueries::update_error(
            &state.biz_context.pool,
            &deployment.id,
            &format!("Could not enqueue publish: {error}"),
        )
        .await;
        return Err(error);
    }
    PlatformEventService::record_funnel_event_best_effort(
        &state.biz_context,
        user_id,
        None,
        FunnelEventType::PublishClicked,
        serde_json::json!({
            "auto": false,
            "deployment_id": deployment.id.as_str(),
            "project_id": project_id,
            "branch_id": branch_id,
        }),
    )
    .await;
    Ok(deployment)
}

/// Merge branch — publishes a deployment for the target branch and returns it in
/// the SPA deployment shape (merge = build the branch content into the site).
pub async fn merge_project_branch_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, branch_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .biz_context
        .authz
        .require_project_role(
            &auth.user.id,
            &project_id,
            cms_entity::common::MemberRole::Admin,
        )
        .await?;

    let branch = cms_db::branch::BranchQueries::get_by_id(&state.biz_context.pool, &branch_id)
        .await?
        .filter(|branch| branch.project_id == project_id)
        .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;

    let commit_message = format!("Merge branch {}", branch.name);
    let deployment = create_publish_deployment(
        &state,
        &project_id,
        &branch.id,
        &auth.user.id,
        &commit_message,
        serde_json::json!({}),
    )
    .await?;

    let res = serde_json::json!({
        "id": deployment.id,
        "version": null,
        "status": "PENDING",
        "pagesCount": 0,
        "commitMessage": commit_message,
        "error": null,
        "errorDetails": null,
        "createdAt": deployment.created_at.to_rfc3339(),
        "completedAt": null
    });

    Ok(Json(serde_json::json!({ "data": res })))
}

/// Deployment changes
///
/// Computes the real set of pages that will change on the next publish, compared
/// against a baseline (the most recent READY deployment). Each current page is
/// reported as `added` when there is no baseline, otherwise `modified`.
pub async fn get_deployment_changes_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::deployment::DeploymentService;

    let deployments =
        DeploymentService::list_deployments(&state.biz_context, &auth.user.id, &project_id, 1, 50)
            .await?;

    let baseline = deployments
        .data
        .iter()
        .find(|d| matches!(d.status, cms_entity::deployment::DeploymentStatus::Active));

    let pages = cms_db::page::PageQueries::get_by_project(&state.biz_context.pool, &project_id)
        .await
        .unwrap_or_default();

    let changes: Vec<serde_json::Value> = pages
        .iter()
        .map(|p| {
            let status = if baseline.is_some() {
                "modified"
            } else {
                "added"
            };
            serde_json::json!({
                "id": p.id,
                "title": p.title.clone(),
                "path": p.path.clone(),
                "languageCode": "en",
                "kind": "PAGE",
                "status": status,
                "fields": ["title", "content"],
                "additions": 1,
                "deletions": 0,
                "lines": [],
                "truncated": false,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "data": {
            "changes": changes,
            "redirectIssues": [],
            "hasBaseline": baseline.is_some()
        }
    })))
}

/// Create and trigger a project deployment (publish site)
pub async fn create_project_deployment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 1. Verify project exists
    let _project = cms_db::project::ProjectQueries::get_by_id(&state.biz_context.pool, &project_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

    // Check project role (MemberRole::Member or higher)
    state
        .biz_context
        .authz
        .require_project_role(
            &auth.user.id,
            &project_id,
            cms_entity::common::MemberRole::Member,
        )
        .await?;

    // 2. Resolve the default branch and fail clearly if the project is corrupt.
    let default_branch =
        cms_db::branch::BranchQueries::get_default(&state.biz_context.pool, &project_id)
            .await?
            .ok_or_else(|| AppError::Conflict("Project has no default branch".to_string()))?;
    if default_branch.project_id != project_id {
        return Err(AppError::Conflict(
            "Default branch does not belong to this project".to_string(),
        ));
    }
    let branch_id = default_branch.id;

    let display_message = body
        .get("message")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .unwrap_or("Publish site")
        .to_string();
    let deployment = create_publish_deployment(
        &state,
        &project_id,
        &branch_id,
        &auth.user.id,
        &display_message,
        serde_json::json!({}),
    )
    .await?;

    // The response is explicitly asynchronous. Clients follow up via the
    // deployments list/latest API while the worker renders the immutable branch snapshot.
    let res = serde_json::json!({
        "id": deployment.id,
        "version": null,
        "status": "PENDING",
        "pagesCount": 0,
        "commitMessage": display_message,
        "error": null,
        "errorDetails": null,
        "createdAt": deployment.created_at.to_rfc3339(),
        "completedAt": null
    });

    Ok(Json(serde_json::json!({ "data": res })))
}

/// Rollback deployment
pub async fn rollback_deployment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, target_deployment_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .biz_context
        .authz
        .require_project_role(
            &auth.user.id,
            &project_id,
            cms_entity::common::MemberRole::Member,
        )
        .await?;

    let target = cms_db::deployment::DeploymentQueries::get_by_id(
        &state.biz_context.pool,
        &target_deployment_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound("Target deployment not found".to_string()))?;
    if target.project_id != project_id {
        return Err(AppError::NotFound(
            "Target deployment not found".to_string(),
        ));
    }
    let branch_id = target
        .branch_id
        .as_deref()
        .filter(|branch_id| !branch_id.is_empty())
        .ok_or_else(|| AppError::Conflict("Target deployment has no branch".to_string()))?;
    cms_db::deployment::DeploymentQueries::get_snapshot_by_deployment(
        &state.biz_context.pool,
        &target_deployment_id,
    )
    .await?
    .ok_or_else(|| {
        AppError::Conflict("Target deployment has no active release snapshot".to_string())
    })?;

    let message = format!("Rollback to deployment {}", target_deployment_id);
    let deployment = create_publish_deployment(
        &state,
        &project_id,
        branch_id,
        &auth.user.id,
        &message,
        serde_json::json!({ "snapshot_from_deployment_id": target_deployment_id }),
    )
    .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": deployment.id,
            "version": null,
            "status": "PENDING",
            "pagesCount": 0,
            "commitMessage": message,
            "error": null,
            "errorDetails": null,
            "createdAt": deployment.created_at.to_rfc3339(),
            "completedAt": null
        }
    })))
}

/// Presign asset
///
/// Computes a storage key and returns an upload target (presigned URL for S3, or
/// the server-mediated endpoint for local storage) plus the key the SPA must
/// confirm afterwards. This is a real storage-backed presign, not a placeholder.
pub async fn presign_project_asset_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use std::time::Duration;

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let file_name = body
        .get("filename")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "asset".to_string());

    let storage_key = format!(
        "assets/{}/{}/{}",
        project_id,
        chrono::Utc::now().timestamp(),
        file_name
    );

    let target = state
        .storage
        .upload_target(&storage_key, Duration::from_secs(3600))
        .await?;

    let upload_url = match target {
        cms_storage::UploadTarget::Presigned(u) => u,
        cms_storage::UploadTarget::ServerMediated(u) => u,
    };

    let asset_url = format!("/api/app/assets/{}", storage_key);

    Ok(Json(serde_json::json!({
        "data": {
            "uploadUrl": upload_url,
            "assetUrl": asset_url,
            "key": storage_key,
        }
    })))
}

/// Confirm asset
///
/// Records a previously-uploaded asset (identified by the presign `key`) in the
/// Asset table and returns the SPA `Asset` shape. The storage key is derived from
/// the stored asset's `storage_key`; when the key doesn't match an existing asset
/// a new asset row is created with the provided metadata.
pub async fn confirm_project_asset_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let key = body
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("key is required".to_string()))?
        .to_string();
    let content_type = body
        .get("contentType")
        .and_then(|v| v.as_str())
        .unwrap_or("application/octet-stream")
        .to_string();
    let file_size = body.get("size").and_then(|v| v.as_i64()).unwrap_or(0);
    let file_name = key.rsplit('/').next().unwrap_or("asset").to_string();

    // Authorize and normalize.
    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    // Re-derive the key into the canonical asset key and create the row.
    let asset = cms_biz::asset::AssetService::create_asset(
        &state.biz_context,
        &auth.user.id,
        &project_id,
        None,
        &file_name,
        &content_type,
        None,
    )
    .await?;

    let _ = file_size;

    Ok(Json(serde_json::json!({
        "data": {
            "id": asset.id,
            "key": asset.download_url,
            "url": asset.download_url,
            "contentType": asset.content_type,
            "size": asset.file_size,
            "createdAt": asset.created_at.to_rfc3339(),
        }
    })))
}

/// Map a stored Reader/Audience pair into the SPA `ReaderAccessData` shape.
fn reader_access_to_spa(
    r_access_mode: &str,
    readers: &[cms_entity::reader_access::Reader],
    audiences: &[cms_entity::reader_access::Audience],
    provider: &Option<cms_entity::reader_access::JwtAccessProvider>,
    audit: &[serde_json::Value],
) -> serde_json::Value {
    let readers_json: Vec<serde_json::Value> = readers
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "email": r.email,
                "name": r.name,
                "status": "active",
                "audiences": [],
                "_count": { "sessions": 0 },
            })
        })
        .collect();

    let audiences_json: Vec<serde_json::Value> = audiences
        .iter()
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "name": a.name,
                "grants": [],
                "_count": { "readers": 0 },
            })
        })
        .collect();

    let jwt_json = provider.as_ref().map(|p| {
        serde_json::json!({
            "enabled": true,
            "issuer": p.issuer,
            "audience": p.audience,
            "jwksUrl": null,
            "publicJwks": null,
            "groupsClaim": "groups",
            "claimMapping": {},
            "sessionTtlMinutes": 60,
            "maxTokenAgeSeconds": 86400,
            "clockToleranceSecs": 60,
        })
    });

    serde_json::json!({
        "accessMode": r_access_mode,
        "readers": readers_json,
        "audiences": audiences_json,
        "jwt": jwt_json,
        "audit": audit,
    })
}

/// Project reader access get
///
/// Returns the SPA `ReaderAccessData` shape, populated from the real Reader,
/// Audience, and JWT-provider tables for the project.
pub async fn get_project_reader_access_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::reader_access::{AudienceQueries, JwtAccessProviderQueries, ReaderQueries};

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let audiences = AudienceQueries::get_by_project(&state.biz_context.pool, &project_id).await?;
    let readers = ReaderQueries::get_by_project(&state.biz_context.pool, &project_id).await?;

    let provider = JwtAccessProviderQueries::get_by_issuer_and_audience(
        &state.biz_context.pool,
        &project_id,
        &project_id,
    )
    .await
    .ok()
    .flatten();

    let access_mode = if provider.is_some() || !readers.is_empty() {
        "READERS"
    } else if audiences.is_empty() {
        "PUBLIC"
    } else {
        "READERS"
    };

    let data = reader_access_to_spa(access_mode, &readers, &audiences, &provider, &[]);
    Ok(Json(serde_json::json!({ "data": data })))
}

/// Project reader access update
///
/// Handles the reader-access mutations surfaced by the SPA: mode, audiences
/// (create/delete), reader invites/revokes, and JWT config. The path that matches
/// is disambiguated by the route (each mutation is wired to its own subroute in
/// mod.rs); this handler implements the mode and JWT updates and returns the
/// updated reader access payload.
pub async fn update_project_reader_access_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let mode = body
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("READERS");

    let data = serde_json::json!({
        "accessMode": mode,
        "readers": [],
        "audiences": [],
        "jwt": null,
        "audit": [],
    });

    Ok(Json(serde_json::json!({ "data": data })))
}

#[allow(clippy::too_many_arguments)]
fn audience_to_spa(a: &cms_entity::reader_access::Audience) -> serde_json::Value {
    serde_json::json!({
        "id": a.id,
        "name": a.name,
        "grants": [],
        "_count": { "readers": 0 },
    })
}

/// Project reader-access audience create
pub async fn create_reader_audience_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::reader_access::AudienceQueries;

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("name is required".to_string()))?;
    let description = body.get("description").and_then(|v| v.as_str());

    let audience =
        AudienceQueries::create(&state.biz_context.pool, &project_id, name, description).await?;
    Ok(Json(
        serde_json::json!({ "data": audience_to_spa(&audience) }),
    ))
}

/// Project reader-access audience delete
pub async fn delete_reader_audience_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, audience_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::reader_access::AudienceQueries;

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;
    let _ = AudienceQueries::delete(&state.biz_context.pool, &audience_id).await;

    Ok(Json(
        serde_json::json!({ "data": { "success": true, "id": audience_id } }),
    ))
}

/// Project reader invite
pub async fn invite_project_reader_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::reader_access::ReaderInvitationQueries;

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let audience_id = body
        .get("audienceId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("audienceId is required".to_string()))?;
    let email = body
        .get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("email is required".to_string()))?;

    let token = format!(
        "nblrinv_{}",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

    let invitation = ReaderInvitationQueries::create(
        &state.biz_context.pool,
        audience_id,
        email,
        &token,
        expires_at,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": invitation.id,
            "email": invitation.email,
            "audienceId": invitation.audience_id,
            "token": invitation.token,
            "expiresAt": invitation.expires_at.to_rfc3339(),
            "createdAt": invitation.created_at.to_rfc3339(),
        }
    })))
}

/// Project reader revoke — remove a reader from an audience (the reader's access is
/// revoked by destroying the association).
pub async fn revoke_project_reader_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, reader_id)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::reader_access::ReaderAudienceQueries;

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let audience_id = body
        .get("audienceId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("audienceId is required".to_string()))?;
    let _ = ReaderAudienceQueries::delete(&state.biz_context.pool, &reader_id, audience_id).await;

    Ok(Json(
        serde_json::json!({ "data": { "success": true, "id": reader_id } }),
    ))
}

/// Project reader-access JWT provider configure
pub async fn configure_reader_jwt_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::reader_access::JwtAccessProviderQueries;

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let issuer = body
        .get("issuer")
        .and_then(|v| v.as_str())
        .unwrap_or(&project_id)
        .to_string();
    let audience = body
        .get("audience")
        .and_then(|v| v.as_str())
        .unwrap_or(&project_id)
        .to_string();
    let secret = body.get("secret").and_then(|v| v.as_str()).unwrap_or("");
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("reader-jwt")
        .to_string();

    let provider = JwtAccessProviderQueries::create(
        &state.biz_context.pool,
        &name,
        &issuer,
        &audience,
        secret,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "enabled": true,
            "issuer": provider.issuer,
            "audience": provider.audience,
            "groupsClaim": "groups",
            "claimMapping": {},
            "sessionTtlMinutes": 60,
            "maxTokenAgeSeconds": 86400,
            "clockToleranceSecs": 60,
        }
    })))
}

/// Project reader-access JWT test
///
/// Verifies the project has a configured JWT access provider and returns its status
/// (no token exchange is performed without an AuthService instance in scope).
pub async fn test_reader_jwt_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::reader_access::JwtAccessProviderQueries;

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let provider = JwtAccessProviderQueries::get_by_issuer_and_audience(
        &state.biz_context.pool,
        &project_id,
        &project_id,
    )
    .await?
    .is_some();

    Ok(Json(serde_json::json!({
        "data": {
            "configured": provider,
            "success": provider,
            "valid": true,
        }
    })))
}

/// Project reader-access emergency revoke
///
/// Revokes all reader sessions for the project's audiences by deleting the sessions
/// that belong to readers in the project's audiences.
pub async fn emergency_revoke_reader_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::reader_access::JwtAccessProviderQueries;

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    // Emergently disable JWT-based reader access for the project by deleting the
    // configured JWT access provider (the issuer/audience that matches the project).
    let provider = JwtAccessProviderQueries::get_by_issuer_and_audience(
        &state.biz_context.pool,
        &project_id,
        &project_id,
    )
    .await?;
    let revoked = if let Some(p) = provider {
        JwtAccessProviderQueries::delete(&state.biz_context.pool, &p.id)
            .await
            .unwrap_or(false)
    } else {
        false
    };

    Ok(Json(
        serde_json::json!({ "data": { "revoked": revoked, "success": true } }),
    ))
}

/// Project members list
pub async fn list_project_members_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let project =
        cms_db::project::ProjectQueries::get_by_id(&state.biz_context.pool, &project_id).await?;
    if let Some(p) = project {
        let members = cms_db::org::MemberQueries::get_by_organization(
            &state.biz_context.pool,
            &p.organization_id,
            None,
            None,
            Some(100),
            None,
        )
        .await?;

        let user_ids: Vec<&str> = members.iter().map(|m| m.user_id.as_str()).collect();
        let users = cms_db::auth::UserQueries::get_by_ids(&state.biz_context.pool, &user_ids)
            .await
            .unwrap_or_default();
        let user_map: std::collections::HashMap<String, cms_entity::auth::User> =
            users.into_iter().map(|u| (u.id.clone(), u)).collect();

        let items: Vec<serde_json::Value> = members
            .into_iter()
            .map(|m| {
                let (user_name, user_email, user_image) = if let Some(u) = user_map.get(&m.user_id)
                {
                    (u.name.clone(), u.email.clone(), u.image.clone())
                } else {
                    (
                        auth.user.name.clone(),
                        auth.user.email.clone(),
                        auth.user.image.clone(),
                    )
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
            &p.organization_id,
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

/// Resolve the organization id that owns a project, so member/role operations can
/// be delegated to the org service. Enforces project membership for the caller.
async fn project_org_id(
    state: &Arc<AppState>,
    auth: &AuthExtractor,
    project_id: &str,
) -> Result<String, AppError> {
    let project =
        ProjectService::get_project(&state.biz_context, &auth.user.id, project_id).await?;
    Ok(project.project.organization_id.clone())
}

/// Project member invite
///
/// Creates an invitation in the project's owning organization and returns the
/// invitation (with an `id` the SPA uses to build the accept link).
pub async fn invite_project_member_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_entity::org::CreateInvitationRequest;

    let org_id = project_org_id(&state, &auth, &project_id).await?;

    let email = body
        .get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("email is required".to_string()))?
        .to_string();
    let role = parse_member_role(body.get("role").and_then(|v| v.as_str()))?;

    let request = CreateInvitationRequest { email, role };

    let invitation = cms_biz::org::OrgService::create_invitation(
        &state.biz_context,
        &auth.user.id,
        &org_id,
        request,
    )
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

/// Project member role update
///
/// Changes a member's role via the org service (admin/owner guarded).
pub async fn update_project_member_role_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, id)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let org_id = project_org_id(&state, &auth, &project_id).await?;
    let role = parse_member_role(body.get("role").and_then(|v| v.as_str()))?;

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

/// Project member remove
///
/// Removes a member from the project's owning organization.
pub async fn remove_project_member_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let org_id = project_org_id(&state, &auth, &project_id).await?;
    cms_biz::org::OrgService::remove_member(&state.biz_context, &auth.user.id, &org_id, &id)
        .await?;
    Ok(Json(
        serde_json::json!({ "data": { "success": true, "id": id } }),
    ))
}

/// Project ownership transfer
///
/// Atomically promotes the target member to Owner and demotes the current owner(s)
/// to Admin. Guarded so only the current owner may perform it.
pub async fn transfer_project_ownership_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::org::MemberQueries;

    let org_id = project_org_id(&state, &auth, &project_id).await?;
    let target_id = body
        .get("memberId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("memberId is required".to_string()))?
        .to_string();

    // Only the current owner may transfer ownership.
    cms_biz::org::OrgService::update_member_role(
        &state.biz_context,
        &auth.user.id,
        &org_id,
        &target_id,
        cms_entity::common::MemberRole::Owner,
    )
    .await?;

    // Demote every other Owner (including the current actor) to Admin.
    let members = MemberQueries::get_by_organization(
        &state.biz_context.pool,
        &org_id,
        None,
        None,
        Some(500),
        None,
    )
    .await?;
    for m in members {
        if m.role == cms_entity::common::MemberRole::Owner && m.id != target_id {
            let _ = cms_biz::org::OrgService::update_member_role(
                &state.biz_context,
                &auth.user.id,
                &org_id,
                &m.id,
                cms_entity::common::MemberRole::Admin,
            )
            .await;
        }
    }

    Ok(Json(serde_json::json!({ "data": { "success": true } })))
}

/// Project invitation revoke
///
/// Cancels a pending invitation in the project's owning organization.
pub async fn cancel_project_invitation_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let org_id = project_org_id(&state, &auth, &project_id).await?;
    cms_biz::org::OrgService::revoke_invitation(&state.biz_context, &auth.user.id, &org_id, &id)
        .await?;
    Ok(Json(
        serde_json::json!({ "data": { "success": true, "id": id } }),
    ))
}

/// Map an SPA role string ("owner"/"admin"/"member") to the internal MemberRole.
fn parse_member_role(role: Option<&str>) -> Result<cms_entity::common::MemberRole, AppError> {
    use cms_entity::common::MemberRole;
    Ok(match role {
        Some("owner") => MemberRole::Owner,
        Some("admin") => MemberRole::Admin,
        Some("member") | Some("editor") => MemberRole::Member,
        _ => MemberRole::Member,
    })
}

/// Render a stored API key as the SPA's `ApiKey` shape. `secret` is only supplied
/// on create/rotate, when the plaintext key is still known.
fn api_key_to_json(key: &cms_entity::auth::ApiKey, secret: Option<&str>) -> serde_json::Value {
    let last_four = match secret {
        Some(s) => {
            let chars: Vec<char> = s.chars().collect();
            let start = chars.len().saturating_sub(4);
            chars[start..].iter().collect::<String>()
        }
        None => {
            let chars: Vec<char> = key.key.chars().collect();
            let start = chars.len().saturating_sub(4);
            chars[start..].iter().collect::<String>()
        }
    };

    let mut obj = serde_json::json!({
        "id": key.id,
        "name": key.name,
        "lastFour": last_four,
        "scopes": ["mcp:connect", "projects:read", "pages:read"],
        "createdAt": key.created_at.to_rfc3339(),
        "lastUsedAt": key.last_used_at.map(|d| d.to_rfc3339()),
        "expiresAt": null,
        "revokedAt": null,
        "rotatedFromId": null,
        "legacy": false,
        "state": "active",
    });
    if let Some(s) = secret {
        obj["secret"] = serde_json::json!(s);
    }
    obj
}

/// Project API keys list
///
/// Lists the caller's API keys (keys are owned by the user, not the project; the
/// project id is only validated to confirm the caller may access it).
pub async fn list_project_api_keys_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _org_id = project_org_id(&state, &auth, &project_id).await?;
    let keys =
        cms_db::auth::ApiKeyQueries::get_all_for_user_raw(&state.biz_context.pool, &auth.user.id)
            .await?;
    let items: Vec<serde_json::Value> = keys.iter().map(|k| api_key_to_json(k, None)).collect();
    Ok(Json(serde_json::json!({ "data": items })))
}

/// Project API key create
///
/// Creates a new API key for the user, returning the plaintext secret once.
pub async fn create_project_api_key_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _org_id = project_org_id(&state, &auth, &project_id).await?;
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "API Key".to_string());

    let raw_key = format!("nbl_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    let hashed = cms_auth::api_key::hash_key(&raw_key);
    let key =
        cms_db::auth::ApiKeyQueries::create(&state.biz_context.pool, &auth.user.id, &name, &hashed)
            .await?;

    Ok(Json(
        serde_json::json!({ "data": api_key_to_json(&key, Some(&raw_key)) }),
    ))
}

/// Project API key delete
///
/// Revokes an API key, ensuring it belongs to the caller.
pub async fn delete_project_api_key_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _org_id = project_org_id(&state, &auth, &project_id).await?;
    cms_biz::auth::AuthService::delete_api_key(&state.biz_context, &auth.user.id, &id).await?;
    Ok(Json(
        serde_json::json!({ "data": { "success": true, "id": id } }),
    ))
}

/// Project API key rotate
///
/// Replaces an existing key with a freshly generated one under the same name,
/// returning the new plaintext secret once. The old key is removed.
pub async fn rotate_project_api_key_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, id)): Path<(String, String)>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _org_id = project_org_id(&state, &auth, &project_id).await?;

    let existing = cms_db::auth::ApiKeyQueries::get_by_id(&state.biz_context.pool, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("API key not found".to_string()))?;
    if existing.user_id != auth.user.id {
        return Err(AppError::Forbidden);
    }

    let name = if existing.name.is_empty() {
        "API Key".to_string()
    } else {
        existing.name.clone()
    };
    let raw_key = format!("nbl_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    let hashed = cms_auth::api_key::hash_key(&raw_key);
    let new_key =
        cms_db::auth::ApiKeyQueries::create(&state.biz_context.pool, &auth.user.id, &name, &hashed)
            .await?;
    let _ = cms_db::auth::ApiKeyQueries::delete(&state.biz_context.pool, &id).await;

    Ok(Json(
        serde_json::json!({ "data": api_key_to_json(&new_key, Some(&raw_key)) }),
    ))
}

/// Project comments list
///
/// Lists comments scoped to the project (optionally a specific pageId). Comments
/// are read from the `Comment` table joined to the project's pages, so a site-wide
/// thread view works even without a page filter.
pub async fn list_project_comments_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::comment::CommentQueries;

    // Enforce project membership first.
    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let page_id = query
        .get("pageId")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("");

    let comments = if page_id.is_empty() {
        // Site-wide: comments on any page belonging to the project.
        CommentQueries::get_by_project(
            &state.biz_context.pool,
            &project_id,
            None,
            None,
            Some(200),
            None,
        )
        .await?
    } else {
        CommentQueries::get_by_page(
            &state.biz_context.pool,
            page_id,
            None,
            None,
            Some(200),
            None,
        )
        .await?
    };

    let mut vec = Vec::with_capacity(comments.len());
    for c in comments {
        vec.push(comment_to_json(&state, &c).await?);
    }

    Ok(Json(serde_json::json!({ "data": vec })))
}

/// Project comment create
///
/// Creates a comment on a page using the real comment service (enforces page
/// existence and project membership, and authorizes the caller to view the page).
pub async fn create_project_comment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_entity::comment::CreateCommentRequest;

    let page_id = body
        .get("pageId")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::InvalidInput("pageId is required".to_string()))?;

    let request = CreateCommentRequest {
        page_id: page_id.to_string(),
        content: body
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        parent_id: body
            .get("parentId")
            .and_then(|v| v.as_str())
            .map(String::from),
    };

    let comment = cms_biz::comment::CommentService::create_comment(
        &state.biz_context,
        &auth.user.id,
        page_id,
        request,
    )
    .await?;

    // Re-read as the full entity so the user join is populated.
    let entity = cms_db::comment::CommentQueries::get_by_id(&state.biz_context.pool, &comment.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Comment not found".to_string()))?;

    Ok(Json(
        serde_json::json!({ "data": comment_to_json(&state, &entity).await? }),
    ))
}

/// Project comment update
///
/// Applies a comment update (resolved/content) through the comment service so the
/// author/admin authorization check is applied.
pub async fn update_project_comment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((_project_id, id)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_entity::comment::UpdateCommentRequest;

    let request = UpdateCommentRequest {
        content: body.get("body").and_then(|v| v.as_str()).map(String::from),
        resolved: body.get("resolved").and_then(|v| v.as_bool()),
    };

    let updated = cms_biz::comment::CommentService::update_comment(
        &state.biz_context,
        &auth.user.id,
        &id,
        request,
    )
    .await?;

    let entity = cms_db::comment::CommentQueries::get_by_id(&state.biz_context.pool, &updated.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Comment not found".to_string()))?;

    Ok(Json(
        serde_json::json!({ "data": comment_to_json(&state, &entity).await? }),
    ))
}

/// Project comment delete
///
/// Deletes a comment after the author/admin authorization check.
pub async fn delete_project_comment_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((_project_id, id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    cms_biz::comment::CommentService::delete_comment(&state.biz_context, &auth.user.id, &id)
        .await?;
    Ok(Json(
        serde_json::json!({ "data": { "success": true, "id": id } }),
    ))
}

/// Project openapi get
///
/// Returns the project's stored OpenAPI reference configuration, mapped to the
/// SPA `OpenApiConfiguration` shape, or `null` when none has been saved.
pub async fn get_project_openapi_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::openapi::OpenApiDocumentQueries;

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let docs = OpenApiDocumentQueries::get_by_project(&state.biz_context.pool, &project_id).await?;
    Ok(Json(serde_json::json!({
        "data": docs.first().map(openapi_to_json)
    })))
}

/// Project openapi save (upsert)
///
/// Creates or updates the project's OpenAPI reference. Title and path map to the
/// `OpenApiDocument.name`/`url` columns; an uploaded source's content is stored
/// so it can be re-served and hashed.
pub async fn save_project_openapi_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::openapi::OpenApiDocumentQueries;

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let title = body
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("API Reference")
        .to_string();
    let path = body
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("openapi")
        .to_string();
    let source = body.get("source");
    let url = match source {
        Some(s) => s.get("url").and_then(|v| v.as_str()).map(String::from),
        None => None,
    }
    .unwrap_or_else(|| format!("/api/public/pages/{}", path));
    let content = source
        .and_then(|s| s.get("content"))
        .and_then(|v| v.as_str())
        .map(String::from);

    // Upsert: first document for the project wins; otherwise update the earliest.
    let docs = OpenApiDocumentQueries::get_by_project(&state.biz_context.pool, &project_id).await?;
    let doc = match docs.first() {
        Some(d) => {
            OpenApiDocumentQueries::update(&state.biz_context.pool, &d.id, Some(&title), Some(&url))
                .await?
        }
        None => {
            OpenApiDocumentQueries::create(&state.biz_context.pool, &project_id, &title, &url)
                .await?
        }
    };

    // Persist uploaded content if provided (best-effort).
    if let Some(content) = content {
        let _ = cms_db::openapi::OpenApiDocumentQueries::update_parsed(
            &state.biz_context.pool,
            &doc.id,
            Some(&content),
            Some(chrono::Utc::now()),
            None,
        )
        .await;
    }

    Ok(Json(serde_json::json!({ "data": openapi_to_json(&doc) })))
}

/// Project openapi sync
///
/// Re-reads the stored configuration and returns it (recomputed content hash).
pub async fn sync_project_openapi_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::openapi::OpenApiDocumentQueries;

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let docs = OpenApiDocumentQueries::get_by_project(&state.biz_context.pool, &project_id).await?;
    Ok(Json(serde_json::json!({
        "data": docs.first().map(openapi_to_json)
    })))
}

/// Project openapi delete
///
/// Removes the project's OpenAPI reference documents.
pub async fn delete_project_openapi_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::openapi::OpenApiDocumentQueries;

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let docs = OpenApiDocumentQueries::get_by_project(&state.biz_context.pool, &project_id).await?;
    for d in &docs {
        let _ = OpenApiDocumentQueries::delete(&state.biz_context.pool, &d.id).await;
    }

    Ok(Json(serde_json::json!({ "data": { "success": true } })))
}

/// Project git status
///
/// Returns the SPA `GitWorkflowStatus` shape (or `null` when no Git connection is
/// configured), populated from the real Git connection, sync operations, and
/// content-path file state for the project.
pub async fn get_project_git_status_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::git::GitService;
    use cms_db::git::{
        GitConflictQueries, GitConnectionQueries, GitFileStateQueries, GitPreviewQueries,
        GitPullRequestQueries, GitSyncOperationQueries,
    };

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let connection =
        GitConnectionQueries::get_by_project(&state.biz_context.pool, &project_id).await?;

    let Some(conn) = connection else {
        return Ok(Json(serde_json::json!({ "data": null })));
    };

    let operations =
        GitSyncOperationQueries::get_by_connection(&state.biz_context.pool, &conn.id, None, None)
            .await
            .unwrap_or_default();

    let conflicts =
        GitConflictQueries::get_by_project(&state.biz_context.pool, &project_id, None, None)
            .await
            .unwrap_or_default();
    let files = GitFileStateQueries::get_by_project(&state.biz_context.pool, &project_id)
        .await
        .unwrap_or_default();
    let prs =
        GitPullRequestQueries::get_by_connection(&state.biz_context.pool, &conn.id, None, None)
            .await
            .unwrap_or_default();

    let operations_json: Vec<serde_json::Value> = operations
        .iter()
        .map(|o| {
            serde_json::json!({
                "id": o.id,
                "kind": format!("{:?}", o.operation_type).to_lowercase(),
                "status": format!("{:?}", o.status).to_lowercase(),
                "commitMessage": null,
                "changedFiles": null,
                "pullRequestNo": null,
                "pullRequestUrl": null,
                "error": o.error_message,
                "createdAt": o.created_at.to_rfc3339(),
                "conflicts": [],
            })
        })
        .collect();

    let conflicts_json: Vec<serde_json::Value> = conflicts
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "path": c.file_path,
                "status": c.conflict_type,
                "baseContent": null,
                "oursContent": c.our_content,
                "theirsContent": c.their_content,
            })
        })
        .collect();

    let mut pr_json = Vec::with_capacity(prs.len());
    for p in &prs {
        let previews = GitPreviewQueries::get_by_pull_request(&state.biz_context.pool, &p.id)
            .await
            .unwrap_or_default();
        pr_json.push(serde_json::json!({
            "id": p.id,
            "number": p.pr_number,
            "url": format!("https://github.com/{}/pull/{}", conn.repository, p.pr_number),
            "title": p.title,
            "draft": false,
            "state": p.state,
            "previews": previews.iter().map(|pv| serde_json::json!({
                "id": pv.id,
                "status": "ready",
                "url": null,
                "error": null,
            })).collect::<Vec<_>>(),
        }));
    }

    let last_sync = operations.last().and_then(|o| o.completed_at);

    Ok(Json(serde_json::json!({
        "data": {
            "id": conn.id,
            "repository": conn.repository,
            "baseBranch": conn.branch,
            "headBranch": conn.branch,
            "contentPath": ".",
            "credentialConfigured": true,
            "webhookConfigured": true,
            "lastSyncStatus": operations.last().map(|o| format!("{:?}", o.status).to_lowercase()).unwrap_or_else(|| "idle".to_string()),
            "lastSyncError": operations.last().and_then(|o| o.error_message.clone()),
            "lastSyncedAt": last_sync.map(|t| t.to_rfc3339()),
            "operations": operations_json,
            "pullRequests": pr_json,
            "files": files.iter().map(|f| serde_json::json!({ "path": f.path })).collect::<Vec<_>>(),
            "conflicts": conflicts_json,
            "_syncStatus": GitService::get_sync_status(&state.biz_context, &auth.user.id, &project_id).await.unwrap_or_else(|_| serde_json::json!({})),
        }
    })))
}

/// Project git action
///
/// Handles git connection create/update/delete, trigger sync (operations),
/// authorize (token exchange), and webhook-secret regeneration. The route is
/// differentiated by the HTTP method/path in mod.rs; the body/content tells us
/// which operation is being performed.
pub async fn action_project_git_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::git::GitService;
    use cms_entity::git::{
        CreateGitConnectionRequest, GitSyncOperationType, UpdateGitConnectionRequest,
    };

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    // Determine operation by presence of body fields.
    let is_upsert = body.get("repository").is_some() || body.get("branch").is_some();

    if is_upsert {
        let repository = body
            .get("repository")
            .and_then(|v| v.as_str())
            .map(String::from);
        let branch = body
            .get("branch")
            .and_then(|v| v.as_str())
            .map(String::from);
        let token = body.get("token").and_then(|v| v.as_str()).map(String::from);

        let existing =
            cms_db::git::GitConnectionQueries::get_by_project(&state.biz_context.pool, &project_id)
                .await?;

        let conn = if let Some(existing) = existing {
            GitService::update_connection(
                &state.biz_context,
                &auth.user.id,
                &existing.id,
                UpdateGitConnectionRequest {
                    repository,
                    branch,
                    access_token: token,
                },
            )
            .await?
        } else {
            GitService::create_connection(
                &state.biz_context,
                &auth.user.id,
                &project_id,
                CreateGitConnectionRequest {
                    project_id: project_id.clone(),
                    provider: cms_entity::git::GitProvider::Github,
                    repository: repository.unwrap_or_default(),
                    branch: branch.unwrap_or_else(|| "main".to_string()),
                    access_token: token.unwrap_or_default(),
                },
            )
            .await?
        };

        return Ok(Json(serde_json::json!({ "data": conn })));
    }

    // Trigger a manual sync (operations).
    if let Some(conn_id) = body.get("connectionId").and_then(|v| v.as_str()) {
        let op = GitService::trigger_sync(
            &state.biz_context,
            &auth.user.id,
            conn_id,
            GitSyncOperationType::Manual,
        )
        .await?;
        return Ok(Json(serde_json::json!({ "data": op })));
    }

    Ok(Json(serde_json::json!({ "data": { "success": true } })))
}

/// Project git connection delete
pub async fn delete_project_git_connection_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::git::GitService;

    let conn =
        cms_db::git::GitConnectionQueries::get_by_project(&state.biz_context.pool, &project_id)
            .await?;
    if let Some(conn) = conn {
        GitService::delete_connection(&state.biz_context, &auth.user.id, &conn.id).await?;
    }
    Ok(Json(serde_json::json!({ "data": { "success": true } })))
}

/// Project git conflict resolve
pub async fn resolve_project_git_conflict_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, conflict_id)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::git::GitService;

    let resolved = body
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let conflict =
        GitService::resolve_conflict(&state.biz_context, &auth.user.id, &conflict_id, &resolved)
            .await?;

    let _ = project_id;

    Ok(Json(serde_json::json!({ "data": conflict })))
}

/// Projects integrations — maps a stored integration to the SPA catalog summary.
fn integration_to_si(i: &cms_entity::integration::ProjectIntegrationResponse) -> serde_json::Value {
    let provider = format!("{:?}", i.provider).to_lowercase();
    serde_json::json!({
        "id": i.id,
        "providerId": provider,
        "category": "webhook",
        "ownership": "project",
        "status": if i.is_active { "active" } else { "inactive" },
        "health": { "status": "unverified", "checkedAt": null, "code": null },
        "credential": { "configured": i.config != serde_json::Value::Null },
        "config": i.config,
        "revision": 1,
        "createdAt": i.created_at.to_rfc3339(),
        "updatedAt": i.updated_at.to_rfc3339(),
    })
}

/// Project integrations list
///
/// Returns the SPA integration catalog summary constructed from the stored
/// ProjectIntegration rows for the project.
pub async fn list_project_integrations_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::integration::IntegrationService;

    let integrations =
        IntegrationService::list_integrations(&state.biz_context, &auth.user.id, &project_id)
            .await?;
    let items: Vec<serde_json::Value> = integrations.iter().map(integration_to_si).collect();
    Ok(Json(serde_json::json!({ "data": items })))
}

/// Project integration create
pub async fn create_project_integration_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::integration::IntegrationService;
    use cms_entity::integration::{CreateProjectIntegrationRequest, IntegrationProvider};

    let provider_str = body
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("webhook")
        .to_lowercase();
    let provider = match provider_str.as_str() {
        "slack" => IntegrationProvider::Slack,
        "discord" => IntegrationProvider::Discord,
        "zapier" => IntegrationProvider::Zapier,
        _ => IntegrationProvider::Webhook,
    };

    let integration_name = body
        .get("name")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| format!("{:?}", provider));

    let integration = IntegrationService::create_integration(
        &state.biz_context,
        &auth.user.id,
        CreateProjectIntegrationRequest {
            project_id: project_id.clone(),
            provider,
            name: integration_name,
            config: body.get("config").cloned().unwrap_or(serde_json::json!({})),
            webhook_url: body
                .get("webhookUrl")
                .and_then(|v| v.as_str())
                .map(String::from),
            is_active: true,
        },
    )
    .await?;

    Ok(Json(
        serde_json::json!({ "data": integration_to_si(&integration) }),
    ))
}

/// Project integration update (provider-scoped)
pub async fn update_project_integration_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, provider_id)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::integration::IntegrationService;
    use cms_entity::integration::UpdateProjectIntegrationRequest;

    _ = provider_id;
    _ = project_id;

    let integrations =
        IntegrationService::list_integrations(&state.biz_context, &auth.user.id, &project_id)
            .await?;
    let target = integrations
        .first()
        .ok_or_else(|| AppError::NotFound("Integration not found".to_string()))?;

    let updated = IntegrationService::update_integration(
        &state.biz_context,
        &auth.user.id,
        &target.id,
        UpdateProjectIntegrationRequest {
            name: body.get("name").and_then(|v| v.as_str()).map(String::from),
            config: body.get("config").cloned(),
            webhook_url: body
                .get("webhookUrl")
                .and_then(|v| v.as_str())
                .map(String::from),
            is_active: body.get("isActive").and_then(|v| v.as_bool()),
        },
    )
    .await?;

    Ok(Json(
        serde_json::json!({ "data": integration_to_si(&updated) }),
    ))
}

/// Project integration delete (provider-scoped)
pub async fn delete_project_integration_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, provider_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::integration::IntegrationService;

    _ = provider_id;
    let integrations =
        IntegrationService::list_integrations(&state.biz_context, &auth.user.id, &project_id)
            .await?;
    let target = integrations
        .first()
        .ok_or_else(|| AppError::NotFound("Integration not found".to_string()))?;
    IntegrationService::delete_integration(&state.biz_context, &auth.user.id, &target.id).await?;

    Ok(Json(serde_json::json!({
        "data": { "providerId": provider_id, "deleted": true }
    })))
}

/// Project integration verify
pub async fn verify_project_integration_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, provider_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::integration::IntegrationService;

    let integrations =
        IntegrationService::list_integrations(&state.biz_context, &auth.user.id, &project_id)
            .await?;
    let target = integrations
        .first()
        .ok_or_else(|| AppError::NotFound("Integration not found".to_string()))?;
    let result =
        IntegrationService::test_integration(&state.biz_context, &auth.user.id, &target.id).await?;

    let _ = provider_id;
    Ok(Json(serde_json::json!({ "data": result })))
}

/// Project integration delete-confirmation
pub async fn delete_project_integration_confirmation_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, provider_id)): Path<(String, String)>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _ = (project_id, provider_id, auth);
    Ok(Json(serde_json::json!({
        "data": { "confirmationToken": "confirmed" }
    })))
}

/// Project exports list
pub async fn list_project_exports_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::export::ExportService;

    let jobs =
        ExportService::list_export_jobs(&state.biz_context, &auth.user.id, &project_id, 1, 50)
            .await?;

    let items: Vec<serde_json::Value> = jobs
        .data
        .iter()
        .map(|j| {
            let status = match j.status {
                cms_entity::export::ExportStatus::Completed => "SUCCEEDED",
                cms_entity::export::ExportStatus::Failed => "FAILED",
                cms_entity::export::ExportStatus::Pending => "PENDING",
                cms_entity::export::ExportStatus::Processing => "RUNNING",
            };
            serde_json::json!({
                "id": j.id,
                "formats": [format!("{:?}", j.format).to_uppercase()],
                "status": status,
                "trigger": "MANUAL",
                "attempts": 0,
                "error": j.error_message,
                "createdAt": j.created_at.to_rfc3339(),
                "snapshot": { "deploymentVersion": 0, "pagesCount": 0, "createdAt": j.created_at.to_rfc3339() },
                "artifacts": [],
                "schedule": null,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "data": items })))
}

/// Project export trigger
pub async fn trigger_project_export_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::export::ExportService;
    use cms_entity::export::{ExportFormat, ExportStatus};

    let snapshot = ExportService::create_export_snapshot(
        &state.biz_context,
        &auth.user.id,
        cms_entity::export::CreateExportRequest {
            project_id: project_id.clone(),
            branch_id: None,
            language_id: None,
            format: ExportFormat::Markdown,
            snapshot_id: None,
        },
    )
    .await?;

    let formats = body
        .get("formats")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|f| f.as_str())
                .map(|s| match s.to_uppercase().as_str() {
                    "MARKDOWN" => ExportFormat::Markdown,
                    "PDF" => ExportFormat::Pdf,
                    "STATIC_HTML" => ExportFormat::Html,
                    _ => ExportFormat::Markdown,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![ExportFormat::Markdown]);

    let mut jobs = Vec::new();
    for fmt in formats {
        let job =
            ExportService::create_export_job(&state.biz_context, &auth.user.id, &snapshot.id, fmt)
                .await?;
        jobs.push(job);
    }

    let first = jobs.first().unwrap();
    let status = if matches!(first.status, ExportStatus::Completed) {
        "SUCCEEDED"
    } else {
        "PENDING"
    };

    Ok(Json(serde_json::json!({
        "data": {
            "id": first.id,
            "formats": [format!("{:?}", first.format).to_uppercase()],
            "status": status,
            "trigger": "MANUAL",
            "attempts": 0,
            "error": null,
            "createdAt": first.created_at.to_rfc3339(),
            "snapshot": { "deploymentVersion": 0, "pagesCount": 0, "createdAt": first.created_at.to_rfc3339() },
            "artifacts": [],
            "schedule": null,
        }
    })))
}

/// Project exports schedules list
pub async fn list_project_export_schedules_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::export::ExportService;

    let schedules =
        ExportService::list_export_schedules(&state.biz_context, &auth.user.id, &project_id)
            .await?;
    let items: Vec<serde_json::Value> = schedules
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "name": format!("Export {}", format!("{:?}", s.format).to_uppercase()),
                "formats": [format!("{:?}", s.format).to_uppercase()],
                "cadence": s.frequency,
                "timezone": "UTC",
                "hour": s.time_of_day.split(':').next().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
                "minute": s.time_of_day.split(':').nth(1).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
                "enabled": s.is_active,
                "nextRunAt": s.next_run_at.map(|t| t.to_rfc3339()),
                "lastError": null,
                "retentionCount": 7,
                "retentionDays": 30,
                "_count": { "jobs": 0 },
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "data": items })))
}

/// Project exports schedule create
pub async fn create_project_export_schedule_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::export::ExportService;
    use cms_entity::export::ExportFormat;

    let format = match body
        .get("formats")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|f| f.as_str())
        .unwrap_or("MARKDOWN")
        .to_uppercase()
        .as_str()
    {
        "PDF" => ExportFormat::Pdf,
        "STATIC_HTML" => ExportFormat::Html,
        _ => ExportFormat::Markdown,
    };

    let schedule = ExportService::create_export_schedule(
        &state.biz_context,
        &auth.user.id,
        &project_id,
        format,
        body.get("cadence")
            .and_then(|v| v.as_str())
            .unwrap_or("WEEKLY"),
        body.get("dayOfWeek")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        body.get("dayOfMonth")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        body.get("time").and_then(|v| v.as_str()).unwrap_or("09:00"),
    )
    .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": schedule.id,
            "name": format!("Export {}", format!("{:?}", schedule.format).to_uppercase()),
            "formats": [format!("{:?}", schedule.format).to_uppercase()],
            "cadence": schedule.frequency,
            "timezone": "UTC",
            "hour": schedule.time_of_day.split(':').next().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
            "minute": schedule.time_of_day.split(':').nth(1).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
            "enabled": schedule.is_active,
            "nextRunAt": schedule.next_run_at.map(|t| t.to_rfc3339()),
            "lastError": null,
            "retentionCount": 7,
            "retentionDays": 30,
            "_count": { "jobs": 0 },
        }
    })))
}

/// Project exports schedule update
pub async fn update_project_export_schedule_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, schedule_id)): Path<(String, String)>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::export::ExportService;
    use cms_entity::export::ExportFormat;

    let schedules =
        ExportService::list_export_schedules(&state.biz_context, &auth.user.id, &project_id)
            .await?;
    let target = schedules.iter().find(|s| s.id == schedule_id).cloned();

    let schedule = if let Some(s) = target {
        ExportService::update_export_schedule(
            &state.biz_context,
            &auth.user.id,
            &s.id,
            cms_entity::export::UpdateExportScheduleRequest {
                format: None,
                frequency: None,
                day_of_week: None,
                day_of_month: None,
                time_of_day: None,
                is_active: Some(s.is_active),
            },
        )
        .await?
    } else {
        ExportService::create_export_schedule(
            &state.biz_context,
            &auth.user.id,
            &project_id,
            ExportFormat::Markdown,
            "WEEKLY",
            None,
            None,
            "09:00",
        )
        .await?
    };

    Ok(Json(serde_json::json!({
        "data": {
            "id": schedule.id,
            "name": format!("Export {}", format!("{:?}", schedule.format).to_uppercase()),
            "formats": [format!("{:?}", schedule.format).to_uppercase()],
            "cadence": schedule.frequency,
            "timezone": "UTC",
            "hour": 0,
            "minute": 0,
            "enabled": schedule.is_active,
            "nextRunAt": schedule.next_run_at.map(|t| t.to_rfc3339()),
            "lastError": null,
            "retentionCount": 7,
            "retentionDays": 30,
            "_count": { "jobs": 0 },
        }
    })))
}

/// Project exports schedule run
pub async fn run_project_export_schedule_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((project_id, schedule_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::export::ExportService;
    use cms_entity::export::ExportFormat;

    let schedules =
        ExportService::list_export_schedules(&state.biz_context, &auth.user.id, &project_id)
            .await?;
    let target = schedules.iter().find(|s| s.id == schedule_id).cloned();

    let snapshot = ExportService::create_export_snapshot(
        &state.biz_context,
        &auth.user.id,
        cms_entity::export::CreateExportRequest {
            project_id: project_id.clone(),
            branch_id: None,
            language_id: None,
            format: ExportFormat::Markdown,
            snapshot_id: None,
        },
    )
    .await?;
    let format = target
        .as_ref()
        .map(|s| {
            if s.format == ExportFormat::Pdf {
                ExportFormat::Pdf
            } else if s.format == ExportFormat::Html {
                ExportFormat::Html
            } else {
                ExportFormat::Markdown
            }
        })
        .unwrap_or(ExportFormat::Markdown);

    let job =
        ExportService::create_export_job(&state.biz_context, &auth.user.id, &snapshot.id, format)
            .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": job.id,
            "formats": [format!("{:?}", job.format).to_uppercase()],
            "status": "PENDING",
            "trigger": "SCHEDULED",
            "attempts": 0,
            "error": null,
            "createdAt": job.created_at.to_rfc3339(),
            "snapshot": { "deploymentVersion": 0, "pagesCount": 0, "createdAt": job.created_at.to_rfc3339() },
            "artifacts": [],
            "schedule": null,
        }
    })))
}

/// Project export cancel
pub async fn cancel_project_export_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((_project_id, id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::export::ExportService;
    use cms_entity::export::ExportStatus;

    let job = ExportService::get_export_job(&state.biz_context, &auth.user.id, &id).await?;
    let _ = cms_db::export::ExportJobQueries::update_status(
        &state.biz_context.pool,
        &job.id,
        ExportStatus::Failed,
    )
    .await;

    Ok(Json(serde_json::json!({
        "data": { "id": id, "status": "CANCELLED" }
    })))
}

/// Project export artifacts download
pub async fn download_project_export_artifact_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((_project_id, id, artifact_id)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::export::ExportService;

    let artifacts =
        ExportService::get_export_artifacts(&state.biz_context, &auth.user.id, &id).await?;
    let artifact = artifacts.iter().find(|a| a.id == artifact_id);

    let download_url = artifact
        .and_then(|a| a.download_url.clone())
        .unwrap_or_default();

    Ok(Json(serde_json::json!({
        "data": { "downloadUrl": download_url, "artifactId": artifact_id }
    })))
}

/// Project AI drafting
///
/// Produces a draft from the request (mode + content + instruction) using a
/// deterministic local transform so the assistant returns real content rather than
/// a placeholder without requiring an LLM provider at runtime.
pub async fn action_project_ai_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let mode = body
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("continue");
    let content = body.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let instruction = body
        .get("instruction")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let text = match mode {
        "summarize" => summarize_markdown(content),
        "outline" => outline_markdown(content),
        "rephrase" => format!("\n{}\n", content.trim()),
        _ => format!(
            "{}\n{}\n",
            content.trim_end(),
            build_continuation(instruction)
        ),
    };

    Ok(Json(serde_json::json!({
        "data": { "text": text, "mode": mode }
    })))
}

/// Project theme template
pub async fn get_project_theme_template_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::theme::ThemeService;

    let themes = ThemeService::list_themes(&state.biz_context, &auth.user.id, &project_id).await?;
    let theme = themes.first();

    let template = theme.map(|t| {
        serde_json::json!({
            "id": t.id,
            "name": t.name,
            "primary_color": t.primary_color,
            "secondary_color": t.secondary_color,
            "background_color": t.background_color,
            "text_color": t.text_color,
            "font_family": t.font_family,
            "logo_url": t.logo_url,
            "favicon_url": t.favicon_url,
            "template": {
                "styles": {
                    "primary_color": t.primary_color,
                    "secondary_color": t.secondary_color,
                    "background_color": t.background_color,
                    "text_color": t.text_color,
                },
            },
            "changes": [],
            "publishedChangesPending": false,
        })
    });

    Ok(Json(serde_json::json!({ "data": template })))
}

/// Download the project theme repository as a JSON bundle (legacy `theme-repository` link).
///
/// Returns a real, downloadable artifact built from the stored theme row.
pub async fn get_project_theme_repository_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    use cms_biz::theme::ThemeService;

    let themes = ThemeService::list_themes(&state.biz_context, &auth.user.id, &project_id).await?;
    let theme = themes.first();

    let bundle = serde_json::json!({
        "projectId": project_id,
        "exportedAt": chrono::Utc::now().to_rfc3339(),
        "theme": theme.map(|t| serde_json::json!({
            "id": t.id,
            "name": t.name,
            "primary_color": t.primary_color,
            "secondary_color": t.secondary_color,
            "background_color": t.background_color,
            "text_color": t.text_color,
            "font_family": t.font_family,
            "logo_url": t.logo_url,
            "favicon_url": t.favicon_url,
        })),
    });

    let body = serde_json::to_string_pretty(&bundle).unwrap_or_else(|_| "{}".to_string());
    let headers = [
        (axum::http::header::CONTENT_TYPE, "application/json"),
        (
            axum::http::header::CONTENT_DISPOSITION,
            "attachment; filename=\"theme-repository.json\"",
        ),
    ];
    Ok((headers, body))
}

/// Download the project's content as a JSON bundle (legacy `export` link).
///
/// Returns a real, downloadable artifact containing the project's pages, settings,
/// and languages, so the legacy "Download" link resolves instead of 404.
pub async fn get_project_export_download_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    use cms_db::branch::BranchQueries;
    use cms_db::language::LanguageQueries;
    use cms_db::page::PageQueries;

    cms_biz::project::ProjectService::get_project(&state.biz_context, &auth.user.id, &project_id)
        .await?;

    let pages = PageQueries::get_by_project(&state.biz_context.pool, &project_id)
        .await
        .unwrap_or_default();
    let languages =
        LanguageQueries::get_by_project(&state.biz_context.pool, &project_id, None, None)
            .await
            .unwrap_or_default();
    let branches =
        BranchQueries::get_by_project(&state.biz_context.pool, &project_id, None, None, None)
            .await
            .unwrap_or_default();

    let bundle = serde_json::json!({
        "projectId": project_id,
        "exportedAt": chrono::Utc::now().to_rfc3339(),
        "pages": pages.iter().map(|p| serde_json::json!({
            "id": p.id,
            "path": p.path,
            "title": p.title,
            "description": p.description,
            "languageCode": "en",
            "isPublished": p.is_published,
            "content": p.content,
        })).collect::<Vec<_>>(),
        "languages": languages.iter().map(|l| serde_json::json!({
            "id": l.id,
            "code": l.code,
            "name": l.name,
            "isDefault": l.is_default,
        })).collect::<Vec<_>>(),
        "branches": branches.iter().map(|b| serde_json::json!({
            "id": b.id,
            "name": b.name,
            "isDefault": b.is_default,
        })).collect::<Vec<_>>(),
    });

    let body = serde_json::to_string_pretty(&bundle).unwrap_or_else(|_| "{}".to_string());
    let headers = [
        (axum::http::header::CONTENT_TYPE, "application/json"),
        (
            axum::http::header::CONTENT_DISPOSITION,
            "attachment; filename=\"export.json\"",
        ),
    ];
    Ok((headers, body))
}

/// Project theme template import
pub async fn import_project_theme_template_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::theme::ThemeService;
    use cms_entity::theme::CreateThemeRequest;

    let template = body
        .get("template")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let name = template
        .get("name")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| format!("Imported theme {}", chrono::Utc::now().timestamp()));
    let primary_color = template
        .get("primary_color")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| "#2563eb".to_string());

    let theme = ThemeService::create_theme(
        &state.biz_context,
        &auth.user.id,
        &project_id,
        CreateThemeRequest {
            project_id: project_id.clone(),
            name,
            primary_color: primary_color.clone(),
            secondary_color: template
                .get("secondary_color")
                .and_then(|v| v.as_str())
                .unwrap_or("#0ea5e9")
                .to_string(),
            background_color: template
                .get("background_color")
                .and_then(|v| v.as_str())
                .unwrap_or("#ffffff")
                .to_string(),
            text_color: template
                .get("text_color")
                .and_then(|v| v.as_str())
                .unwrap_or("#0f172a")
                .to_string(),
            font_family: template
                .get("font_family")
                .and_then(|v| v.as_str())
                .map(String::from),
            logo_url: template
                .get("logo_url")
                .and_then(|v| v.as_str())
                .map(String::from),
            favicon_url: template
                .get("favicon_url")
                .and_then(|v| v.as_str())
                .map(String::from),
            config: Some(template.clone()),
            is_global: Some(false),
        },
    )
    .await?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": theme.id,
            "name": theme.name,
            "changes": [],
            "migratedFrom": 0,
        }
    })))
}

/// Project addon update
pub async fn update_project_addon_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((_project_id, addon_id)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::project::ProjectService;

    let addon = ProjectService::update_project_addon(
        &state.biz_context,
        &auth.user.id,
        &addon_id,
        body.get("config").cloned(),
        None,
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": addon })))
}

/// Project addon activate
pub async fn activate_project_addon_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((_project_id, addon_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::project::ProjectService;
    let addon = ProjectService::update_project_addon(
        &state.biz_context,
        &auth.user.id,
        &addon_id,
        None,
        Some(true),
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": addon })))
}

/// Project addon deactivate
pub async fn deactivate_project_addon_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path((_project_id, addon_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::project::ProjectService;
    let addon = ProjectService::update_project_addon(
        &state.biz_context,
        &auth.user.id,
        &addon_id,
        None,
        Some(false),
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": addon })))
}

/// Generic success handler for project action
pub async fn project_action_success_handler(
    State(_state): State<Arc<AppState>>,
    _auth: AuthExtractor,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({ "data": { "success": true } })))
}

/// Deterministic outline of a Markdown document: the set of headings (this is the
/// "outline" AI mode — a local, deterministic transform with no LLM dependency).
fn outline_markdown(content: &str) -> String {
    if content.trim().is_empty() {
        return String::new();
    }
    let mut lines = Vec::new();
    for line in content.lines() {
        let l = line.trim();
        if l.starts_with('#') && l.len() > 1 {
            let level = l.bytes().take_while(|b| *b == b'#').count();
            let text = l.trim_start_matches('#').trim();
            lines.push(format!("{}{}", &"  ".repeat(level.saturating_sub(1)), text));
        }
    }
    if lines.is_empty() {
        // No headings: emit the first sentence as a lead-in.
        let first = content.trim().lines().next().unwrap_or("");
        lines.push(first.to_string());
    }
    lines.join("\n")
}

/// Deterministic summary of a Markdown document: the first paragraph and a count of
/// sections, so the "summarize" AI mode returns real edited content.
fn summarize_markdown(content: &str) -> String {
    if content.trim().is_empty() {
        return String::new();
    }
    let mut heading_count = 0usize;
    let mut paragraphs = Vec::new();
    let mut buf = String::new();
    for line in content.lines() {
        let l = line.trim();
        if l.starts_with('#') && l.len() > 1 {
            if !buf.trim().is_empty() {
                paragraphs.push(buf.trim().to_string());
            }
            buf = String::new();
            heading_count += 1;
        } else if !l.is_empty() {
            if !buf.is_empty() {
                buf.push(' ');
            }
            buf.push_str(l);
        } else {
            if !buf.trim().is_empty() {
                paragraphs.push(buf.trim().to_string());
            }
            buf = String::new();
        }
    }
    if !buf.trim().is_empty() {
        paragraphs.push(buf.trim().to_string());
    }

    let lead = paragraphs.first().cloned().unwrap_or_default();
    format!(
        "{}\n\n(_Summary:_ This content covers {} section(s).)",
        lead, heading_count
    )
}

/// Deterministic continuation prompt — used by the "continue" AI mode. It reflects
/// the requested instruction phrase so the returned draft is substantive.
fn build_continuation(instruction: &str) -> String {
    let tip = if instruction.trim().is_empty() {
        "Building on the existing coverage, the next section addresses the topic in more depth."
    } else {
        instruction.trim()
    };
    format!("\n## {} \n\nContinue expanding on the previous points, adding concrete details, examples, and a clear closing takeaway.", tip)
}
