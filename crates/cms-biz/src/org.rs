//! Organization Business Logic
//!
//! This module contains business logic for organizations, members, and invitations.

use cms_db::org::{InvitationQueries, MemberQueries, OrganizationQueries};
use cms_entity::{
    auth::UserResponse,
    common::{Id, MemberRole, PaginatedResponse},
    org::{
        CreateInvitationRequest, CreateOrganizationRequest, InvitationResponse,
        ListInvitationsResponse, ListMembersQuery, ListMembersResponse, Member, MemberResponse,
        MemberWithUserResponse, Organization, OrganizationResponse, UpdateOrganizationRequest,
    },
};
use uuid::Uuid;

use crate::{AppError, BizContext};

/// Organization service
pub struct OrgService;

impl OrgService {
    /// Create a new organization
    pub async fn create_organization(
        ctx: &BizContext,
        user_id: &str,
        request: CreateOrganizationRequest,
    ) -> Result<OrganizationResponse, AppError> {
        // Generate a unique slug
        let mut slug = request.name.to_lowercase().replace(' ', "-");
        let original_slug = slug.clone();
        let mut counter = 1;

        loop {
            let is_available =
                OrganizationQueries::is_slug_available(&ctx.pool, &slug, None).await?;

            if is_available {
                break;
            }

            slug = format!("{}-{}", original_slug, counter);
            counter += 1;
        }

        // Create the organization
        let org = OrganizationQueries::create(
            &ctx.pool,
            &request.name,
            &slug,
            request.description.as_deref(),
        )
        .await?;

        // Make the creating user the owner
        MemberQueries::create(&ctx.pool, user_id, &org.id, MemberRole::Owner).await?;

        Ok(org.into())
    }

