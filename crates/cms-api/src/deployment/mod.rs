//! Deployment API module
//!
//! This module contains handlers for deployment routes.

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the deployment router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(list_deployments_handler))
        .route("/", post(create_deployment_handler))
        .route("/{id}", get(get_deployment_handler))
        .route("/{id}/logs", get(get_deployment_logs_handler))
        .route("/{id}/retry", post(retry_deployment_handler))
        .route("/{id}/cancel", post(cancel_deployment_handler))
        .with_state(state)
}
