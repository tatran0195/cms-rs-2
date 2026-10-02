//! Git Integration Business Logic
//!
//! This module contains business logic for Git integration,
//! including repository connections, sync operations, and conflict resolution.

use chrono::Utc;
use cms_db::{
    branch::BranchQueries,
    git::{
        GitAuditEventQueries, GitConflictQueries, GitConnectionQueries, GitFileStateQueries,
        GitPreviewQueries, GitPullRequestQueries, GitSyncOperationQueries,
        GitWebhookDeliveryQueries,
    },
    page::PageQueries,
    project::ProjectQueries,
};
use cms_entity::{
    common::{Id, MemberRole, PaginatedResponse},
    git::{
        CreateGitConnectionRequest, GitAuditEvent, GitConflict, GitConnection,
        GitConnectionResponse, GitFileState, GitPreview, GitProvider, GitPullRequest,
        GitSyncOperation, GitSyncOperationResponse, GitSyncOperationStatus, GitSyncOperationType,
        GitWebhookDelivery, UpdateGitConnectionRequest,
    },
};
use uuid::Uuid;

use crate::{AppError, BizContext};

/// Git service
pub struct GitService;

impl GitService {
    /// Create a new Git connection
    pub async fn create_connection(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        request: CreateGitConnectionRequest,
    ) -> Result<GitConnectionResponse, AppError> {
        // Verify project exists
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Admin)
            .await?;

        // Check if a connection already exists for this project
        let existing = GitConnectionQueries::get_by_project(&ctx.pool, project_id).await?;
        if existing.is_some() {
            return Err(AppError::Conflict(
                "Git connection already exists for this project".to_string(),
            ));
        }

        let connection = GitConnectionQueries::create(
            &ctx.pool,
            project_id,
            request.provider,
            &request.repository,
            &request.branch,
            &request.access_token,
        )
        .await?;

