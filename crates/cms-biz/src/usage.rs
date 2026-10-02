//! Usage and Billing Business Logic
//!
//! This module contains business logic for usage tracking, metering,
//! and entitlement management.

use chrono::Utc;
use cms_db::usage::{
    AnalyticsEventQueries, OrganizationUsagePlanQueries, UsageCheckpointQueries,
    UsageEntitlementQueries, UsageMeterQueries, UsagePlanMeterQueries, UsagePlanQueries,
};
use cms_entity::{
    common::{Id, MemberRole, PaginatedResponse},
    usage::{
        OrganizationUsagePlan, OrganizationUsagePlanResponse, UsageEntitlement,
        UsageEntitlementResponse, UsageMeter, UsageMeterResponse, UsagePlan, UsagePlanMeter,
        UsagePlanMeterResponse, UsagePlanResponse,
    },
};
use uuid::Uuid;

use crate::{AppError, BizContext};

/// Usage service
pub struct UsageService;

impl UsageService {
    /// Create a usage plan
    pub async fn create_usage_plan(
        ctx: &BizContext,
        user_id: &str,
        name: &str,
        description: Option<&str>,
        price: i64,
        billing_period: &str,
    ) -> Result<UsagePlanResponse, AppError> {
        let plan =
            UsagePlanQueries::create(&ctx.pool, name, description, price, billing_period).await?;

        Ok(plan.into())
    }

