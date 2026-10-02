//! Domain API module
//!
//! This module contains handlers for domain routes.

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the domain router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(list_domains_handler))
        .route("/", post(create_domain_handler))
        .route("/{id}", get(get_domain_handler))
        .route("/{id}", put(update_domain_handler))
        .route("/{id}", delete(delete_domain_handler))
        .route("/{id}/verify", post(verify_domain_handler))
        .route("/check/{hostname}", get(check_domain_availability_handler))
        .route(
            "/deployments/{deployment_id}/set-primary",
            post(set_primary_domain_handler),
        )
        .with_state(state)
}
