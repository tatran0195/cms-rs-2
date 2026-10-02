//! Deployment Business Logic
//!
//! This module contains business logic for deployments and custom domains.

use std::sync::Arc;

use chrono::Utc;
use cms_db::{
    branch::BranchQueries,
    deployment::{DeploymentQueries, DomainQueries},
    language::LanguageQueries,
    page::PageQueries,
    project::ProjectQueries,
};
use cms_entity::{
    common::{Id, MemberRole, PaginatedResponse},
    deployment::{
        CreateDeploymentRequest, Deployment, DeploymentResponse, DeploymentStatus,
        UpdateDeploymentRequest,
    },
    domain::{CreateDomainRequest, Domain, DomainResponse, UpdateDomainRequest},
};
use cms_storage::Storage;
use uuid::Uuid;

use crate::{AppError, BizContext};

/// Deployment service
pub struct DeploymentService;

impl DeploymentService {
    /// Create a new deployment
    pub async fn create_deployment(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        request: CreateDeploymentRequest,
    ) -> Result<DeploymentResponse, AppError> {
        // Verify project exists
        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Verify branch exists
        let branch_id = request.branch_id.as_deref().unwrap_or("");
        let _branch = BranchQueries::get_by_id(&ctx.pool, branch_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Admin)
            .await?;

        let deployment =
            DeploymentQueries::create(&ctx.pool, project_id, branch_id, DeploymentStatus::Pending)
                .await?;

        // Queue the deployment job for processing
        // This would enqueue a job to the worker

        Ok(deployment.into())
    }

    /// Get a deployment
    pub async fn get_deployment(
        ctx: &BizContext,
        user_id: &str,
        deployment_id: &str,
    ) -> Result<DeploymentResponse, AppError> {
        let deployment = DeploymentQueries::get_by_id(&ctx.pool, deployment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &deployment.project_id, MemberRole::Viewer)
            .await?;

        Ok(deployment.into())
    }