    /// Get a usage plan
    pub async fn get_usage_plan(
        ctx: &BizContext,
        plan_id: &str,
    ) -> Result<UsagePlanResponse, AppError> {
        let plan = UsagePlanQueries::get_by_id(&ctx.pool, plan_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Usage plan not found".to_string()))?;

        Ok(plan.into())
    }

    /// List usage plans
    pub async fn list_usage_plans(
        ctx: &BizContext,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResponse<UsagePlanResponse>, AppError> {
        let plans =
            UsagePlanQueries::get_all(&ctx.pool, Some(page as i64), Some(page_size as i64)).await?;

        let total = UsagePlanQueries::count(&ctx.pool).await?;

        Ok(PaginatedResponse::new(
            plans.into_iter().map(|p| p.into()).collect(),
            total as u64,
            page,
            page_size,
        ))
    }

    /// Create a usage meter
    pub async fn create_usage_meter(
        ctx: &BizContext,
        user_id: &str,
        code: &str,
        name: &str,
        description: Option<&str>,
        unit: &str,
    ) -> Result<UsageMeterResponse, AppError> {
        let meter = UsageMeterQueries::create(&ctx.pool, code, name, description, unit).await?;

        Ok(meter.into())
    }

    /// Get a usage meter
    pub async fn get_usage_meter(
        ctx: &BizContext,
        meter_id: &str,
    ) -> Result<UsageMeterResponse, AppError> {
        let meter = UsageMeterQueries::get_by_id(&ctx.pool, meter_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Usage meter not found".to_string()))?;

        Ok(meter.into())
    }

    /// List usage meters
    pub async fn list_usage_meters(
        ctx: &BizContext,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResponse<UsageMeterResponse>, AppError> {
        let meters =
            UsageMeterQueries::get_all(&ctx.pool, Some(page as i64), Some(page_size as i64))
                .await?;

        let total = UsageMeterQueries::count(&ctx.pool).await?;

        Ok(PaginatedResponse::new(
            meters.into_iter().map(|m| m.into()).collect(),
            total as u64,
            page,
            page_size,
        ))
    }

    /// Assign usage plan to organization
    pub async fn assign_usage_plan(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        plan_id: &str,
        starts_at: chrono::DateTime<chrono::Utc>,
        ends_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<OrganizationUsagePlanResponse, AppError> {
        // Check if user has admin permissions for the organization
        ctx.authz
            .require_org_admin(user_id, org_id)
            .await?;

        let plan = UsagePlanQueries::get_by_id(&ctx.pool, plan_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Usage plan not found".to_string()))?;

        // Check if organization already has a plan
        let existing = OrganizationUsagePlanQueries::get_by_organization(&ctx.pool, org_id).await?;
        if existing.is_some() {
            return Err(AppError::Conflict(
                "Organization already has a usage plan".to_string(),
            ));
        }

        let org_plan =
            OrganizationUsagePlanQueries::create(&ctx.pool, org_id, plan_id, starts_at, ends_at)
                .await?;

        Ok(org_plan.into())
    }

    /// Get organization's usage plan
    pub async fn get_organization_usage_plan(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
    ) -> Result<Option<OrganizationUsagePlanResponse>, AppError> {
        // Check if user has access to the organization
        ctx.authz
            .require_org_member(user_id, org_id)
            .await?;

        let org_plan = OrganizationUsagePlanQueries::get_by_organization(&ctx.pool, org_id).await?;

        Ok(org_plan.map(|p| p.into()))
    }

    /// Update organization's usage plan
    pub async fn update_organization_usage_plan(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        usage_plan_id: &str,
    ) -> Result<OrganizationUsagePlanResponse, AppError> {
        ctx.authz
            .require_org_admin(user_id, org_id)
            .await?;

        let _plan = UsagePlanQueries::get_by_id(&ctx.pool, usage_plan_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Usage plan not found".to_string()))?;

        let org_plan =
            OrganizationUsagePlanQueries::update_plan(&ctx.pool, org_id, usage_plan_id).await?;

        Ok(org_plan.into())
    }

    /// List all usage entitlements
    pub async fn list_usage_entitlements(
        ctx: &BizContext,
        _user_id: &str,
    ) -> Result<Vec<UsageEntitlementResponse>, AppError> {
        let entitlements = UsageEntitlementQueries::get_all(&ctx.pool, None, None).await?;
        Ok(entitlements.into_iter().map(|e| e.into()).collect())
    }

    /// Set entitlement for organization
    pub async fn set_entitlement(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        feature_code: &str,
        is_enabled: bool,
        limit_value: Option<i64>,
    ) -> Result<UsageEntitlementResponse, AppError> {
        // Check if user has admin permissions for the organization
        ctx.authz
            .require_org_admin(user_id, org_id)
            .await?;

        let entitlement = UsageEntitlementQueries::create_or_update(
            &ctx.pool,
            org_id,
            feature_code,
            None,
            is_enabled,
        )
        .await?;

        Ok(entitlement.into())
    }

    /// Check if organization has entitlement
    pub async fn check_entitlement(
        ctx: &BizContext,
        _org_id: &str,
        feature_code: &str,
    ) -> Result<bool, AppError> {
        let entitlement = UsageEntitlementQueries::get_by_code(&ctx.pool, feature_code).await?;

        Ok(entitlement.map(|e| e.is_enabled).unwrap_or(false))
    }

    /// Record a usage event
    #[allow(clippy::too_many_arguments)]
    pub async fn record_usage_event(
        ctx: &BizContext,
        org_id: &str,
        project_id: Option<&str>,
        event_type: &str,
        metadata: serde_json::Value,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(), AppError> {
        AnalyticsEventQueries::create(
            &ctx.pool,
            Some(org_id),
            project_id,
            None,
            event_type,
            metadata,
            ip_address,
            user_agent,
        )
        .await?;

        Ok(())
    }

    /// Track a usage event
    pub async fn track_usage_event(
        ctx: &BizContext,
        request: cms_entity::usage::TrackAnalyticsEventRequest,
    ) -> Result<(), AppError> {
        Self::record_usage_event(
            ctx,
            "",
            None,
            &request.event_type,
            request.metadata,
            request.ip_address.as_deref(),
            request.user_agent.as_deref(),
        )
        .await
    }

    /// Get usage summary for an organization
    pub async fn get_usage_summary(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        ctx.authz
            .require_org_admin(user_id, org_id)
            .await?;
        let plan = Self::get_organization_usage_plan(ctx, user_id, org_id).await?;
        Ok(serde_json::json!({
            "organization_id": org_id,
            "plan": plan,
            "usage": {}
        }))
    }
}

/// Process usage job (for worker)
pub async fn process_usage_job(
    pool: &cms_db::PgPool,
    payload: &serde_json::Value,
) -> Result<(), AppError> {
    // Parse the job payload
    let event_type = payload
        .get("event_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing event_type".to_string()))?;
    let org_id = payload.get("org_id").and_then(|v| v.as_str());
    let user_id = payload.get("user_id").and_then(|v| v.as_str());
    let metadata = payload.get("metadata").cloned().unwrap_or_default();

    // Check for idempotent ingestion first
    if let Some(org_id) = org_id {
        let today = chrono::Utc::now().date_naive();
        let period_start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();

        let _checkpoint =
            UsageCheckpointQueries::create(pool, event_type, org_id, period_start).await?;
    }

    // Record the usage event
    AnalyticsEventQueries::create(
        pool, org_id, None, user_id, event_type, metadata, None, None,
    )
    .await?;

    Ok(())
}
