//! Admin API module
//!
//! This module contains handlers for admin-only routes.

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;
pub mod parity;

use handlers::*;

use crate::extractors::UserId;

/// Create the admin router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/orgs", get(list_all_organizations_handler))
        .route("/orgs/{id}/stats", get(get_organization_stats_handler))
        .route("/stats", get(get_system_stats_handler))
        .route("/health", get(get_system_health_handler))
        .route("/overview", get(parity::overview_handler))
        .route("/funnel", get(parity::funnel_handler))
        .route("/users", get(parity::users_handler))
        .route("/users/{id}", get(parity::user_handler))
        .route("/users/{id}/role", post(parity::set_user_role_handler))
        .route("/users/{id}/suspend", post(parity::suspend_user_handler))
        .route(
            "/users/{id}/unsuspend",
            post(parity::unsuspend_user_handler),
        )
        .route("/sites", get(parity::sites_handler))
        .route("/sites/{id}", get(parity::site_handler))
        .route("/operations", get(parity::operations_handler))
        .route(
            "/organizations/invite",
            post(parity::invite_organization_handler),
        )
        .route("/sites/{id}/takedown", post(parity::takedown_site_handler))
        .route("/sites/{id}/restore", post(parity::restore_site_handler))
        .with_state(state)
}
