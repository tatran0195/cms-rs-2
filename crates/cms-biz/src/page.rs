//! Page Business Logic
//!
//! This module contains business logic for pages, including the page tree structure,
//! path materialization, cycle detection, and reordering.

use cms_db::{
    branch::BranchQueries, language::LanguageQueries, page::PageQueries, project::ProjectQueries,
    PgPool,
};
use cms_entity::{
    common::{Id, PaginatedResponse},
    page::{
        CreatePageRequest, GetPageTreeResponse, ListPagesQuery, ListPagesResponse, Page,
        PageListItem, PageResponse, PageTreeNode, ReorderPagesRequest, UpdatePageRequest,
    },
};
use uuid::Uuid;

use crate::{
    platform_event::{FunnelEventType, PlatformEventService},
    AppError, BizContext,
};

/// Page service
pub struct PageService;

fn normalize_page_kind(kind: Option<&str>) -> Result<String, AppError> {
    let kind = kind.unwrap_or("PAGE").trim().to_ascii_uppercase();
    match kind.as_str() {
        "PAGE" | "GROUP" => Ok(kind),
        _ => Err(AppError::Validation(
            "Page kind must be PAGE or GROUP".to_string(),
        )),
    }
}

/// Build a stable Unicode-aware URL slug. Keeping letters from non-Latin scripts
/// prevents Arabic/Chinese/Japanese titles from collapsing to `untitled`.
fn normalize_page_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut pending_separator = false;
    for character in value.trim().chars() {
        if character.is_alphanumeric() {
            if pending_separator && !slug.is_empty() {
                slug.push('-');
            }
            pending_separator = false;
            slug.extend(character.to_lowercase());
        } else {
            pending_separator = true;
        }
    }
    slug.trim_matches('-').to_string()
}

struct PageEditEvent<'a> {
    organization_id: &'a str,
    project_id: &'a str,
    branch_id: &'a str,
    page_id: &'a str,
    language_id: Option<&'a str>,
    operation: &'a str,
}

async fn record_page_edited(ctx: &BizContext, user_id: &str, event: PageEditEvent<'_>) {
    PlatformEventService::record_funnel_event_best_effort(
        ctx,
        user_id,
        Some(event.organization_id),
        FunnelEventType::PageEdited,
        serde_json::json!({
            "project_id": event.project_id,
            "branch_id": event.branch_id,
            "page_id": event.page_id,
            "language_id": event.language_id,
            "operation": event.operation,
        }),
    )
    .await;
}

