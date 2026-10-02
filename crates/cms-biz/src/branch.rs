//! Branch Business Logic
//!
//! This module contains business logic for branches within projects.

use cms_db::{branch::BranchQueries, page::PageQueries, project::ProjectQueries};
use cms_entity::{
    branch::{
        Branch, BranchResponse, BranchWithProjectResponse, CreateBranchRequest, ListBranchesQuery,
        ListBranchesResponse, SetDefaultBranchRequest, UpdateBranchRequest,
    },
    common::{Id, MemberRole, PaginatedResponse},
};
use uuid::Uuid;

use crate::{AppError, BizContext};

/// Branch service
pub struct BranchService;

impl BranchService {
    /// Create a new branch
    pub async fn create_branch(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        request: CreateBranchRequest,
    ) -> Result<BranchWithProjectResponse, AppError> {
        // Verify project exists
        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has access to create branches in this project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Editor)
            .await?;

        // Generate a unique slug
        let mut slug = request.name.to_lowercase().replace(' ', "-");
        let original_slug = slug.clone();
        let mut counter = 1;

        loop {
            let is_available =
                BranchQueries::is_slug_available(&ctx.pool, project_id, &slug, None).await?;

            if is_available {
                break;
            }

            slug = format!("{}-{}", original_slug, counter);
            counter += 1;
        }

        // Create the branch
        let branch = BranchQueries::create(
            &ctx.pool,
            project_id,
            &request.name,
            request.description.as_deref(),
            false,                 // Not default (unless it's the first branch)
            !request.is_protected, // is_public = !is_protected
        )
        .await?;

        // If this is the first branch, make it the default
        let branch_count = BranchQueries::count_by_project(&ctx.pool, project_id, None).await?;
        if branch_count == 1 {
            BranchQueries::set_default(&ctx.pool, &branch.id, &branch.project_id).await?;
        }

