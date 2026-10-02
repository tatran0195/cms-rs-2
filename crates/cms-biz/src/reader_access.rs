//! Reader Access Business Logic
//!
//! This module contains business logic for reader access to private documentation.
//! It includes readers, audiences, grants, invitations, and JWT-based authentication.

use std::sync::Arc;

use chrono::{Duration, Utc};
use cms_auth::{
    jwt::{JwtService, ReaderJwtClaims},
    AuthService,
};
use cms_db::{
    branch::BranchQueries,
    language::LanguageQueries,
    project::ProjectQueries,
    reader_access::{
        AudienceGrantQueries, AudienceQueries, JwtAccessProviderQueries, JwtReplayQueries,
        ReaderAudienceQueries, ReaderAuditLogQueries, ReaderInvitationQueries, ReaderQueries,
        ReaderSessionQueries,
    },
};
use cms_entity::{
    common::{Id, MemberRole, PaginatedResponse},
    reader_access::{
        AcceptInvitationRequest, Audience, AudienceGrant, AudienceGrantResponse, AudienceResponse,
        CreateAudienceRequest, CreateInvitationRequest, CreateReaderRequest, JwtAccessProvider,
        JwtReplay, Reader, ReaderAudience, ReaderAudienceResponse, ReaderAuditLog,
        ReaderInvitation, ReaderInvitationResponse, ReaderResponse, ReaderSession,
    },
};
use uuid::Uuid;

use crate::{AppError, BizContext};

/// Reader access service
pub struct ReaderAccessService;

impl ReaderAccessService {
    /// Create a new reader
    pub async fn create_reader(
        ctx: &BizContext,
        request: CreateReaderRequest,
    ) -> Result<ReaderResponse, AppError> {
        // Check if reader already exists
        let existing = ReaderQueries::get_by_email(&ctx.pool, &request.email).await?;
        if existing.is_some() {
            return Err(AppError::Conflict(
                "Reader with this email already exists".to_string(),
            ));
        }

        let reader =
            ReaderQueries::create(&ctx.pool, &request.email, request.name.as_deref()).await?;

        Ok(reader.into())
    }

    /// Get a reader by ID
    pub async fn get_reader(ctx: &BizContext, reader_id: &str) -> Result<ReaderResponse, AppError> {
        let reader = ReaderQueries::get_by_id(&ctx.pool, reader_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Reader not found".to_string()))?;

        Ok(reader.into())
    }

    /// Update a reader
    pub async fn update_reader(
        ctx: &BizContext,
        reader_id: &str,
        name: Option<&str>,
    ) -> Result<ReaderResponse, AppError> {
        let reader = ReaderQueries::update(&ctx.pool, reader_id, name).await?;
        Ok(reader.into())
    }

    /// Delete a reader
    pub async fn delete_reader(ctx: &BizContext, reader_id: &str) -> Result<bool, AppError> {
        ReaderQueries::delete(&ctx.pool, reader_id).await
    }

    /// Create an audience
    pub async fn create_audience(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        request: CreateAudienceRequest,
    ) -> Result<AudienceResponse, AppError> {
        // Verify project exists
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Admin)
            .await?;

        let audience = AudienceQueries::create(
            &ctx.pool,
            project_id,
            &request.name,
            request.description.as_deref(),
        )
        .await?;

        Ok(audience.into())
    }

    /// Get an audience by ID
    pub async fn get_audience(
        ctx: &BizContext,
        user_id: &str,
        audience_id: &str,
    ) -> Result<AudienceResponse, AppError> {
        let audience = AudienceQueries::get_by_id(&ctx.pool, audience_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Audience not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &audience.project_id, MemberRole::Viewer)
            .await?;

        Ok(audience.into())
    }

