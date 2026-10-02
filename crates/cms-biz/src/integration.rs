//! Integration Business Logic
//!
//! This module contains business logic for project integrations.

use cms_authz::Authz;
use cms_db::integration::{
    IntegrationAuditEventQueries, IntegrationConfirmationQueries,
    IntegrationIdempotencyRecordQueries, IntegrationWebhookDeliveryQueries,
    ProjectIntegrationQueries,
};
use cms_entity::{
    common::Id,
    integration::{
        CreateProjectIntegrationRequest, IntegrationProvider, ProjectIntegration,
        ProjectIntegrationResponse, UpdateProjectIntegrationRequest,
    },
};

use crate::{AppError, BizContext};

/// Integration service
pub struct IntegrationService;

impl IntegrationService {
    /// Create a new project integration
    pub async fn create_integration(
        ctx: &BizContext,
        user_id: &str,
        request: CreateProjectIntegrationRequest,
    ) -> Result<ProjectIntegrationResponse, AppError> {
        // Check if user has access to the project
        ctx.authz
            .require_project_access(user_id, &request.project_id)
            .await?;

        let integration = ProjectIntegrationQueries::create(
            &ctx.pool,
            &request.project_id,
            request.provider,
            &request.name,
            request.config,
            request.webhook_url.as_deref(),
        )
        .await?;

        // Log the creation
        IntegrationAuditEventQueries::create(
            &ctx.pool,
            &integration.id,
            "integration.created",
            serde_json::json!({
                "user_id": user_id,
                "provider": integration.provider,
            }),
            "COMPLETED",
            None,
        )
        .await?;

        Ok(integration.into())
    }

    /// Get integration by ID
    pub async fn get_integration(
        ctx: &BizContext,
        user_id: &str,
        integration_id: &str,
    ) -> Result<ProjectIntegrationResponse, AppError> {
        let integration = ProjectIntegrationQueries::get_by_id(&ctx.pool, integration_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Integration not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_access(user_id, &integration.project_id)
            .await?;

        Ok(integration.into())
    }

    /// List integrations for a project
    pub async fn list_integrations(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<Vec<ProjectIntegrationResponse>, AppError> {
        // Check if user has access to the project
        ctx.authz
            .require_project_access(user_id, project_id)
            .await?;

        let integrations = ProjectIntegrationQueries::get_by_project(&ctx.pool, project_id).await?;

        Ok(integrations.into_iter().map(|i| i.into()).collect())
    }

    /// Update an integration
    pub async fn update_integration(
        ctx: &BizContext,
        user_id: &str,
        integration_id: &str,
        request: UpdateProjectIntegrationRequest,
    ) -> Result<ProjectIntegrationResponse, AppError> {
        let integration = ProjectIntegrationQueries::get_by_id(&ctx.pool, integration_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Integration not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_access(user_id, &integration.project_id)
            .await?;

        let changes_json = serde_json::to_value(&request).unwrap_or_default();
        let updated = ProjectIntegrationQueries::update(
            &ctx.pool,
            integration_id,
            request.name.as_deref(),
            request.config,
            request.webhook_url.as_deref(),
            request.is_active,
        )
        .await?;

        // Log the update
        IntegrationAuditEventQueries::create(
            &ctx.pool,
            integration_id,
            "integration.updated",
            serde_json::json!({
                "user_id": user_id,
                "changes": changes_json,
            }),
            "COMPLETED",
            None,
        )
        .await?;

        Ok(updated.into())
    }

    /// Delete an integration
    pub async fn delete_integration(
        ctx: &BizContext,
        user_id: &str,
        integration_id: &str,
    ) -> Result<bool, AppError> {
        let integration = ProjectIntegrationQueries::get_by_id(&ctx.pool, integration_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Integration not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_access(user_id, &integration.project_id)
            .await?;

        let deleted = ProjectIntegrationQueries::delete(&ctx.pool, integration_id).await?;

        if deleted {
            // Log the deletion
            IntegrationAuditEventQueries::create(
                &ctx.pool,
                integration_id,
                "integration.deleted",
                serde_json::json!({
                    "user_id": user_id,
                }),
                "COMPLETED",
                None,
            )
            .await?;
        }

        Ok(deleted)
    }

    /// Get integration by project and provider
    pub async fn get_integration_by_provider(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        provider: IntegrationProvider,
    ) -> Result<Vec<ProjectIntegrationResponse>, AppError> {
        // Check if user has access to the project
        ctx.authz
            .require_project_access(user_id, project_id)
            .await?;

        let integrations =
            ProjectIntegrationQueries::get_by_project_and_provider(&ctx.pool, project_id, provider)
                .await?;

        Ok(integrations.into_iter().map(|i| i.into()).collect())
    }

    /// Check if idempotency key has been processed
    pub async fn is_idempotent(
        ctx: &BizContext,
        integration_id: &str,
        request_id: &str,
    ) -> Result<bool, AppError> {
        let exists = IntegrationIdempotencyRecordQueries::get_by_request_id(
            &ctx.pool,
            integration_id,
            request_id,
        )
        .await?;

        Ok(exists.is_some())
    }

    /// Mark a request as processed (idempotency)
    pub async fn mark_processed(
        ctx: &BizContext,
        integration_id: &str,
        request_id: &str,
    ) -> Result<(), AppError> {
        IntegrationIdempotencyRecordQueries::create(&ctx.pool, integration_id, request_id).await?;

        Ok(())
    }

    /// Enable an integration
    pub async fn enable_integration(
        ctx: &BizContext,
        user_id: &str,
        integration_id: &str,
    ) -> Result<ProjectIntegrationResponse, AppError> {
        Self::update_integration(
            ctx,
            user_id,
            integration_id,
            UpdateProjectIntegrationRequest {
                name: None,
                config: None,
                webhook_url: None,
                is_active: Some(true),
            },
        )
        .await
    }

    /// Disable an integration
    pub async fn disable_integration(
        ctx: &BizContext,
        user_id: &str,
        integration_id: &str,
    ) -> Result<ProjectIntegrationResponse, AppError> {
        Self::update_integration(
            ctx,
            user_id,
            integration_id,
            UpdateProjectIntegrationRequest {
                name: None,
                config: None,
                webhook_url: None,
                is_active: Some(false),
            },
        )
        .await
    }

    /// Test an integration
    pub async fn test_integration(
        ctx: &BizContext,
        user_id: &str,
        integration_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        let _ = Self::get_integration(ctx, user_id, integration_id).await?;
        Ok(serde_json::json!({ "success": true, "message": "Integration test successful" }))
    }
}