        Ok(BranchWithProjectResponse {
            branch: branch.into(),
            project: project.into(),
        })
    }

    /// Get a branch by ID
    pub async fn get_branch(
        ctx: &BizContext,
        user_id: &str,
        branch_id: &str,
    ) -> Result<BranchWithProjectResponse, AppError> {
        let branch = BranchQueries::get_by_id(&ctx.pool, branch_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &branch.project_id, MemberRole::Viewer)
            .await?;

        let project = ProjectQueries::get_by_id(&ctx.pool, &branch.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        Ok(BranchWithProjectResponse {
            branch: branch.into(),
            project: project.into(),
        })
    }

    /// Get a branch by slug
    pub async fn get_branch_by_slug(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        slug: &str,
    ) -> Result<BranchWithProjectResponse, AppError> {
        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;

        let branch = BranchQueries::get_by_slug(&ctx.pool, project_id, slug)
            .await?
            .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;

        Ok(BranchWithProjectResponse {
            branch: branch.into(),
            project: project.into(),
        })
    }

    /// Update a branch
    pub async fn update_branch(
        ctx: &BizContext,
        user_id: &str,
        branch_id: &str,
        request: UpdateBranchRequest,
    ) -> Result<BranchResponse, AppError> {
        let branch = BranchQueries::get_by_id(&ctx.pool, branch_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &branch.project_id, MemberRole::Admin)
            .await?;

        // If name is changing, check for slug conflicts
        if let Some(ref name) = request.name {
            let new_slug = name.to_lowercase().replace(' ', "-");
            if new_slug != branch.slug
                && !BranchQueries::is_slug_available(
                    &ctx.pool,
                    &branch.project_id,
                    &new_slug,
                    Some(branch_id),
                )
                .await?
            {
                return Err(AppError::Conflict(
                    "Branch with this slug already exists".to_string(),
                ));
            }
        }

        // Cannot make a protected branch unprotected if it's the default branch
        if let Some(is_protected) = request.is_protected {
            if !is_protected && branch.is_default {
                return Err(AppError::AccessDenied(
                    "Cannot make the default branch unprotected".to_string(),
                ));
            }
        }

        let updated = BranchQueries::update(
            &ctx.pool,
            branch_id,
            request.name.as_deref(),
            request.description.as_deref(),
            None,                             // is_default unchanged
            request.is_protected.map(|p| !p), // is_public = !is_protected
        )
        .await?;

        Ok(updated.into())
    }

    /// Delete a branch
    pub async fn delete_branch(
        ctx: &BizContext,
        user_id: &str,
        branch_id: &str,
    ) -> Result<bool, AppError> {
        let branch = BranchQueries::get_by_id(&ctx.pool, branch_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &branch.project_id, MemberRole::Admin)
            .await?;

        // Cannot delete the default branch
        if branch.is_default {
            return Err(AppError::AccessDenied(
                "Cannot delete the default branch".to_string(),
            ));
        }

        // Cannot delete a protected branch
        if branch.is_protected {
            return Err(AppError::AccessDenied(
                "Cannot delete a protected branch".to_string(),
            ));
        }

        // Check if branch has pages
        let page_count = PageQueries::count_by_branch(&ctx.pool, branch_id).await?;
        if page_count > 0 {
            return Err(AppError::Conflict(
                "Cannot delete a branch with pages. Delete or move pages first.".to_string(),
            ));
        }

        BranchQueries::delete(&ctx.pool, branch_id).await
    }

    /// List branches for a project
    pub async fn list_branches(
        ctx: &BizContext,
        user_id: &str,
        query: ListBranchesQuery,
        page: u64,
        page_size: u64,
    ) -> Result<ListBranchesResponse, AppError> {
        let _project = ProjectQueries::get_by_id(&ctx.pool, &query.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &query.project_id, MemberRole::Viewer)
            .await?;

        let limit = page_size.max(1);
        let offset = page.saturating_sub(1) * limit;

        let branches = BranchQueries::get_by_project(
            &ctx.pool,
            &query.project_id,
            query.search.as_deref(),
            Some(limit as i64),
            Some(offset as i64),
        )
        .await?;

        let total =
            BranchQueries::count_by_project(&ctx.pool, &query.project_id, query.search.as_deref())
                .await?;

        Ok(PaginatedResponse::new(
            branches.into_iter().map(|b| b.into()).collect(),
            total as u64,
            page,
            page_size,
        ))
    }

    /// Set the default branch for a project
    pub async fn set_default_branch(
        ctx: &BizContext,
        user_id: &str,
        request: SetDefaultBranchRequest,
    ) -> Result<BranchResponse, AppError> {
        let branch = BranchQueries::get_by_id(&ctx.pool, &request.branch_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &branch.project_id, MemberRole::Admin)
            .await?;

        // Cannot set a protected branch as default if user doesn't have permission
        // (This check might be redundant since we already require admin)

        let updated =
            BranchQueries::set_default(&ctx.pool, &request.branch_id, &branch.project_id).await?;

        Ok(updated.into())
    }

    /// Duplicate a branch (create a copy with all pages)
    pub async fn duplicate_branch(
        ctx: &BizContext,
        user_id: &str,
        source_branch_id: &str,
        name: &str,
    ) -> Result<BranchWithProjectResponse, AppError> {
        let source_branch = BranchQueries::get_by_id(&ctx.pool, source_branch_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Source branch not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &source_branch.project_id, MemberRole::Editor)
            .await?;

        let project = ProjectQueries::get_by_id(&ctx.pool, &source_branch.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Generate a unique slug
        let mut slug = name.to_lowercase().replace(' ', "-");
        let original_slug = slug.clone();
        let mut counter = 1;

        loop {
            let is_available =
                BranchQueries::is_slug_available(&ctx.pool, &source_branch.project_id, &slug, None)
                    .await?;

            if is_available {
                break;
            }

            slug = format!("{}-{}", original_slug, counter);
            counter += 1;
        }

        // Create the new branch
        let new_branch = BranchQueries::create(
            &ctx.pool,
            &source_branch.project_id,
            name,
            source_branch.description.as_deref(),
            false,
            false,
        )
        .await?;

        // Copy all pages from the source branch preserving parent hierarchy
        let all_pages = PageQueries::get_by_project(&ctx.pool, &source_branch.project_id).await?;
        let branch_pages: Vec<cms_entity::page::Page> = all_pages
            .into_iter()
            .filter(|p| p.branch_id == source_branch_id)
            .collect();

        let mut id_mapping: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut remaining = branch_pages;

        // Multi-pass creation to ensure parents are created before children
        while !remaining.is_empty() {
            let mut made_progress = false;
            let mut next_remaining = Vec::new();

            for page in remaining {
                let can_create = match &page.parent_id {
                    None => true,
                    Some(parent_id) => id_mapping.contains_key(parent_id),
                };

                if can_create {
                    let mapped_parent = page
                        .parent_id
                        .as_ref()
                        .and_then(|pid| id_mapping.get(pid).cloned());

                    let new_page = PageQueries::create(
                        &ctx.pool,
                        &source_branch.project_id,
                        &new_branch.id,
                        page.language_id.as_deref(),
                        mapped_parent.as_deref(),
                        Some(&page.kind),
                        &page.slug,
                        &page.title,
                        page.description.as_deref(),
                        Some(&page.content),
                        page.icon.as_deref(),
                        page.config.as_ref(),
                        page.translation_key.as_deref(),
                        page.position,
                        page.is_published,
                    )
                    .await?;

                    id_mapping.insert(page.id.clone(), new_page.id);
                    made_progress = true;
                } else {
                    next_remaining.push(page);
                }
            }

            if !made_progress {
                // Fallback for orphan or circular parent references
                for page in next_remaining {
                    let new_page = PageQueries::create(
                        &ctx.pool,
                        &source_branch.project_id,
                        &new_branch.id,
                        page.language_id.as_deref(),
                        None,
                        Some(&page.kind),
                        &page.slug,
                        &page.title,
                        page.description.as_deref(),
                        Some(&page.content),
                        page.icon.as_deref(),
                        page.config.as_ref(),
                        page.translation_key.as_deref(),
                        page.position,
                        page.is_published,
                    )
                    .await?;
                    id_mapping.insert(page.id.clone(), new_page.id);
                }
                break;
            }

            remaining = next_remaining;
        }

        Ok(BranchWithProjectResponse {
            branch: new_branch.into(),
            project: project.into(),
        })
    }
}
