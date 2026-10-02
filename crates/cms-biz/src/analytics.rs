//! Analytics Business Logic
//!
//! This module contains business logic for analytics tracking.

use cms_db::analytics::{AnalyticsEventQueries, AnalyticsQueries};
use cms_entity::{
    analytics::{
        AnalyticsDashboardResponse, AnalyticsEvent, AnalyticsEventResponse, AnalyticsQueryRequest,
        AnalyticsQueryResponse, AnalyticsResultItem,
    },
    common::MemberRole,
};

use crate::{AppError, BizContext};

/// Analytics service
pub struct AnalyticsService;

impl AnalyticsService {
    /// Record an analytics event
    #[allow(clippy::too_many_arguments)]
    pub async fn record_event(
        ctx: &BizContext,
        org_id: Option<&str>,
        project_id: Option<&str>,
        user_id: Option<&str>,
        event_type: &str,
        metadata: serde_json::Value,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<AnalyticsEventResponse, AppError> {
        let event = AnalyticsEventQueries::create(
            &ctx.pool, org_id, project_id, user_id, event_type, metadata, ip_address, user_agent,
        )
        .await?;

        Ok(event.into())
    }

    /// Query analytics events
    pub async fn query_events(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        request: AnalyticsQueryRequest,
        page: u64,
        page_size: u64,
    ) -> Result<AnalyticsQueryResponse, AppError> {
        // Check if user has admin role in the organization
        ctx.authz
            .require_org_admin(user_id, org_id)
            .await?;

        let limit = page_size as i64;
        let offset = ((page.saturating_sub(1)) as i64) * limit;

        let events = AnalyticsEventQueries::query(
            &ctx.pool,
            request.project_id.as_deref(),
            request.user_id.as_deref(),
            request.event_type.as_deref(),
            request.start_date,
            request.end_date,
            limit,
            offset,
        )
        .await?;

        let event_responses: Vec<AnalyticsEventResponse> =
            events.into_iter().map(|e| e.into()).collect();
        let total = event_responses.len() as i64;

        Ok(AnalyticsQueryResponse {
            query: request,
            results: vec![],
            total,
            events: event_responses,
            page: page as i64,
            page_size: page_size as i64,
        })
    }

    /// Get analytics summary
    pub async fn get_summary(
        ctx: &BizContext,
        user_id: &str,
        org_id: &str,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<serde_json::Value, AppError> {
        // Check if user has admin role in the organization
        ctx.authz
            .require_org_admin(user_id, org_id)
            .await?;

        AnalyticsQueries::get_summary(&ctx.pool, org_id, start_date, end_date).await
    }

    /// Track an analytics event
    pub async fn track_event(
        ctx: &BizContext,
        request: cms_entity::analytics::TrackAnalyticsEventRequest,
    ) -> Result<AnalyticsEventResponse, AppError> {
        Self::record_event(
            ctx,
            request.organization_id.as_deref(),
            request.project_id.as_deref(),
            request.user_id.as_deref(),
            &request.event_type,
            request.metadata,
            request.ip_address.as_deref(),
            request.user_agent.as_deref(),
        )
        .await
    }

    /// List analytics events
    pub async fn list_events(
        ctx: &BizContext,
        user_id: &str,
        query: cms_entity::analytics::ListAnalyticsEventsQuery,
    ) -> Result<cms_entity::common::PaginatedResponse<AnalyticsEventResponse>, AppError> {
        let org_id = query.organization_id.clone().unwrap_or_default();
        let page = query.page.unwrap_or(1) as u64;
        let page_size = query.page_size.unwrap_or(20) as u64;
        let response = Self::query_events(ctx, user_id, &org_id, query, page, page_size).await?;
        Ok(cms_entity::common::PaginatedResponse::new(
            response.events,
            response.total as u64,
            page,
            page_size,
        ))
    }

    /// Query analytics
    pub async fn query_analytics(
        ctx: &BizContext,
        user_id: &str,
        request: AnalyticsQueryRequest,
    ) -> Result<AnalyticsQueryResponse, AppError> {
        let org_id = request.organization_id.clone().unwrap_or_default();
        let page = request.page.unwrap_or(1) as u64;
        let page_size = request.page_size.unwrap_or(20) as u64;
        Self::query_events(ctx, user_id, &org_id, request, page, page_size).await
    }

    /// Get dashboard for a project
    pub async fn get_dashboard(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<AnalyticsDashboardResponse, AppError> {
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;

        AnalyticsQueries::get_dashboard(&ctx.pool, project_id).await
    }

    /// Get page views
    pub async fn get_page_views(
        ctx: &BizContext,
        _user_id: &str,
        page_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        AnalyticsQueries::get_page_views(&ctx.pool, page_id).await
    }

    /// Get organization stats
    pub async fn get_organization_stats(
        ctx: &BizContext,
        org_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        AnalyticsQueries::get_organization_stats(&ctx.pool, org_id).await
    }

    /// Get system stats
    pub async fn get_system_stats(ctx: &BizContext) -> Result<serde_json::Value, AppError> {
        AnalyticsQueries::get_system_stats(&ctx.pool).await
    }
}

/// Process analytics job (for worker)
pub async fn process_analytics_job(
    pool: &cms_db::PgPool,
    payload: &serde_json::Value,
) -> Result<(), AppError> {
    let event_type = payload
        .get("event_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing event_type".to_string()))?;
    let org_id = payload.get("org_id").and_then(|v| v.as_str());
    let project_id = payload.get("project_id").and_then(|v| v.as_str());
    let user_id = payload.get("user_id").and_then(|v| v.as_str());
    let metadata = payload.get("metadata").cloned().unwrap_or_default();
    let ip_address = payload.get("ip_address").and_then(|v| v.as_str());
    let user_agent = payload.get("user_agent").and_then(|v| v.as_str());

    AnalyticsEventQueries::create(
        pool, org_id, project_id, user_id, event_type, metadata, ip_address, user_agent,
    )
    .await?;

    Ok(())
}
