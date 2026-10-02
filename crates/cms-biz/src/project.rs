//! Project Business Logic
//!
//! This module contains business logic for projects, including creation,
//! updates, deletion, and membership management.

use cms_db::{
    org::MemberQueries,
    project::{ProjectAddonQueries, ProjectQueries, ProjectSettingsQueries},
};
use cms_entity::{
    common::{Id, MemberRole, PaginatedResponse},
    org::OrganizationResponse,
    project::{
        CreateProjectRequest, ListProjectsQuery, ListProjectsResponse, Project, ProjectAddon,
        ProjectAddonResponse, ProjectResponse, ProjectSettings, ProjectWithOrgResponse,
        UpdateProjectRequest, UpdateProjectSettingsRequest,
    },
};
use uuid::Uuid;

use crate::{AppError, BizContext};

/// Project service
pub struct ProjectService;

impl ProjectService {
    /// Create a new project
    pub async fn create_project(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        request: CreateProjectRequest,
    ) -> Result<ProjectWithOrgResponse, AppError> {
        let effective_org_id = if org_id.trim().is_empty() {
            // Check if user already belongs to an organization
            let user_memberships =
                cms_db::org::MemberQueries::get_by_user(&ctx.pool, user_id).await?;
            if let Some(first_membership) = user_memberships.first() {
                first_membership.organization_id.clone()
            } else {
                // Mint dedicated organization for user (site is its own workspace)
                let org_slug = uuid::Uuid::new_v4().to_string();
                let org = cms_db::org::OrganizationQueries::create(
                    &ctx.pool,
                    &request.name,
                    &org_slug,
                    None,
                )
                .await?;
                let _ = cms_db::org::MemberQueries::create(
                    &ctx.pool,
                    user_id,
                    &org.id,
                    cms_entity::common::MemberRole::Owner,
                )
                .await?;
                org.id
            }
        } else {
            ctx.authz.require_org_member(user_id, org_id).await?;
            org_id.to_string()
        };

        // Generate a unique slug
        let mut slug = request.name.to_lowercase().replace(' ', "-");
        let original_slug = slug.clone();
        let mut counter = 1;

        loop {
            let is_available =
                ProjectQueries::is_slug_available(&ctx.pool, &effective_org_id, &slug, None)
                    .await?;

            if is_available {
                break;
            }

            slug = format!("{}-{}", original_slug, counter);
            counter += 1;
        }

        // Create the project
        let project = ProjectQueries::create(
            &ctx.pool,
            &effective_org_id,
            &request.name,
            &slug,
            request.description.as_deref(),
            request.icon.as_deref(),
            request.is_public,
        )
        .await?;

        // Get the organization for the response
        let org = cms_db::org::OrganizationQueries::get_by_id(&ctx.pool, &effective_org_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

        // Create default settings
        ProjectSettingsQueries::upsert(
            &ctx.pool,
            &project.id,
            None,
            None,
            None,
            Some(true),
            Some(true),
        )
        .await?;

        Ok(ProjectWithOrgResponse {
            project: project.into(),
            organization: org.into(),
        })
    }

    /// Get a project by ID
    pub async fn get_project(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<ProjectWithOrgResponse, AppError> {
        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has access to the project
        // Either through organization membership or reader access
        ctx.authz
            .require_org_member(user_id, &project.organization_id)
            .await?;

        let org = cms_db::org::OrganizationQueries::get_by_id(&ctx.pool, &project.organization_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

        Ok(ProjectWithOrgResponse {
            project: project.into(),
            organization: org.into(),
        })
    }

    /// Get a public project by org and project slug
    pub async fn get_public_project(
        ctx: &BizContext,
        org_slug: &str,
        project_slug: &str,
    ) -> Result<ProjectWithOrgResponse, AppError> {
        let org = cms_db::org::OrganizationQueries::get_by_slug(&ctx.pool, org_slug)
            .await?
            .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;
        let project = ProjectQueries::get_by_slug(&ctx.pool, &org.id, project_slug)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;
        if !project.is_public {
            return Err(AppError::NotFound("Project not found".to_string()));
        }
        Ok(ProjectWithOrgResponse {
            project: project.into(),
            organization: org.into(),
        })
    }

    /// Get a project by slug
    pub async fn get_project_by_slug(
        ctx: &BizContext,
        user_id: &str,
        org_slug: &str,
        project_slug: &str,
    ) -> Result<ProjectWithOrgResponse, AppError> {
        let org = cms_db::org::OrganizationQueries::get_by_slug(&ctx.pool, org_slug)
            .await?
            .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

        // Check if user has access to the organization
        ctx.authz.require_org_member(user_id, &org.id).await?;

        let project = ProjectQueries::get_by_slug(&ctx.pool, &org.id, project_slug)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        Ok(ProjectWithOrgResponse {
            project: project.into(),
            organization: org.into(),
        })
    }

    /// Update a project
    pub async fn update_project(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        request: UpdateProjectRequest,
    ) -> Result<ProjectResponse, AppError> {
        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has admin role in the organization
        ctx.authz
            .require_org_admin(user_id, &project.organization_id)
            .await?;

        let updated = ProjectQueries::update(
            &ctx.pool,
            project_id,
            request.name.as_deref(),
            request.slug.as_deref(),
            request.description.as_deref(),
            request.icon.as_deref(),
            request.is_public,
            request.config.as_ref(),
        )
        .await?;

        Ok(updated.into())
    }

    /// Delete a project
    pub async fn delete_project(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<bool, AppError> {
        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Only organization owner can delete a project
        ctx.authz
            .require_org_owner(user_id, &project.organization_id)
            .await?;

        ProjectQueries::delete(&ctx.pool, project_id).await
    }

    /// List projects for an organization
    pub async fn list_projects(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        query: ListProjectsQuery,
        page: u64,
        page_size: u64,
    ) -> Result<ListProjectsResponse, AppError> {
        // Check if user is a member of the organization
        ctx.authz.require_org_member(user_id, org_id).await?;

        let offset = page.saturating_sub(1) * page_size;
        let projects = ProjectQueries::get_by_organization(
            &ctx.pool,
            org_id,
            query.is_public,
            query.search.as_deref(),
            Some(page_size as i64),
            Some(offset as i64),
        )
        .await?;

        let total = ProjectQueries::count_by_organization(
            &ctx.pool,
            org_id,
            query.is_public,
            query.search.as_deref(),
        )
        .await?;

        Ok(PaginatedResponse::new(
            projects.into_iter().map(|p| p.into()).collect(),
            total as u64,
            page,
            page_size,
        ))
    }

    /// List all projects accessible to a user
    pub async fn list_all_projects_for_user(
        ctx: &BizContext,
        user_id: &str,
        page: u64,
        page_size: u64,
    ) -> Result<ListProjectsResponse, AppError> {
        // Get all organizations the user is a member of
        let members = MemberQueries::get_by_user(&ctx.pool, user_id).await?;
        let org_ids: Vec<&str> = members.iter().map(|m| m.organization_id.as_str()).collect();

        if org_ids.is_empty() {
            return Ok(PaginatedResponse::new(vec![], 0, page, page_size));
        }

        let offset = page.saturating_sub(1) * page_size;
        let projects = ProjectQueries::get_by_organizations(
            &ctx.pool,
            &org_ids,
            Some(page_size as i64),
            Some(offset as i64),
        )
        .await?;

        let total = ProjectQueries::count_by_organizations(&ctx.pool, &org_ids).await?;

        Ok(PaginatedResponse::new(
            projects.into_iter().map(|p| p.into()).collect(),
            total as u64,
            page,
            page_size,
        ))
    }

    /// Get project settings
    pub async fn get_project_settings(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<ProjectSettings, AppError> {
        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Admin)
            .await?;

        let settings = ProjectSettingsQueries::get(&ctx.pool, project_id)
            .await?
            .unwrap_or_else(|| ProjectSettings {
                project_id: project_id.to_string(),
                theme: None,
                default_language: None,
                custom_domain: None,
                search_enabled: true,
                comments_enabled: true,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            });

        Ok(settings)
    }

    /// Update project settings
    pub async fn update_project_settings(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        request: UpdateProjectSettingsRequest,
    ) -> Result<ProjectSettings, AppError> {
        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Admin)
            .await?;

        let settings = ProjectSettingsQueries::upsert(
            &ctx.pool,
            project_id,
            request.theme.as_deref(),
            request.default_language.as_deref(),
            request.custom_domain.as_deref(),
            request.search_enabled,
            request.comments_enabled,
        )
        .await?;

        Ok(settings)
    }

    /// List project addons
    pub async fn list_project_addons(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<Vec<ProjectAddonResponse>, AppError> {
        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_org_member(user_id, &project.organization_id)
            .await?;

        let addons = ProjectAddonQueries::get_by_project(&ctx.pool, project_id).await?;

        Ok(addons.into_iter().map(|a| a.into()).collect())
    }

    /// Create a project addon
    pub async fn create_project_addon(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        addon_type: &str,
        config: serde_json::Value,
        is_enabled: bool,
    ) -> Result<ProjectAddonResponse, AppError> {
        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Admin)
            .await?;

        let addon =
            ProjectAddonQueries::create(&ctx.pool, project_id, addon_type, config, is_enabled)
                .await?;

        Ok(addon.into())
    }

    /// Update a project addon
    pub async fn update_project_addon(
        ctx: &BizContext,
        user_id: &str,
        addon_id: &str,
        config: Option<serde_json::Value>,
        is_enabled: Option<bool>,
    ) -> Result<ProjectAddonResponse, AppError> {
        let addon = ProjectAddonQueries::get_by_id(&ctx.pool, addon_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Addon not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &addon.project_id, MemberRole::Admin)
            .await?;

        let updated = ProjectAddonQueries::update(&ctx.pool, addon_id, config, is_enabled).await?;

        Ok(updated.into())
    }

    /// Delete a project addon
    pub async fn delete_project_addon(
        ctx: &BizContext,
        user_id: &str,
        addon_id: &str,
    ) -> Result<bool, AppError> {
        let addon = ProjectAddonQueries::get_by_id(&ctx.pool, addon_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Addon not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &addon.project_id, MemberRole::Admin)
            .await?;

        ProjectAddonQueries::delete(&ctx.pool, addon_id).await
    }
}