    /// Get an organization by ID
    pub async fn get_organization(
        ctx: &BizContext,
        org_id: &str,
    ) -> Result<OrganizationResponse, AppError> {
        let org = OrganizationQueries::get_by_id(&ctx.pool, org_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

        Ok(org.into())
    }

    /// Get an organization by slug
    pub async fn get_organization_by_slug(
        ctx: &BizContext,
        slug: &str,
    ) -> Result<OrganizationResponse, AppError> {
        let org = OrganizationQueries::get_by_slug(&ctx.pool, slug)
            .await?
            .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

        Ok(org.into())
    }

    /// Update an organization
    pub async fn update_organization(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        request: UpdateOrganizationRequest,
    ) -> Result<OrganizationResponse, AppError> {
        // Check if user is an admin or owner of the organization
        ctx.authz
            .require_org_admin(user_id, org_id)
            .await?;

        let org = OrganizationQueries::update(
            &ctx.pool,
            org_id,
            request.name.as_deref(),
            request.description.as_deref(),
            request.logo.as_deref(),
        )
        .await?;

        Ok(org.into())
    }

    /// Delete an organization
    pub async fn delete_organization(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
    ) -> Result<bool, AppError> {
        // Only the owner can delete an organization
        ctx.authz
            .require_org_owner(user_id, org_id)
            .await?;

        OrganizationQueries::delete(&ctx.pool, org_id).await
    }

    /// List organizations for a user
    pub async fn list_organizations_for_user(
        ctx: &BizContext,
        user_id: &str,
    ) -> Result<Vec<OrganizationResponse>, AppError> {
        let members = MemberQueries::get_by_user(&ctx.pool, user_id).await?;

        let org_ids: Vec<&str> = members.iter().map(|m| m.organization_id.as_str()).collect();

        let orgs = OrganizationQueries::get_by_ids(&ctx.pool, &org_ids).await?;

        Ok(orgs.into_iter().map(|o| o.into()).collect())
    }

    /// List all organizations (admin)
    pub async fn list_all_organizations(
        ctx: &BizContext,
        _user_id: &str,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResponse<OrganizationResponse>, AppError> {
        let limit = page_size as i64;
        let offset = ((page.saturating_sub(1)) as i64) * limit;
        let orgs = OrganizationQueries::list_all(&ctx.pool, Some(limit), Some(offset)).await?;
        let total = OrganizationQueries::count_all(&ctx.pool).await? as u64;
        Ok(PaginatedResponse::new(
            orgs.into_iter().map(|o| o.into()).collect(),
            total,
            page,
            page_size,
        ))
    }

    /// Add a member to an organization
    pub async fn add_member(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        new_user_id: &str,
        role: MemberRole,
    ) -> Result<MemberResponse, AppError> {
        // Check if current user is an admin or owner
        ctx.authz
            .require_org_admin(user_id, org_id)
            .await?;

        // Check if the user is already a member
        let existing = MemberQueries::get_by_user_and_org(&ctx.pool, new_user_id, org_id).await?;

        if existing.is_some() {
            return Err(AppError::Conflict(
                "User is already a member of this organization".to_string(),
            ));
        }

        let member = MemberQueries::create(&ctx.pool, new_user_id, org_id, role).await?;

        Ok(member.into())
    }

    /// Update a member's role
    pub async fn update_member_role(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        member_id: &str,
        new_role: MemberRole,
    ) -> Result<MemberResponse, AppError> {
        // Check if current user is an admin or owner
        ctx.authz
            .require_org_admin(user_id, org_id)
            .await?;

        // Cannot demote yourself
        if member_id == user_id && new_role < MemberRole::Admin {
            return Err(AppError::AccessDenied("Cannot demote yourself".to_string()));
        }

        // Only owner can change another owner's role
        let current_user_role = MemberQueries::get_by_user_and_org(&ctx.pool, user_id, org_id)
            .await?
            .map(|m| m.role)
            .unwrap_or(MemberRole::Guest);

        let target_member = MemberQueries::get_by_id(&ctx.pool, member_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;

        if target_member.role == MemberRole::Owner && current_user_role != MemberRole::Owner {
            return Err(AppError::AccessDenied(
                "Only the owner can change another owner's role".to_string(),
            ));
        }

        let member = MemberQueries::update_role(&ctx.pool, member_id, new_role).await?;

        Ok(member.into())
    }

    /// Remove a member from an organization
    pub async fn remove_member(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        member_id: &str,
    ) -> Result<bool, AppError> {
        // Check if current user is an admin or owner
        ctx.authz
            .require_org_admin(user_id, org_id)
            .await?;

        // Cannot remove yourself
        if member_id == user_id {
            return Err(AppError::AccessDenied(
                "Cannot remove yourself from the organization".to_string(),
            ));
        }

        // Only owner can remove another owner
        let current_user_role = MemberQueries::get_by_user_and_org(&ctx.pool, user_id, org_id)
            .await?
            .map(|m| m.role)
            .unwrap_or(MemberRole::Guest);

        let target_member = MemberQueries::get_by_id(&ctx.pool, member_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;

        if target_member.role == MemberRole::Owner && current_user_role != MemberRole::Owner {
            return Err(AppError::AccessDenied(
                "Only the owner can remove another owner".to_string(),
            ));
        }

        MemberQueries::delete(&ctx.pool, member_id).await
    }

    /// List members of an organization
    pub async fn list_members(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        query: ListMembersQuery,
        page: u64,
        page_size: u64,
    ) -> Result<ListMembersResponse, AppError> {
        // Check if user is a member of the organization
        ctx.authz
            .require_org_member(user_id, org_id)
            .await?;

        let offset = page.saturating_sub(1) * page_size;
        let members = MemberQueries::get_by_organization(
            &ctx.pool,
            org_id,
            query.role.as_ref(),
            query.search.as_deref(),
            Some(page_size as i64),
            Some(offset as i64),
        )
        .await?;

        let total = MemberQueries::count_by_organization(
            &ctx.pool,
            org_id,
            query.role.as_ref(),
            query.search.as_deref(),
        )
        .await?;

        // Enrich with user information
        let user_ids: Vec<&str> = members.iter().map(|m| m.user_id.as_str()).collect();
        let users = cms_db::auth::UserQueries::get_by_ids(&ctx.pool, &user_ids).await?;
        let user_map: std::collections::HashMap<String, UserResponse> = users
            .into_iter()
            .map(|u| {
                let id = u.id.clone();
                (id, u.into())
            })
            .collect();

        let member_responses: Vec<MemberWithUserResponse> = members
            .into_iter()
            .filter_map(|m| {
                user_map.get(&m.user_id).map(|user| MemberWithUserResponse {
                    member: m.into(),
                    user: user.clone(),
                })
            })
            .collect();

        Ok(PaginatedResponse::new(
            member_responses,
            total as u64,
            page,
            page_size,
        ))
    }

    /// Create an invitation
    pub async fn create_invitation(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        request: CreateInvitationRequest,
    ) -> Result<InvitationResponse, AppError> {
        // Check if current user is an admin or owner
        ctx.authz
            .require_org_admin(user_id, org_id)
            .await?;

        let token = uuid::Uuid::new_v4().to_string();
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        let invitation = InvitationQueries::create(
            &ctx.pool,
            org_id,
            &request.email,
            request.role,
            &token,
            expires_at,
        )
        .await?;

        Ok(invitation.into())
    }

    /// Accept an invitation
    pub async fn accept_invitation(
        ctx: &BizContext,
        token: &str,
        user_id: &str,
    ) -> Result<MemberResponse, AppError> {
        let invitation = InvitationQueries::get_by_token(&ctx.pool, token)
            .await?
            .ok_or_else(|| AppError::InvalidToken("Invitation not found".to_string()))?;

        // Check if invitation has expired
        if invitation.expires_at < chrono::Utc::now() {
            return Err(AppError::TokenExpired);
        }

        // Check if user is already a member
        let existing =
            MemberQueries::get_by_user_and_org(&ctx.pool, user_id, &invitation.organization_id)
                .await?;

        if existing.is_some() {
            return Err(AppError::Conflict(
                "User is already a member of this organization".to_string(),
            ));
        }

        // Create the membership
        let member = MemberQueries::create(
            &ctx.pool,
            user_id,
            &invitation.organization_id,
            invitation.role,
        )
        .await?;

        // Delete the invitation
        InvitationQueries::delete(&ctx.pool, &invitation.id).await?;

        Ok(member.into())
    }

    /// List invitations for an organization
    pub async fn list_invitations(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        page: u64,
        page_size: u64,
    ) -> Result<ListInvitationsResponse, AppError> {
        // Check if user is an admin or owner
        ctx.authz
            .require_org_admin(user_id, org_id)
            .await?;

        let invitations = InvitationQueries::get_by_organization(
            &ctx.pool,
            org_id,
            Some(page as i64),
            Some(page_size as i64),
        )
        .await?;

        let total = InvitationQueries::count_by_organization(&ctx.pool, org_id).await?;

        Ok(PaginatedResponse::new(
            invitations,
            total as u64,
            page,
            page_size,
        ))
    }

    /// Revoke an invitation
    pub async fn revoke_invitation(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        invitation_id: &str,
    ) -> Result<bool, AppError> {
        // Check if user is an admin or owner
        ctx.authz
            .require_org_admin(user_id, org_id)
            .await?;

        InvitationQueries::delete(&ctx.pool, invitation_id).await
    }
}
