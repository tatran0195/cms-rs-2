//! Usage API module
//!
//! This module contains handlers for usage and billing routes.

use std::sync::Arc;

use axum::{
    routing::{get, post, put},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the usage router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        // Usage plan routes
        .route("/plans", get(list_usage_plans_handler))
        .route("/plans/{id}", get(get_usage_plan_handler))
        // Usage meter routes
        .route("/meters", get(list_usage_meters_handler))
        .route("/meters/{id}", get(get_usage_meter_handler))
        // Usage entitlement routes
        .route("/entitlements", get(list_usage_entitlements_handler))
        // Organization usage plan routes
        .route(
            "/orgs/{org_id}/plan",
            get(get_organization_usage_plan_handler),
        )
        .route(
            "/orgs/{org_id}/plan",
            put(update_organization_usage_plan_handler),
        )
        // Usage tracking
        .route("/track", post(track_usage_event_handler))
        .route("/orgs/{org_id}/summary", get(get_usage_summary_handler))
        .with_state(state)
}