    /// List audiences for a project
    pub async fn list_audiences(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<Vec<AudienceResponse>, AppError> {
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;

        let audiences = AudienceQueries::get_by_project(&ctx.pool, project_id).await?;

        Ok(audiences.into_iter().map(|a| a.into()).collect())
    }

    /// Update an audience
    pub async fn update_audience(
        ctx: &BizContext,
        user_id: &str,
        audience_id: &str,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<AudienceResponse, AppError> {
        let audience = AudienceQueries::get_by_id(&ctx.pool, audience_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Audience not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &audience.project_id, MemberRole::Admin)
            .await?;

        let updated = AudienceQueries::update(&ctx.pool, audience_id, name, description).await?;

        Ok(updated.into())
    }

    /// Delete an audience
    pub async fn delete_audience(
        ctx: &BizContext,
        user_id: &str,
        audience_id: &str,
    ) -> Result<bool, AppError> {
        let audience = AudienceQueries::get_by_id(&ctx.pool, audience_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Audience not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &audience.project_id, MemberRole::Admin)
            .await?;

        AudienceQueries::delete(&ctx.pool, audience_id).await
    }

    /// Add a reader to an audience
    pub async fn add_reader_to_audience(
        ctx: &BizContext,
        user_id: &str,
        audience_id: &str,
        reader_id: &str,
    ) -> Result<ReaderAudienceResponse, AppError> {
        let audience = AudienceQueries::get_by_id(&ctx.pool, audience_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Audience not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &audience.project_id, MemberRole::Admin)
            .await?;

        let reader_audience =
            ReaderAudienceQueries::create(&ctx.pool, reader_id, audience_id).await?;

        Ok(reader_audience.into())
    }

    /// Remove a reader from an audience
    pub async fn remove_reader_from_audience(
        ctx: &BizContext,
        user_id: &str,
        audience_id: &str,
        reader_id: &str,
    ) -> Result<bool, AppError> {
        let audience = AudienceQueries::get_by_id(&ctx.pool, audience_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Audience not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &audience.project_id, MemberRole::Admin)
            .await?;

        ReaderAudienceQueries::delete(&ctx.pool, reader_id, audience_id).await
    }

    /// Create an audience grant
    pub async fn create_audience_grant(
        ctx: &BizContext,
        user_id: &str,
        audience_id: &str,
        project_id: &str,
        branch_id: Option<&str>,
        language_id: Option<&str>,
    ) -> Result<AudienceGrantResponse, AppError> {
        let audience = AudienceQueries::get_by_id(&ctx.pool, audience_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Audience not found".to_string()))?;

        // Check if user has admin role in the audience's source project
        ctx.authz
            .require_project_role(user_id, &audience.project_id, MemberRole::Admin)
            .await?;

        // Fetch source project
        let audience_project = ProjectQueries::get_by_id(&ctx.pool, &audience.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Audience project not found".to_string()))?;

        // Fetch target project
        let target_project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Target project not found".to_string()))?;

        // Verify target project belongs to the same organization as the audience project
        if audience_project.organization_id != target_project.organization_id {
            return Err(AppError::AccessDenied(
                "Audience and target project must belong to the same organization".to_string(),
            ));
        }

        // Check if user has admin role in the target project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Admin)
            .await?;

        // Verify branch belongs to the target project if specified
        if let Some(bid) = branch_id {
            let branch = BranchQueries::get_by_id(&ctx.pool, bid)
                .await?
                .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;
            if branch.project_id != target_project.id {
                return Err(AppError::Validation(
                    "Branch does not belong to the target project".to_string(),
                ));
            }
        }

        // Verify language belongs to the target project if specified
        if let Some(lid) = language_id {
            let language = LanguageQueries::get_by_id(&ctx.pool, lid)
                .await?
                .ok_or_else(|| AppError::NotFound("Language not found".to_string()))?;
            if language.project_id != target_project.id {
                return Err(AppError::Validation(
                    "Language does not belong to the target project".to_string(),
                ));
            }
        }

        let grant = AudienceGrantQueries::create(
            &ctx.pool,
            audience_id,
            project_id,
            branch_id,
            language_id,
        )
        .await?;

        Ok(grant.into())
    }

    /// Delete an audience grant
    pub async fn delete_audience_grant(
        ctx: &BizContext,
        user_id: &str,
        grant_id: &str,
    ) -> Result<bool, AppError> {
        let grant = AudienceGrantQueries::get_by_id(&ctx.pool, grant_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Grant not found".to_string()))?;

        let audience = AudienceQueries::get_by_id(&ctx.pool, &grant.audience_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Audience not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &audience.project_id, MemberRole::Admin)
            .await?;

        AudienceGrantQueries::delete(&ctx.pool, grant_id).await
    }

    /// Create an invitation
    pub async fn create_invitation(
        ctx: &BizContext,
        user_id: &str,
        audience_id: &str,
        request: CreateInvitationRequest,
    ) -> Result<ReaderInvitationResponse, AppError> {
        let audience = AudienceQueries::get_by_id(&ctx.pool, audience_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Audience not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &audience.project_id, MemberRole::Admin)
            .await?;

        // Generate a token
        let token = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + Duration::days(7);

        let invitation = ReaderInvitationQueries::create(
            &ctx.pool,
            audience_id,
            &request.email,
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
    ) -> Result<ReaderAudienceResponse, AppError> {
        let invitation = ReaderInvitationQueries::get_by_token(&ctx.pool, token)
            .await?
            .ok_or_else(|| AppError::InvalidToken("Invitation not found".to_string()))?;

        // Check if invitation has expired
        if invitation.expires_at < Utc::now() {
            return Err(AppError::TokenExpired);
        }

        // Find or create the reader
        let reader = ReaderQueries::get_by_email(&ctx.pool, &invitation.email).await?;
        let reader = if let Some(reader) = reader {
            reader
        } else {
            ReaderQueries::create(&ctx.pool, &invitation.email, None).await?
        };

        // Add reader to audience
        let reader_audience =
            ReaderAudienceQueries::create(&ctx.pool, &reader.id, &invitation.audience_id).await?;

        // Delete the invitation
        ReaderInvitationQueries::delete(&ctx.pool, &invitation.id).await?;

        // Log the action
        ReaderAuditLogQueries::create(
            &ctx.pool,
            &reader.id,
            &invitation.audience_id,
            "invitation_accepted",
            serde_json::json!({ "invitation_id": invitation.id }),
        )
        .await?;

        Ok(reader_audience.into())
    }

    /// List invitations for an audience
    pub async fn list_invitations(
        ctx: &BizContext,
        user_id: &str,
        audience_id: &str,
    ) -> Result<Vec<ReaderInvitationResponse>, AppError> {
        let audience = AudienceQueries::get_by_id(&ctx.pool, audience_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Audience not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &audience.project_id, MemberRole::Admin)
            .await?;

        let invitations = ReaderInvitationQueries::get_by_audience(&ctx.pool, audience_id).await?;

        Ok(invitations.into_iter().map(|i| i.into()).collect())
    }

    /// List grants for an audience
    pub async fn list_audience_grants(
        ctx: &BizContext,
        user_id: &str,
        audience_id: &str,
    ) -> Result<Vec<AudienceGrantResponse>, AppError> {
        let audience = AudienceQueries::get_by_id(&ctx.pool, audience_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Audience not found".to_string()))?;

        ctx.authz
            .require_project_role(user_id, &audience.project_id, MemberRole::Viewer)
            .await?;

        let grants = AudienceGrantQueries::get_by_audience(&ctx.pool, audience_id).await?;
        Ok(grants.into_iter().map(|g| g.into()).collect())
    }

    /// Create a reader invitation
    pub async fn create_reader_invitation(
        ctx: &BizContext,
        user_id: &str,
        audience_id: &str,
        request: CreateInvitationRequest,
    ) -> Result<ReaderInvitationResponse, AppError> {
        Self::create_invitation(ctx, user_id, audience_id, request).await
    }

    /// Accept a reader invitation
    pub async fn accept_reader_invitation(
        ctx: &BizContext,
        token: &str,
    ) -> Result<ReaderAudienceResponse, AppError> {
        Self::accept_invitation(ctx, token).await
    }

    /// Revoke an invitation
    pub async fn revoke_invitation(
        ctx: &BizContext,
        user_id: &str,
        invitation_id: &str,
    ) -> Result<bool, AppError> {
        let invitation = ReaderInvitationQueries::get_by_id(&ctx.pool, invitation_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invitation not found".to_string()))?;

        let audience = AudienceQueries::get_by_id(&ctx.pool, &invitation.audience_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Audience not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &audience.project_id, MemberRole::Admin)
            .await?;

        ReaderInvitationQueries::delete(&ctx.pool, invitation_id).await
    }

    /// Create a reader session
    pub async fn create_reader_session(
        ctx: &BizContext,
        reader_id: &str,
        auth_service: &AuthService,
    ) -> Result<String, AppError> {
        // Generate a session token
        let session_token = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + Duration::days(30);

        // Create the session
        ReaderSessionQueries::create(&ctx.pool, reader_id, &session_token, expires_at).await?;

        // Create JWT token
        let claims = ReaderJwtClaims::new(reader_id, "", ""); // Would need project_id and audience_id
        let token = JwtService::create_reader_jwt(
            reader_id,
            "", // project_id
            "", // audience_id
            auth_service,
        )?;

        Ok(token)
    }

    /// Validate reader JWT
    pub async fn validate_reader_jwt(
        ctx: &BizContext,
        auth_service: &AuthService,
        token: &str,
        project_id: &str,
    ) -> Result<cms_auth::ReaderPrincipal, AppError> {
        // Decode the JWT
        let claims = JwtService::validate_reader_jwt(token, auth_service)?;

        // Check replay protection
        let provider = if let (Some(issuer), Some(audience)) = (&claims.issuer, &claims.audience) {
            JwtAccessProviderQueries::get_by_issuer_and_audience(&ctx.pool, issuer, audience)
                .await?
        } else {
            None
        };

        if let Some(provider) = provider {
            let is_replay =
                JwtReplayQueries::is_replayed(&ctx.pool, &claims.jti, &provider.id).await?;

            if is_replay {
                return Err(AppError::InvalidToken("JWT replay detected".to_string()));
            }

            // Record the JWT usage
            JwtReplayQueries::create(&ctx.pool, &claims.jti, &provider.id).await?;
        }

        // Verify reader has access to the project
        let has_access =
            ReaderAudienceQueries::has_grant_for_project(&ctx.pool, &claims.reader_id, project_id)
                .await?;

        if !has_access {
            return Err(AppError::AccessDenied(
                "Reader does not have access to this project".to_string(),
            ));
        }

        Ok(cms_auth::ReaderPrincipal {
            reader_id: claims.reader_id,
            project_id: project_id.to_string(),
        })
    }

    /// Create a JWT access provider
    pub async fn create_jwt_access_provider(
        ctx: &BizContext,
        user_id: &str,
        name: &str,
        issuer: &str,
        audience: &str,
        secret: &str,
    ) -> Result<JwtAccessProvider, AppError> {
        // Check if user has admin role in the project
        // (This would need to be implemented based on the project context)

        let provider =
            JwtAccessProviderQueries::create(&ctx.pool, name, issuer, audience, secret).await?;

        Ok(provider)
    }

    /// List JWT access providers
    pub async fn list_jwt_access_providers(
        ctx: &BizContext,
        user_id: &str,
    ) -> Result<Vec<JwtAccessProvider>, AppError> {
        // In a real implementation, this would be filtered by user permissions
        // For now, return all providers

        // This is a placeholder - in practice, we'd need to query with proper filtering
        Ok(Vec::new())
    }

    /// Delete a JWT access provider
    pub async fn delete_jwt_access_provider(
        ctx: &BizContext,
        user_id: &str,
        provider_id: &str,
    ) -> Result<bool, AppError> {
        // Check if user has permission to delete this provider
        // (This would need to be implemented)

        JwtAccessProviderQueries::delete(&ctx.pool, provider_id).await
    }

    /// List audit logs for a reader
    pub async fn list_audit_logs(
        ctx: &BizContext,
        user_id: &str,
        reader_id: &str,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResponse<ReaderAuditLog>, AppError> {
        // Check if user has admin role
        // (This would need to be implemented based on the project context)

        let logs = ReaderAuditLogQueries::get_by_reader(
            &ctx.pool,
            reader_id,
            Some(page as i64),
            Some(page_size as i64),
        )
        .await?;

        let total = 0; // Would need to count

        Ok(PaginatedResponse::new(logs, total, page, page_size))
    }
}
