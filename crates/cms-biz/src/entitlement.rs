//! Entitlement Business Logic
//!
//! This module contains business logic for entitlements and feature flags.

use cms_db::usage::UsageEntitlementQueries;
use cms_entity::{common::Id, usage::UsageEntitlement};

use crate::{AppError, BizContext};

/// Entitlement service
pub struct EntitlementService;

impl EntitlementService {
    /// Create an entitlement
    pub async fn create_entitlement(
        ctx: &BizContext,
        user_id: &str,
        usage_meter_id: &str,
        name: &str,
        description: Option<&str>,
        is_enabled: bool,
    ) -> Result<UsageEntitlement, AppError> {
        ctx.authz.require_system_admin(user_id).await?;

        let entitlement = UsageEntitlementQueries::create(
            &ctx.pool,
            usage_meter_id,
            name,
            description,
            is_enabled,
        )
        .await?;

        Ok(entitlement)
    }

    /// Get an entitlement
    pub async fn get_entitlement(
        ctx: &BizContext,
        entitlement_id: &str,
    ) -> Result<UsageEntitlement, AppError> {
        let entitlement = UsageEntitlementQueries::get_by_id(&ctx.pool, entitlement_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Entitlement not found".to_string()))?;

        Ok(entitlement)
    }

    /// Update an entitlement
    pub async fn update_entitlement(
        ctx: &BizContext,
        user_id: &str,
        entitlement_id: &str,
        name: Option<&str>,
        description: Option<&str>,
        is_enabled: Option<bool>,
    ) -> Result<UsageEntitlement, AppError> {
        ctx.authz.require_system_admin(user_id).await?;

        let entitlement = UsageEntitlementQueries::update(
            &ctx.pool,
            entitlement_id,
            name,
            description,
            is_enabled,
        )
        .await?;

        Ok(entitlement)
    }

    /// Delete an entitlement
    pub async fn delete_entitlement(
        ctx: &BizContext,
        user_id: &str,
        entitlement_id: &str,
    ) -> Result<bool, AppError> {
        ctx.authz.require_system_admin(user_id).await?;

        UsageEntitlementQueries::delete(&ctx.pool, entitlement_id).await
    }

    /// List entitlements
    pub async fn list_entitlements(
        ctx: &BizContext,
        page: u64,
        page_size: u64,
    ) -> Result<Vec<UsageEntitlement>, AppError> {
        UsageEntitlementQueries::get_all(&ctx.pool, Some(page as i64), Some(page_size as i64)).await
    }
}