        Ok(connection.into())
    }

    /// Get Git connection for a project
    pub async fn get_connection(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<Option<GitConnectionResponse>, AppError> {
        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;

        let connection = GitConnectionQueries::get_by_project(&ctx.pool, project_id).await?;

        Ok(connection.map(|c| c.into()))
    }

    /// Update Git connection
    pub async fn update_connection(
        ctx: &BizContext,
        user_id: &str,
        connection_id: &str,
        request: UpdateGitConnectionRequest,
    ) -> Result<GitConnectionResponse, AppError> {
        let connection = GitConnectionQueries::get_by_id(&ctx.pool, connection_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Git connection not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &connection.project_id, MemberRole::Admin)
            .await?;

        let updated = GitConnectionQueries::update(
            &ctx.pool,
            connection_id,
            request.repository.as_deref(),
            request.branch.as_deref(),
            request.access_token.as_deref(),
        )
        .await?;

        Ok(updated.into())
    }

    /// Delete Git connection
    pub async fn delete_connection(
        ctx: &BizContext,
        user_id: &str,
        connection_id: &str,
    ) -> Result<bool, AppError> {
        let connection = GitConnectionQueries::get_by_id(&ctx.pool, connection_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Git connection not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &connection.project_id, MemberRole::Admin)
            .await?;

        GitConnectionQueries::delete(&ctx.pool, connection_id).await
    }

    /// Trigger a sync operation
    pub async fn trigger_sync(
        ctx: &BizContext,
        user_id: &str,
        connection_id: &str,
        operation_type: GitSyncOperationType,
    ) -> Result<GitSyncOperationResponse, AppError> {
        let connection = GitConnectionQueries::get_by_id(&ctx.pool, connection_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Git connection not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &connection.project_id, MemberRole::Editor)
            .await?;

        let operation = GitSyncOperationQueries::create(
            &ctx.pool,
            connection_id,
            operation_type,
            GitSyncOperationStatus::Pending,
        )
        .await?;

        Ok(operation.into())
    }

    /// Get sync operation status
    pub async fn get_sync_operation(
        ctx: &BizContext,
        user_id: &str,
        operation_id: &str,
    ) -> Result<GitSyncOperationResponse, AppError> {
        let operation = GitSyncOperationQueries::get_by_id(&ctx.pool, operation_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Sync operation not found".to_string()))?;

        let connection = GitConnectionQueries::get_by_id(&ctx.pool, &operation.connection_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Git connection not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &connection.project_id, MemberRole::Viewer)
            .await?;

        Ok(operation.into())
    }

    /// List sync operations for a connection
    pub async fn list_sync_operations(
        ctx: &BizContext,
        user_id: &str,
        connection_id: &str,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResponse<GitSyncOperationResponse>, AppError> {
        let connection = GitConnectionQueries::get_by_id(&ctx.pool, connection_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Git connection not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &connection.project_id, MemberRole::Viewer)
            .await?;

        let operations = GitSyncOperationQueries::get_by_connection(
            &ctx.pool,
            connection_id,
            Some(page as i64),
            Some(page_size as i64),
        )
        .await?;

        let total = GitSyncOperationQueries::count_by_connection(&ctx.pool, connection_id).await?;

        Ok(PaginatedResponse::new(
            operations.into_iter().map(|o| o.into()).collect(),
            total as u64,
            page,
            page_size,
        ))
    }

    /// Get file state
    pub async fn get_file_state(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        path: &str,
    ) -> Result<Option<GitFileState>, AppError> {
        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;

        GitFileStateQueries::get_by_path(&ctx.pool, project_id, path).await
    }

    /// List conflicts
    pub async fn list_conflicts(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResponse<GitConflict>, AppError> {
        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Editor)
            .await?;

        let conflicts = GitConflictQueries::get_by_project(
            &ctx.pool,
            project_id,
            Some(page as i64),
            Some(page_size as i64),
        )
        .await?;

        let total = GitConflictQueries::count_by_project(&ctx.pool, project_id).await?;

        Ok(PaginatedResponse::new(
            conflicts,
            total as u64,
            page,
            page_size,
        ))
    }

    /// Resolve a conflict
    pub async fn resolve_conflict(
        ctx: &BizContext,
        user_id: &str,
        conflict_id: &str,
        resolved_content: &str,
    ) -> Result<GitConflict, AppError> {
        let conflict = GitConflictQueries::get_by_id(&ctx.pool, conflict_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Conflict not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &conflict.project_id, MemberRole::Editor)
            .await?;

        let updated =
            GitConflictQueries::resolve(&ctx.pool, conflict_id, user_id, resolved_content).await?;

        Ok(updated)
    }

    /// Get pull requests
    pub async fn list_pull_requests(
        ctx: &BizContext,
        user_id: &str,
        connection_id: &str,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResponse<GitPullRequest>, AppError> {
        let connection = GitConnectionQueries::get_by_id(&ctx.pool, connection_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Git connection not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &connection.project_id, MemberRole::Viewer)
            .await?;

        let prs = GitPullRequestQueries::get_by_connection(
            &ctx.pool,
            connection_id,
            Some(page as i64),
            Some(page_size as i64),
        )
        .await?;

        let total = GitPullRequestQueries::count_by_connection(&ctx.pool, connection_id).await?;

        Ok(PaginatedResponse::new(prs, total as u64, page, page_size))
    }

    /// Get previews for a pull request
    pub async fn list_previews(
        ctx: &BizContext,
        user_id: &str,
        pull_request_id: &str,
    ) -> Result<Vec<GitPreview>, AppError> {
        let pr = GitPullRequestQueries::get_by_id(&ctx.pool, pull_request_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Pull request not found".to_string()))?;

        let connection = GitConnectionQueries::get_by_id(&ctx.pool, &pr.connection_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Git connection not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &connection.project_id, MemberRole::Viewer)
            .await?;

        GitPreviewQueries::get_by_pull_request(&ctx.pool, pull_request_id).await
    }

    /// List Git connections for a project
    pub async fn list_connections(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<Vec<GitConnectionResponse>, AppError> {
        let conn = Self::get_connection(ctx, user_id, project_id).await?;
        Ok(conn.into_iter().collect())
    }

    /// Get sync status for a project
    pub async fn get_sync_status(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;

        let connection = GitConnectionQueries::get_by_project(&ctx.pool, project_id).await?;
        if let Some(conn) = connection {
            let operations =
                GitSyncOperationQueries::get_by_connection(&ctx.pool, &conn.id, Some(1), None)
                    .await?;
            if let Some(latest) = operations.into_iter().next() {
                return Ok(serde_json::json!({
                    "status": format!("{:?}", latest.status).to_lowercase(),
                    "last_sync": latest.completed_at.or(Some(latest.updated_at)),
                    "commit_hash": latest.commit_hash,
                }));
            }
        }

        Ok(serde_json::json!({
            "status": "idle",
            "last_sync": null,
            "commit_hash": null,
        }))
    }
}

// ---------------------------------------------------------------------------
// Worker: Git Sync Implementation
// ---------------------------------------------------------------------------

/// Extract a title from Markdown content or fallback to the filename
fn extract_title_from_markdown(content: &str, fallback_slug: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            let title = heading.trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }

    // Capitalize slug words as fallback
    fallback_slug
        .replace(['-', '_'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Derive slug and page path from a Git file path
fn derive_slug_and_path(git_file_path: &str) -> (String, String) {
    let clean = git_file_path
        .trim_start_matches('/')
        .trim_end_matches(".md")
        .trim_end_matches(".markdown")
        .trim_end_matches(".mdx");

    let segments: Vec<&str> = clean.split('/').filter(|s| !s.is_empty()).collect();
    let slug = segments.last().copied().unwrap_or("index").to_string();
    let path = format!("/{}", clean);

    (slug, path)
}

/// Process Git job (for worker)
pub async fn process_git_job(
    pool: &cms_db::PgPool,
    payload: &serde_json::Value,
) -> Result<(), AppError> {
    let operation_id = payload
        .get("operation_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing operation_id".to_string()))?;

    let operation = GitSyncOperationQueries::get_by_id(pool, operation_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Sync operation not found".to_string()))?;

    // Mark as started
    GitSyncOperationQueries::update_started(pool, operation_id).await?;

    let connection = GitConnectionQueries::get_by_id(pool, &operation.connection_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Git connection not found".to_string()))?;

    let sync_result = do_git_sync(pool, &connection).await;

    match sync_result {
        Ok(commit_hash) => {
            GitSyncOperationQueries::update_completed(pool, operation_id).await?;
            tracing::info!(
                "Git sync operation {} completed successfully (commit: {:?})",
                operation_id,
                commit_hash
            );
            Ok(())
        }
        Err(e) => {
            tracing::error!("Git sync operation {} failed: {}", operation_id, e);
            GitSyncOperationQueries::update_error(pool, operation_id, &e.to_string()).await?;
            Err(e)
        }
    }
}

/// Perform actual Git repository synchronization via HTTP APIs
async fn do_git_sync(
    pool: &cms_db::PgPool,
    connection: &GitConnection,
) -> Result<Option<String>, AppError> {
    // 1. Find default branch for project
    let branch = BranchQueries::get_default(pool, &connection.project_id).await?;

    let branch = match branch {
        Some(b) => b,
        None => {
            let branches =
                BranchQueries::get_by_project(pool, &connection.project_id, None, Some(1), None)
                    .await?;
            branches.into_iter().next().ok_or_else(|| {
                AppError::NotFound("No branches found in project to sync pages into".to_string())
            })?
        }
    };

    let client = reqwest::Client::builder()
        .user_agent("CMS-Worker/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::Internal(e.into()))?;

    match connection.provider {
        GitProvider::Github => sync_github_repo(pool, &client, connection, &branch.id).await,
        GitProvider::Gitlab => sync_gitlab_repo(pool, &client, connection, &branch.id).await,
        GitProvider::Bitbucket | GitProvider::AzureDevops => {
            tracing::warn!(
                "Git sync for provider {:?} is not yet configured for direct REST API",
                connection.provider
            );
            Ok(None)
        }
    }
}

/// Sync from GitHub using GitHub Git Trees API
async fn sync_github_repo(
    pool: &cms_db::PgPool,
    client: &reqwest::Client,
    connection: &GitConnection,
    branch_id: &str,
) -> Result<Option<String>, AppError> {
    let repo = connection
        .repository
        .trim_start_matches("https://github.com/")
        .trim_end_matches(".git");
    let tree_url = format!(
        "https://api.github.com/repos/{}/git/trees/{}?recursive=1",
        repo, connection.branch
    );

    let mut req = client.get(&tree_url);
    if !connection.access_token.is_empty() {
        req = req.bearer_auth(&connection.access_token);
    }

    let res = req
        .send()
        .await
        .map_err(|e| AppError::GitOperationFailed(format!("GitHub API request failed: {}", e)))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(AppError::GitOperationFailed(format!(
            "GitHub API returned status {}: {}",
            status, body
        )));
    }

    let tree_response: serde_json::Value = res.json().await.map_err(|e| {
        AppError::GitOperationFailed(format!("Failed to parse GitHub tree response: {}", e))
    })?;

    let commit_sha = tree_response
        .get("sha")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    let tree_entries = tree_response
        .get("tree")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for entry in tree_entries {
        let entry_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let file_path = entry.get("path").and_then(|v| v.as_str()).unwrap_or("");

        if entry_type != "blob" {
            continue;
        }

        let is_markdown = file_path.ends_with(".md")
            || file_path.ends_with(".markdown")
            || file_path.ends_with(".mdx");

        if !is_markdown || file_path.starts_with('.') || file_path.starts_with(".github/") {
            continue;
        }

        // Fetch raw file content
        let raw_url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}",
            repo, connection.branch, file_path
        );

        let mut raw_req = client.get(&raw_url);
        if !connection.access_token.is_empty() {
            raw_req = raw_req.bearer_auth(&connection.access_token);
        }

        if let Ok(raw_res) = raw_req.send().await {
            if raw_res.status().is_success() {
                if let Ok(content) = raw_res.text().await {
                    let (slug, page_path) = derive_slug_and_path(file_path);
                    let title = extract_title_from_markdown(&content, &slug);

                    // Check if page exists
                    let existing_page = PageQueries::get_by_path(
                        pool,
                        &connection.project_id,
                        branch_id,
                        &page_path,
                    )
                    .await?;
                    if let Some(page) = existing_page {
                        let _ = PageQueries::update(
                            pool,
                            &page.id,
                            None,
                            None,
                            None,
                            None,
                            Some(&title),
                            None,
                            Some(&content),
                            None,
                            None,
                            None,
                            None,
                            Some(true),
                        )
                        .await;
                    } else {
                        let _ = PageQueries::create(
                            pool,
                            &connection.project_id,
                            branch_id,
                            None,
                            None,
                            Some("PAGE"),
                            &slug,
                            &title,
                            None,
                            Some(&content),
                            None,
                            None,
                            None,
                            0,
                            true,
                        )
                        .await;
                    }

                    // Record or update GitFileState
                    let existing_state =
                        GitFileStateQueries::get_by_path(pool, &connection.project_id, &page_path)
                            .await?;
                    if let Some(state) = existing_state {
                        let _ = GitFileStateQueries::update(pool, &state.id, commit_sha.as_deref())
                            .await;
                    } else {
                        let _ = GitFileStateQueries::create(
                            pool,
                            &connection.project_id,
                            &page_path,
                            file_path,
                        )
                        .await;
                    }
                }
            }
        }
    }

    Ok(commit_sha)
}

/// Sync from GitLab using GitLab Repository Tree API
async fn sync_gitlab_repo(
    pool: &cms_db::PgPool,
    client: &reqwest::Client,
    connection: &GitConnection,
    branch_id: &str,
) -> Result<Option<String>, AppError> {
    let repo_encoded = urlencoding::encode(&connection.repository);
    let tree_url = format!(
        "https://gitlab.com/api/v4/projects/{}/repository/tree?ref={}&recursive=true",
        repo_encoded, connection.branch
    );

    let mut req = client.get(&tree_url);
    if !connection.access_token.is_empty() {
        req = req.header("PRIVATE-TOKEN", &connection.access_token);
    }

    let res = req
        .send()
        .await
        .map_err(|e| AppError::GitOperationFailed(format!("GitLab API request failed: {}", e)))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(AppError::GitOperationFailed(format!(
            "GitLab API returned status {}: {}",
            status, body
        )));
    }

    let tree_entries: Vec<serde_json::Value> = res.json().await.map_err(|e| {
        AppError::GitOperationFailed(format!("Failed to parse GitLab tree response: {}", e))
    })?;

    for entry in tree_entries {
        let entry_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let file_path = entry.get("path").and_then(|v| v.as_str()).unwrap_or("");

        if entry_type != "blob" {
            continue;
        }

        let is_markdown = file_path.ends_with(".md")
            || file_path.ends_with(".markdown")
            || file_path.ends_with(".mdx");

        if !is_markdown || file_path.starts_with('.') {
            continue;
        }

        let path_encoded = urlencoding::encode(file_path);
        let raw_url = format!(
            "https://gitlab.com/api/v4/projects/{}/repository/files/{}/raw?ref={}",
            repo_encoded, path_encoded, connection.branch
        );

        let mut raw_req = client.get(&raw_url);
        if !connection.access_token.is_empty() {
            raw_req = raw_req.header("PRIVATE-TOKEN", &connection.access_token);
        }

        if let Ok(raw_res) = raw_req.send().await {
            if raw_res.status().is_success() {
                if let Ok(content) = raw_res.text().await {
                    let (slug, page_path) = derive_slug_and_path(file_path);
                    let title = extract_title_from_markdown(&content, &slug);

                    let existing_page = PageQueries::get_by_path(
                        pool,
                        &connection.project_id,
                        branch_id,
                        &page_path,
                    )
                    .await?;
                    if let Some(page) = existing_page {
                        let _ = PageQueries::update(
                            pool,
                            &page.id,
                            None,
                            None,
                            None,
                            None,
                            Some(&title),
                            None,
                            Some(&content),
                            None,
                            None,
                            None,
                            None,
                            Some(true),
                        )
                        .await;
                    } else {
                        let _ = PageQueries::create(
                            pool,
                            &connection.project_id,
                            branch_id,
                            None,
                            None,
                            Some("PAGE"),
                            &slug,
                            &title,
                            None,
                            Some(&content),
                            None,
                            None,
                            None,
                            0,
                            true,
                        )
                        .await;
                    }
                }
            }
        }
    }

    Ok(None)
}