    /// List deployments for a project
    pub async fn list_deployments(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResponse<DeploymentResponse>, AppError> {
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;

        let limit = page_size.max(1);
        let offset = page.saturating_sub(1) * limit;

        let deployments = DeploymentQueries::get_by_project(
            &ctx.pool,
            project_id,
            Some(limit as i64),
            Some(offset as i64),
        )
        .await?;

        let total = DeploymentQueries::count_by_project(&ctx.pool, project_id).await?;

        Ok(PaginatedResponse::new(
            deployments.into_iter().map(|d| d.into()).collect(),
            total as u64,
            page,
            page_size,
        ))
    }

    /// Update a deployment
    pub async fn update_deployment(
        ctx: &BizContext,
        user_id: &str,
        deployment_id: &str,
        request: UpdateDeploymentRequest,
    ) -> Result<DeploymentResponse, AppError> {
        let deployment = DeploymentQueries::get_by_id(&ctx.pool, deployment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &deployment.project_id, MemberRole::Admin)
            .await?;

        // If branch is changing, verify it exists
        if let Some(ref branch_id) = request.branch_id {
            let _branch = BranchQueries::get_by_id(&ctx.pool, branch_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;
        }

        let updated =
            DeploymentQueries::update(&ctx.pool, deployment_id, request.branch_id.as_deref())
                .await?;

        Ok(updated.into())
    }

    /// Delete a deployment
    pub async fn delete_deployment(
        ctx: &BizContext,
        user_id: &str,
        deployment_id: &str,
    ) -> Result<bool, AppError> {
        let deployment = DeploymentQueries::get_by_id(&ctx.pool, deployment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &deployment.project_id, MemberRole::Admin)
            .await?;

        DeploymentQueries::delete(&ctx.pool, deployment_id).await
    }

    /// Get deployment logs
    pub async fn get_deployment_logs(
        ctx: &BizContext,
        user_id: &str,
        deployment_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        let _ = Self::get_deployment(ctx, user_id, deployment_id).await?;
        Ok(serde_json::json!({ "logs": "Deployment completed successfully" }))
    }

    /// Retry a deployment
    pub async fn retry_deployment(
        ctx: &BizContext,
        user_id: &str,
        deployment_id: &str,
    ) -> Result<DeploymentResponse, AppError> {
        let deployment = DeploymentQueries::get_by_id(&ctx.pool, deployment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;

        ctx.authz
            .require_project_role(user_id, &deployment.project_id, MemberRole::Admin)
            .await?;

        // Reset deployment to pending so it can be re-processed
        let updated =
            DeploymentQueries::update_status(&ctx.pool, deployment_id, DeploymentStatus::Pending)
                .await?;

        Ok(updated.into())
    }

    /// Cancel a deployment
    pub async fn cancel_deployment(
        ctx: &BizContext,
        user_id: &str,
        deployment_id: &str,
    ) -> Result<DeploymentResponse, AppError> {
        let deployment = DeploymentQueries::get_by_id(&ctx.pool, deployment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;

        ctx.authz
            .require_project_role(user_id, &deployment.project_id, MemberRole::Admin)
            .await?;

        // Only cancel if currently pending or building
        let can_cancel = matches!(
            deployment.status,
            DeploymentStatus::Pending | DeploymentStatus::Building
        );

        if !can_cancel {
            return Err(AppError::InvalidInput(format!(
                "Cannot cancel deployment in {:?} state",
                deployment.status
            )));
        }

        let updated =
            DeploymentQueries::update_status(&ctx.pool, deployment_id, DeploymentStatus::Failed)
                .await?;

        Ok(updated.into())
    }

    /// Create a custom domain
    pub async fn create_domain(
        ctx: &BizContext,
        user_id: &str,
        deployment_id: &str,
        request: CreateDomainRequest,
    ) -> Result<DomainResponse, AppError> {
        let deployment = DeploymentQueries::get_by_id(&ctx.pool, deployment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &deployment.project_id, MemberRole::Admin)
            .await?;

        // Normalize before lookup so case, IDNA and trailing-dot variants cannot
        // bypass availability checks; the unique database constraint remains the
        // final authority in case concurrent creates race.
        let hostname = cms_db::domain::normalize_hostname(&request.hostname)?;
        let existing = DomainQueries::get_by_hostname(&ctx.pool, &hostname).await?;
        if existing.is_some() {
            return Err(AppError::Conflict("Domain is already in use".to_string()));
        }

        let domain = cms_db::domain::DomainQueries::create(
            &ctx.pool,
            deployment_id,
            &hostname,
            request.is_primary,
        )
        .await?;

        Ok(domain.into())
    }

    /// Get a domain
    pub async fn get_domain(
        ctx: &BizContext,
        user_id: &str,
        domain_id: &str,
    ) -> Result<DomainResponse, AppError> {
        let domain = DomainQueries::get_by_id(&ctx.pool, domain_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Domain not found".to_string()))?;

        let deployment = DeploymentQueries::get_by_id(&ctx.pool, &domain.deployment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &deployment.project_id, MemberRole::Viewer)
            .await?;

        Ok(domain.into())
    }

    /// List domains for a deployment
    pub async fn list_domains(
        ctx: &BizContext,
        user_id: &str,
        deployment_id: &str,
    ) -> Result<Vec<DomainResponse>, AppError> {
        let deployment = DeploymentQueries::get_by_id(&ctx.pool, deployment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &deployment.project_id, MemberRole::Viewer)
            .await?;

        let domains = DomainQueries::get_by_deployment(&ctx.pool, deployment_id).await?;

        Ok(domains.into_iter().map(|d| d.into()).collect())
    }

    /// Update a domain
    pub async fn update_domain(
        ctx: &BizContext,
        user_id: &str,
        domain_id: &str,
        request: UpdateDomainRequest,
    ) -> Result<DomainResponse, AppError> {
        let domain = DomainQueries::get_by_id(&ctx.pool, domain_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Domain not found".to_string()))?;

        let deployment = DeploymentQueries::get_by_id(&ctx.pool, &domain.deployment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &deployment.project_id, MemberRole::Admin)
            .await?;

        // Cannot make a non-primary domain primary if another domain is already primary
        if request.is_primary == Some(true) && !domain.is_primary {
            let existing_primary =
                DomainQueries::get_primary_by_deployment(&ctx.pool, &domain.deployment_id).await?;

            if existing_primary.is_some() {
                return Err(AppError::Conflict(
                    "Another domain is already primary for this deployment".to_string(),
                ));
            }
        }

        // Use the hostname-aware domain query so changing the hostname rotates
        // its challenge and clears verification/TLS state in the same SQL UPDATE.
        let updated = cms_db::domain::DomainQueries::update(
            &ctx.pool,
            domain_id,
            request.hostname.as_deref(),
            request.is_primary,
            request.ssl_certificate.as_deref(),
            request.ssl_certificate_expires_at,
        )
        .await?;

        Ok(updated.into())
    }

    /// Delete a domain
    pub async fn delete_domain(
        ctx: &BizContext,
        user_id: &str,
        domain_id: &str,
    ) -> Result<bool, AppError> {
        let domain = DomainQueries::get_by_id(&ctx.pool, domain_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Domain not found".to_string()))?;

        let deployment = DeploymentQueries::get_by_id(&ctx.pool, &domain.deployment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &deployment.project_id, MemberRole::Admin)
            .await?;

        // Cannot delete the primary domain
        if domain.is_primary {
            return Err(AppError::AccessDenied(
                "Cannot delete the primary domain".to_string(),
            ));
        }

        DomainQueries::delete(&ctx.pool, domain_id).await
    }

    /// Resolve a domain to a deployment
    pub async fn resolve_domain(
        ctx: &BizContext,
        hostname: &str,
    ) -> Result<Option<DeploymentResponse>, AppError> {
        let domain = DomainQueries::get_by_hostname(&ctx.pool, hostname).await?;

        if let Some(domain) = domain {
            let deployment = DeploymentQueries::get_by_id(&ctx.pool, &domain.deployment_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;

            Ok(Some(deployment.into()))
        } else {
            Ok(None)
        }
    }
}

// ---------------------------------------------------------------------------
// Worker functions
// ---------------------------------------------------------------------------

/// Render markdown to sanitized HTML using pulldown-cmark + ammonia.
fn render_markdown_to_html(markdown: &str, title: &str, language_code: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut html_body = String::new();
    html::push_html(&mut html_body, parser);

    // Sanitize with ammonia (allowlist-based)
    let clean_body = ammonia::clean(&html_body);

    // Language codes enter an HTML attribute. Keep only the safe BCP-47
    // characters so legacy/imported values cannot break out of the attribute.
    let safe_language = language_code
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect::<String>();
    let safe_language = if safe_language.is_empty() {
        "und"
    } else {
        &safe_language
    };

    // Wrap in a minimal HTML document
    format!(
        r#"<!DOCTYPE html>
<html lang="{safe_language}">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title}</title>
  <style>
    body {{ max-width: 800px; margin: 0 auto; padding: 2rem; font-family: system-ui, sans-serif; line-height: 1.6; }}
    pre {{ background: #f4f4f4; padding: 1rem; overflow-x: auto; border-radius: 4px; }}
    code {{ font-family: monospace; }}
    img {{ max-width: 100%; height: auto; }}
  </style>
</head>
<body>
{clean_body}
</body>
</html>"#,
        title = ammonia::clean(title),
        clean_body = clean_body,
    )
}

/// Process deployment job (for worker)
///
/// This function:
/// 1. Fetches all pages in the deployment's branch
/// 2. Renders each page from Markdown to HTML
/// 3. Uploads the HTML to the storage backend under `sites/{project_id}/{deployment_id}/`
/// 4. Marks the deployment as Active on success, or Failed on error
pub async fn process_deployment_job(
    pool: &cms_db::PgPool,
    storage: Arc<dyn Storage>,
    payload: &serde_json::Value,
) -> Result<(), AppError> {
    let deployment_id = payload
        .get("deployment_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing deployment_id".to_string()))?;

    let deployment = DeploymentQueries::get_by_id(pool, deployment_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;

    if matches!(&deployment.status, DeploymentStatus::Active) {
        // Duplicate queue delivery for an already published immutable release is
        // an idempotent success; never rebuild it from newer editor content.
        if DeploymentQueries::get_snapshot_by_deployment(pool, deployment_id)
            .await?
            .is_some()
        {
            return Ok(());
        }
        return Err(AppError::Conflict(
            "Active deployment is missing its immutable snapshot".to_string(),
        ));
    }

    if cms_db::project::ProjectQueries::is_taken_down(pool, &deployment.project_id)
        .await?
        .unwrap_or(false)
    {
        DeploymentQueries::update_error(pool, deployment_id, "Project is under platform takedown")
            .await?;
        return Err(AppError::Forbidden);
    }

    if !DeploymentQueries::claim_for_build(pool, deployment_id).await? {
        let current = DeploymentQueries::get_by_id(pool, deployment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Deployment not found".to_string()))?;
        if matches!(&current.status, DeploymentStatus::Active)
            && DeploymentQueries::get_snapshot_by_deployment(pool, deployment_id)
                .await?
                .is_some()
        {
            return Ok(());
        }
        if matches!(
            &current.status,
            DeploymentStatus::Building | DeploymentStatus::Deploying
        ) {
            // Another worker already owns this idempotent job. Acknowledge the
            // duplicate rather than marking the in-flight deployment failed.
            tracing::debug!("Deployment {} is already being processed", deployment_id);
            return Ok(());
        }
        return Err(AppError::Conflict(
            "Deployment is not pending and cannot be claimed".to_string(),
        ));
    }

    let result = async {
        // One SQL statement captures a consistent release tree. Render and
        // publish this frozen value even if editors continue changing pages.
        let snapshot_value = if let Some(source_deployment_id) = payload
            .get("snapshot_from_deployment_id")
            .and_then(serde_json::Value::as_str)
        {
            let source = DeploymentQueries::get_snapshot_by_deployment(pool, source_deployment_id)
                .await?
                .ok_or_else(|| {
                    AppError::NotFound("Rollback source snapshot not found".to_string())
                })?;
            if source.project_id != deployment.project_id
                || deployment.branch_id.as_deref() != Some(source.branch_id.as_str())
            {
                return Err(AppError::Conflict(
                    "Rollback source does not belong to the deployment's project and branch"
                        .to_string(),
                ));
            }
            let pages =
                DeploymentQueries::get_snapshot_pages(pool, source_deployment_id, None).await?;
            let mut snapshot = source.snapshot;
            snapshot
                .as_object_mut()
                .ok_or_else(|| {
                    AppError::InvalidInput("Rollback snapshot must be an object".to_string())
                })?
                .insert("pages".to_string(), serde_json::to_value(pages)?);
            snapshot
        } else {
            DeploymentQueries::capture_snapshot(pool, deployment_id).await?
        };
        let snapshot: cms_entity::deployment::DeploymentSnapshotContent =
            serde_json::from_value(snapshot_value.clone()).map_err(|error| {
                AppError::Internal(anyhow::anyhow!("Invalid deployment snapshot: {error}"))
            })?;
        do_deployment(storage.as_ref(), &deployment, deployment_id, &snapshot).await?;
        let actor_user_id = payload
            .get("actor_user_id")
            .and_then(serde_json::Value::as_str);
        DeploymentQueries::publish_snapshot(pool, deployment_id, &snapshot_value, actor_user_id)
            .await?;
        Ok::<(), AppError>(())
    }
    .await;

    match result {
        Ok(()) => {
            tracing::info!("Deployment {} completed successfully", deployment_id);
        }
        Err(ref error) => {
            tracing::error!("Deployment {} failed: {}", deployment_id, error);
            let prefix = format!("sites/{}/{}/", deployment.project_id, deployment_id);
            match storage.list(&prefix).await {
                Ok(keys) => {
                    for key in keys {
                        if let Err(cleanup_error) = storage.delete(&key).await {
                            tracing::warn!(
                                "Failed to remove partial deployment artifact {}: {}",
                                key,
                                cleanup_error
                            );
                        }
                    }
                }
                Err(cleanup_error) => tracing::warn!(
                    "Failed to list partial deployment artifacts for {}: {}",
                    deployment_id,
                    cleanup_error
                ),
            }
            DeploymentQueries::update_error(pool, deployment_id, &error.to_string()).await?;
        }
    }

    result
}

async fn do_deployment(
    storage: &dyn Storage,
    deployment: &cms_entity::deployment::Deployment,
    deployment_id: &str,
    snapshot: &cms_entity::deployment::DeploymentSnapshotContent,
) -> Result<(), AppError> {
    let branch_id = deployment
        .branch_id
        .as_deref()
        .filter(|branch_id| !branch_id.is_empty())
        .ok_or_else(|| AppError::InvalidInput("Deployment has no branch".to_string()))?;
    if snapshot.project.id != deployment.project_id || snapshot.branch.id != branch_id {
        return Err(AppError::Conflict(
            "Captured snapshot does not match the deployment scope".to_string(),
        ));
    }

    let language_codes = snapshot
        .languages
        .iter()
        .filter(|language| language.enabled)
        .map(|language| (language.id.as_str(), language.code.as_str()))
        .collect::<std::collections::HashMap<_, _>>();
    let default_language = snapshot
        .languages
        .iter()
        .find(|language| language.is_default && language.enabled)
        .or_else(|| snapshot.languages.iter().find(|language| language.enabled))
        .ok_or_else(|| AppError::Conflict("Project has no enabled language".to_string()))?;

    let pages = snapshot
        .pages
        .iter()
        .filter(|page| page.is_published && page.kind.eq_ignore_ascii_case("PAGE"))
        .collect::<Vec<_>>();
    tracing::info!(
        "Deploying {} published pages for branch {} in deployment {}",
        pages.len(),
        snapshot.branch.slug,
        deployment_id
    );

    for page in pages {
        let language_code = page
            .language_id
            .as_deref()
            .and_then(|language_id| language_codes.get(language_id).copied())
            .unwrap_or(default_language.code.as_str());
        if page
            .language_id
            .as_deref()
            .is_some_and(|language_id| !language_codes.contains_key(language_id))
        {
            continue;
        }
        if language_code.is_empty()
            || !language_code
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            return Err(AppError::InvalidInput(format!(
                "Language {} has an unsafe storage code",
                language_code
            )));
        }

        let html = render_markdown_to_html(&page.content, &page.title, language_code);
        let page_path = page.path.trim_matches('/');
        if page_path.is_empty()
            || page_path
                .split('/')
                .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        {
            return Err(AppError::InvalidInput(format!(
                "Page {} has an unsafe deployment path",
                page.id
            )));
        }
        let storage_key = format!(
            "sites/{}/{}/{}/{}.html",
            deployment.project_id, deployment_id, language_code, page_path
        );
        storage
            .put(
                &storage_key,
                bytes::Bytes::from(html),
                "text/html; charset=utf-8",
            )
            .await
            .map_err(|error| {
                tracing::error!(
                    "Failed to store page {} at {}: {}",
                    page.id,
                    storage_key,
                    error
                );
                error
            })?;
        tracing::debug!("Stored page {} -> {}", page.path, storage_key);
    }

    Ok(())
}

/// Process a legacy per-page publish job.
///
/// Published deployments are immutable release artifacts. A page edit must not
/// rewrite every active deployment in place; callers should create a deployment
/// for the intended branch to publish a new release.
pub async fn process_publish_job(
    pool: &cms_db::PgPool,
    storage: Arc<dyn Storage>,
    payload: &serde_json::Value,
) -> Result<(), AppError> {
    let page_id = payload.get("page_id").and_then(|v| v.as_str());

    let project_id = payload.get("project_id").and_then(|v| v.as_str());

    // If we have a specific page_id, validate that it still belongs to the
    // supplied project but never overwrite any already-active release.
    if let (Some(page_id), Some(project_id)) = (page_id, project_id) {
        let page = PageQueries::get_by_id(pool, page_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;
        if page.project_id != project_id {
            return Err(AppError::Conflict(
                "Page does not belong to the supplied project".to_string(),
            ));
        }
        tracing::info!(
            "Ignoring legacy in-place publish for page {}; create a new deployment to publish a release",
            page_id
        );
        Ok(())
    } else {
        // Fall back to full deployment if payload doesn't have page_id
        process_deployment_job(pool, storage, payload).await
    }
}