impl PageService {
    /// Create a page in one project/branch/language tree.
    pub async fn create_page(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        branch_id: &str,
        request: CreatePageRequest,
    ) -> Result<PageResponse, AppError> {
        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;
        let branch = BranchQueries::get_by_id(&ctx.pool, branch_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;
        if branch.project_id != project_id {
            return Err(AppError::Conflict(
                "Branch does not belong to this project".to_string(),
            ));
        }
        ctx.authz
            .require_project_role(user_id, project_id, cms_entity::common::MemberRole::Member)
            .await?;

        let title = request.title.trim();
        if title.is_empty() || title.chars().count() > 200 {
            return Err(AppError::Validation(
                "Page title must contain 1 to 200 characters".to_string(),
            ));
        }
        if request
            .content
            .as_ref()
            .is_some_and(|content| content.len() > 2 * 1024 * 1024)
        {
            return Err(AppError::PayloadTooLarge);
        }
        if request
            .description
            .as_ref()
            .is_some_and(|description| description.chars().count() > 1000)
        {
            return Err(AppError::Validation(
                "Page description must be 1000 characters or fewer".to_string(),
            ));
        }

        let language = if let Some(language_id) =
            request.language_id.as_deref().filter(|id| !id.is_empty())
        {
            let language = LanguageQueries::get_by_id(&ctx.pool, language_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Language not found".to_string()))?;
            if language.project_id != project_id {
                return Err(AppError::Conflict(
                    "Language does not belong to this project".to_string(),
                ));
            }
            language
        } else {
            LanguageQueries::get_default(&ctx.pool, project_id)
                .await?
                .ok_or_else(|| AppError::Conflict("Project has no default language".to_string()))?
        };
        let language_id = language.id;

        let parent_id = request.parent_id.as_deref().filter(|id| !id.is_empty());
        let parent = if let Some(parent_id) = parent_id {
            let parent = PageQueries::get_by_id(&ctx.pool, parent_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Parent page not found".to_string()))?;
            if parent.project_id != project_id
                || parent.branch_id != branch_id
                || parent.language_id.as_deref() != Some(language_id.as_str())
            {
                return Err(AppError::Conflict(
                    "Parent page must belong to the same project, branch, and language".to_string(),
                ));
            }
            Some(parent)
        } else {
            None
        };
        let normalized_parent_id = parent.as_ref().map(|parent| parent.id.as_str());
        let parent_path = parent.as_ref().map(|parent| parent.path.as_str());

        let kind = normalize_page_kind(request.kind.as_deref())?;
        let mut slug = normalize_page_slug(&request.slug);
        if slug.is_empty() {
            slug = normalize_page_slug(title);
        }
        if slug.is_empty() {
            slug = "untitled".to_string();
        }
        if slug.chars().count() > 160 {
            return Err(AppError::Validation(
                "Page slug must be 160 characters or fewer".to_string(),
            ));
        }
        if request.position.is_some_and(|position| position < 0) {
            return Err(AppError::Validation(
                "Page position cannot be negative".to_string(),
            ));
        }

        let original_slug = slug.clone();
        let mut found_available_path = None;
        for suffix in 0..=1000u32 {
            slug = if suffix == 0 {
                original_slug.clone()
            } else {
                format!("{original_slug}-{suffix}")
            };
            let candidate_path = match parent_path {
                Some(path) => format!("{}/{}", path.trim_end_matches('/'), slug),
                None => format!("/{slug}"),
            };
            if candidate_path.len() > 1024 {
                return Err(AppError::Validation(
                    "Page path must be 1024 bytes or fewer".to_string(),
                ));
            }
            if PageQueries::is_path_available(
                &ctx.pool,
                project_id,
                branch_id,
                Some(&language_id),
                &candidate_path,
                None,
            )
            .await?
            {
                found_available_path = Some(candidate_path);
                break;
            }
        }
        let Some(_) = found_available_path else {
            return Err(AppError::Conflict(
                "Could not allocate a unique page path after 1000 attempts".to_string(),
            ));
        };

        let position = if let Some(position) = request.position {
            position
        } else {
            PageQueries::get_max_position(
                &ctx.pool,
                project_id,
                branch_id,
                Some(&language_id),
                normalized_parent_id,
            )
            .await?
            .checked_add(1)
            .ok_or_else(|| AppError::Validation("Page position limit reached".to_string()))?
        };

        // PageQueries rechecks parent scope while holding the row lock; this
        // closes the race where a parent is moved between the reads above and
        // this insert.
        let page = PageQueries::create(
            &ctx.pool,
            project_id,
            branch_id,
            Some(&language_id),
            normalized_parent_id,
            Some(&kind),
            &slug,
            title,
            request.description.as_deref(),
            request.content.as_deref(),
            request.icon.as_deref(),
            request.config.as_ref(),
            request.translation_key.as_deref(),
            position,
            request.is_published,
        )
        .await?;
        if kind == "PAGE" {
            record_page_edited(
                ctx,
                user_id,
                PageEditEvent {
                    organization_id: &project.organization_id,
                    project_id,
                    branch_id,
                    page_id: &page.id,
                    language_id: page.language_id.as_deref(),
                    operation: "create",
                },
            )
            .await;
        }
        Ok(page.into())
    }

    /// Get a page by ID
    pub async fn get_page(
        ctx: &BizContext,
        user_id: &str,
        page_id: &str,
    ) -> Result<PageResponse, AppError> {
        let page = PageQueries::get_by_id(&ctx.pool, page_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(
                user_id,
                &page.project_id,
                cms_entity::common::MemberRole::Viewer,
            )
            .await?;

        Ok(page.into())
    }

    /// Get a page by path
    pub async fn get_page_by_path(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        branch_id: &str,
        path: &str,
    ) -> Result<PageResponse, AppError> {
        // Verify project and branch exist
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        let _branch = BranchQueries::get_by_id(&ctx.pool, branch_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, project_id, cms_entity::common::MemberRole::Viewer)
            .await?;

        let page = PageQueries::get_by_path(&ctx.pool, project_id, branch_id, path)
            .await?
            .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

        Ok(page.into())
    }

    /// List pages in a project and branch
    pub async fn list_pages(
        ctx: &BizContext,
        user_id: &str,
        query: ListPagesQuery,
        page: u64,
        page_size: u64,
    ) -> Result<ListPagesResponse, AppError> {
        // Verify project and branch exist
        let project = ProjectQueries::get_by_id(&ctx.pool, &query.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        let _branch = BranchQueries::get_by_id(&ctx.pool, &query.branch_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(
                user_id,
                &query.project_id,
                cms_entity::common::MemberRole::Viewer,
            )
            .await?;

        let offset = page.saturating_sub(1) * page_size;
        let pages = PageQueries::get_by_project_branch_and_language(
            &ctx.pool,
            &query.project_id,
            &query.branch_id,
            query.language_id.as_deref(),
            query.parent_id.as_deref(),
            query.is_published,
            query.search.as_deref(),
            Some(page_size as i64),
            Some(offset as i64),
        )
        .await?;

        let total = PageQueries::count_by_project_branch_and_language(
            &ctx.pool,
            &query.project_id,
            &query.branch_id,
            query.language_id.as_deref(),
            query.parent_id.as_deref(),
            query.is_published,
            query.search.as_deref(),
        )
        .await?;

        Ok(PaginatedResponse::new(pages, total as u64, page, page_size))
    }

    /// Get the full page tree for a project and branch
    pub async fn get_page_tree(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        branch_id: &str,
        is_published: Option<bool>,
    ) -> Result<GetPageTreeResponse, AppError> {
        // Verify project and branch exist
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        let _branch = BranchQueries::get_by_id(&ctx.pool, branch_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, project_id, cms_entity::common::MemberRole::Viewer)
            .await?;

        let tree = PageQueries::get_tree(&ctx.pool, project_id, branch_id, is_published).await?;

        Ok(tree)
    }

    /// Update page content/metadata and keep its materialized subtree paths in
    /// sync when the page moves or its slug changes.
    pub async fn update_page(
        ctx: &BizContext,
        user_id: &str,
        page_id: &str,
        request: UpdatePageRequest,
    ) -> Result<PageResponse, AppError> {
        let page = PageQueries::get_by_id(&ctx.pool, page_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;
        ctx.authz
            .require_project_role(
                user_id,
                &page.project_id,
                cms_entity::common::MemberRole::Editor,
            )
            .await?;
        let project = ProjectQueries::get_by_id(&ctx.pool, &page.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        if request
            .title
            .as_ref()
            .is_some_and(|title| title.trim().is_empty() || title.trim().chars().count() > 200)
        {
            return Err(AppError::Validation(
                "Page title must contain 1 to 200 characters".to_string(),
            ));
        }
        if request
            .content
            .as_ref()
            .is_some_and(|content| content.len() > 2 * 1024 * 1024)
        {
            return Err(AppError::PayloadTooLarge);
        }
        if request
            .description
            .as_ref()
            .is_some_and(|description| description.chars().count() > 1000)
        {
            return Err(AppError::Validation(
                "Page description must be 1000 characters or fewer".to_string(),
            ));
        }
        if request.position.is_some_and(|position| position < 0) {
            return Err(AppError::Validation(
                "Page position cannot be negative".to_string(),
            ));
        }

        let requested_language_id = if let Some(language_id) = request.language_id.as_deref() {
            let language = LanguageQueries::get_by_id(&ctx.pool, language_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Language not found".to_string()))?;
            if language.project_id != page.project_id {
                return Err(AppError::Conflict(
                    "Language does not belong to this project".to_string(),
                ));
            }
            Some(language.id)
        } else {
            None
        };
        let effective_language_id = requested_language_id
            .as_deref()
            .or(page.language_id.as_deref())
            .ok_or_else(|| AppError::Conflict("Page has no language assignment".to_string()))?;
        let changing_language = page.language_id.as_deref() != Some(effective_language_id);

        let normalized_kind = request
            .kind
            .as_deref()
            .map(|kind| normalize_page_kind(Some(kind)))
            .transpose()?;
        let normalized_slug = if let Some(slug) = request.slug.as_deref() {
            let slug = normalize_page_slug(slug);
            if slug.is_empty() || slug.chars().count() > 160 {
                return Err(AppError::Validation(
                    "Page slug must contain 1 to 160 letters or numbers".to_string(),
                ));
            }
            Some(slug)
        } else {
            None
        };

        let effective_parent_id = match request.parent_id.as_ref() {
            Some(Some(parent_id)) if !parent_id.is_empty() => Some(parent_id.as_str()),
            Some(_) => None,
            None => page.parent_id.as_deref(),
        };
        let parent_update = match request.parent_id.as_ref() {
            None => None,
            Some(None) => Some(""),
            Some(Some(parent_id)) => Some(parent_id.as_str()),
        };
        if let Some(parent_id) = effective_parent_id {
            let parent = PageQueries::get_by_id(&ctx.pool, parent_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Parent page not found".to_string()))?;
            if parent.project_id != page.project_id
                || parent.branch_id != page.branch_id
                || parent.language_id.as_deref() != Some(effective_language_id)
            {
                return Err(AppError::Conflict(
                    "Parent page must belong to the same project, branch, and language".to_string(),
                ));
            }
            if request.parent_id.is_some()
                && parent_id != page.parent_id.as_deref().unwrap_or("")
                && PageQueries::would_create_cycle(&ctx.pool, page_id, Some(parent_id)).await?
            {
                return Err(AppError::Conflict(
                    "Cannot move page: would create a cycle in the page tree".to_string(),
                ));
            }
        }
        if changing_language {
            let children =
                PageQueries::get_by_parent(&ctx.pool, &page.project_id, &page.branch_id, page_id)
                    .await?;
            if !children.is_empty() {
                return Err(AppError::Conflict(
                    "Move or translate child pages before changing this page's language"
                        .to_string(),
                ));
            }
        }

        let effective_parent_path = if let Some(parent_id) = effective_parent_id {
            Some(
                PageQueries::get_path(&ctx.pool, parent_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound("Parent page not found".to_string()))?,
            )
        } else {
            None
        };
        let effective_slug = normalized_slug.as_deref().unwrap_or(&page.slug);
        let candidate_path = match effective_parent_path.as_deref() {
            Some(parent_path) => {
                format!("{}/{}", parent_path.trim_end_matches('/'), effective_slug)
            }
            None => format!("/{effective_slug}"),
        };
        if candidate_path.len() > 1024 {
            return Err(AppError::Validation(
                "Page path must be 1024 bytes or fewer".to_string(),
            ));
        }
        if !PageQueries::is_path_available(
            &ctx.pool,
            &page.project_id,
            &page.branch_id,
            Some(effective_language_id),
            &candidate_path,
            Some(page_id),
        )
        .await?
        {
            return Err(AppError::Conflict(
                "Page path already exists in this branch and language".to_string(),
            ));
        }

        let config_patch = request.config.as_ref().map(|config| config.as_ref());
        let is_published = request
            .hidden
            .map(|hidden| !hidden)
            .or(request.is_published);
        let updated = PageQueries::update(
            &ctx.pool,
            page_id,
            parent_update,
            requested_language_id.as_deref(),
            normalized_kind.as_deref(),
            normalized_slug.as_deref(),
            request.title.as_deref().map(str::trim),
            request.description.as_deref(),
            request.content.as_deref(),
            request.icon.as_deref(),
            config_patch,
            request.translation_key.as_deref(),
            request.position,
            is_published,
        )
        .await?;
        if updated.kind.eq_ignore_ascii_case("PAGE") {
            record_page_edited(
                ctx,
                user_id,
                PageEditEvent {
                    organization_id: &project.organization_id,
                    project_id: &updated.project_id,
                    branch_id: &updated.branch_id,
                    page_id: &updated.id,
                    language_id: updated.language_id.as_deref(),
                    operation: "update",
                },
            )
            .await;
        }
        Ok(updated.into())
    }

    /// Delete a page
    pub async fn delete_page(
        ctx: &BizContext,
        user_id: &str,
        page_id: &str,
    ) -> Result<bool, AppError> {
        let page = PageQueries::get_by_id(&ctx.pool, page_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

        // Check if user has access to delete the page
        ctx.authz
            .require_project_role(
                user_id,
                &page.project_id,
                cms_entity::common::MemberRole::Editor,
            )
            .await?;

        // Page deletion reparents direct children to the root and recomputes all
        // materialized paths transactionally. Path collisions or corrupt trees
        // fail without leaving children attached to stale paths.
        PageQueries::delete(&ctx.pool, page_id).await
    }

    /// Reorder pages
    pub async fn reorder_pages(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        branch_id: &str,
        request: ReorderPagesRequest,
    ) -> Result<Vec<PageResponse>, AppError> {
        // Verify all pages belong to the same project and branch
        let page_id_refs: Vec<&str> = request.page_ids.iter().map(|s| s.as_str()).collect();
        let pages = PageQueries::get_by_ids(&ctx.pool, &page_id_refs).await?;

        for page in &pages {
            if page.project_id != project_id || page.branch_id != branch_id {
                return Err(AppError::Conflict(
                    "All pages must belong to the same project and branch".to_string(),
                ));
            }
        }

        // Check if user has access to edit pages in this project
        ctx.authz
            .require_project_role(user_id, project_id, cms_entity::common::MemberRole::Editor)
            .await?;

        let reordered = PageQueries::reorder(&ctx.pool, &request.page_ids).await?;

        Ok(reordered.into_iter().map(|p| p.into()).collect())
    }

    /// Recompute materialized paths for a branch
    ///
    /// This is called when the page tree structure changes and we need to ensure
    /// all paths are correct. In practice, this is handled automatically during
    /// create/update operations, but this function can be called explicitly if needed.
    pub async fn recompute_paths(
        ctx: &BizContext,
        project_id: &str,
        branch_id: &str,
    ) -> Result<u64, AppError> {
        // Get all pages in this branch
        let pages = PageQueries::get_by_project_and_branch(
            &ctx.pool, project_id, branch_id, None, None, None, None, None,
        )
        .await?;

        let mut count = 0u64;

        // Process root pages first
        let root_pages: Vec<_> = pages.iter().filter(|p| p.parent_id.is_none()).collect();

        for root_page in root_pages {
            count += Self::recompute_path_for_page(&ctx.pool, root_page.id.as_str()).await?;
        }

        Ok(count)
    }

    /// Recompute path for a single page and its descendants
    async fn recompute_path_for_page(pool: &PgPool, start_page_id: &str) -> Result<u64, AppError> {
        use cms_db::page::PageQueries;

        let mut count = 0u64;
        let mut queue = vec![start_page_id.to_string()];

        while let Some(page_id) = queue.pop() {
            let page = PageQueries::get_by_id(pool, &page_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

            // If this is a root page, path is just /slug
            // Otherwise, path is parent_path/slug
            let new_path = if let Some(parent_id) = &page.parent_id {
                let parent = PageQueries::get_by_id(pool, parent_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound("Parent page not found".to_string()))?;
                format!("{}/{}", parent.path.trim_end_matches('/'), page.slug)
            } else {
                format!("/{}", page.slug)
            };

            // Update the path if it changed
            if new_path != page.path {
                PageQueries::update_path(pool, &page_id, &new_path).await?;
            }
            count += 1;

            // Process children
            let children =
                PageQueries::get_by_parent(pool, &page.project_id, &page.branch_id, &page_id)
                    .await?;
            for child in children {
                queue.push(child.id.to_string());
            }
        }

        Ok(count)
    }

    /// Helper to resolve public project by org slug and project slug
    async fn resolve_public_project(
        pool: &PgPool,
        org_slug: &str,
        project_slug: &str,
    ) -> Result<cms_entity::project::Project, AppError> {
        use cms_db::org::OrganizationQueries;

        let org = OrganizationQueries::get_by_slug(pool, org_slug)
            .await?
            .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

        let project = ProjectQueries::get_by_slug(pool, &org.id, project_slug)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        if !project.is_public {
            return Err(AppError::Forbidden);
        }

        Ok(project)
    }

    /// Get a public page by path
    pub async fn get_public_page(
        ctx: &BizContext,
        org_slug: &str,
        project_slug: &str,
        page_path: &str,
    ) -> Result<PageResponse, AppError> {
        let project = Self::resolve_public_project(&ctx.pool, org_slug, project_slug).await?;

        let default_branch = BranchQueries::get_default(&ctx.pool, &project.id).await?;
        let branch_id = default_branch
            .map(|b| b.id)
            .ok_or_else(|| AppError::NotFound("Default branch not found".to_string()))?;

        let normalized_path = if page_path.starts_with('/') {
            page_path.to_string()
        } else {
            format!("/{}", page_path)
        };

        let page =
            PageQueries::get_by_path(&ctx.pool, &project.id, &branch_id, &normalized_path).await?;

        let page = match page {
            Some(p) => p,
            None => PageQueries::get_by_id(&ctx.pool, page_path)
                .await?
                .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?,
        };

        if !page.is_published {
            return Err(AppError::NotFound("Page not found".to_string()));
        }

        Ok(page.into())
    }

    /// List public pages
    pub async fn list_public_pages(
        ctx: &BizContext,
        org_slug: &str,
        project_slug: &str,
    ) -> Result<Vec<PageResponse>, AppError> {
        let project = Self::resolve_public_project(&ctx.pool, org_slug, project_slug).await?;

        let default_branch = BranchQueries::get_default(&ctx.pool, &project.id).await?;
        let branch_id = default_branch.map(|b| b.id).unwrap_or_default();

        let all_pages = PageQueries::get_by_project(&ctx.pool, &project.id).await?;
        let published_pages = all_pages
            .into_iter()
            .filter(|p| p.is_published && (branch_id.is_empty() || p.branch_id == branch_id))
            .map(|p| p.into())
            .collect();

        Ok(published_pages)
    }

    /// Search public pages
    pub async fn search_public_pages(
        ctx: &BizContext,
        org_slug: &str,
        project_slug: &str,
        search_term: &str,
    ) -> Result<Vec<PageResponse>, AppError> {
        let project = Self::resolve_public_project(&ctx.pool, org_slug, project_slug).await?;

        let default_branch = BranchQueries::get_default(&ctx.pool, &project.id).await?;
        let branch_id = default_branch.map(|b| b.id).unwrap_or_default();

        let all_pages = PageQueries::get_by_project(&ctx.pool, &project.id).await?;
        let search_lower = search_term.to_lowercase();

        let results = all_pages
            .into_iter()
            .filter(|p| {
                p.is_published
                    && (branch_id.is_empty() || p.branch_id == branch_id)
                    && (p.title.to_lowercase().contains(&search_lower)
                        || p.content.to_lowercase().contains(&search_lower)
                        || p.slug.to_lowercase().contains(&search_lower))
            })
            .map(|p| p.into())
            .collect();

        Ok(results)
    }

    /// Get project sitemap
    pub async fn get_project_sitemap(
        ctx: &BizContext,
        org_slug: &str,
        project_slug: &str,
    ) -> Result<serde_json::Value, AppError> {
        let project = Self::resolve_public_project(&ctx.pool, org_slug, project_slug).await?;

        let default_branch = BranchQueries::get_default(&ctx.pool, &project.id).await?;
        let branch_id = default_branch.map(|b| b.id).unwrap_or_default();

        let all_pages = PageQueries::get_by_project(&ctx.pool, &project.id).await?;
        let urls: Vec<serde_json::Value> = all_pages
            .into_iter()
            .filter(|p| p.is_published && (branch_id.is_empty() || p.branch_id == branch_id))
            .map(|p| {
                serde_json::json!({
                    "loc": format!("/{}/{}{}", org_slug, project_slug, p.path),
                    "lastmod": p.updated_at.to_rfc3339(),
                    "title": p.title,
                })
            })
            .collect();

        Ok(serde_json::json!({ "urls": urls }))
    }
}
