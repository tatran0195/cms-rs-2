//! Integration API module
//!
//! This module contains handlers for integration routes.

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the integration router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(list_integrations_handler))
        .route("/", post(create_integration_handler))
        .route("/{id}", get(get_integration_handler))
        .route("/{id}", put(update_integration_handler))
        .route("/{id}", delete(delete_integration_handler))
        .route("/{id}/enable", post(enable_integration_handler))
        .route("/{id}/disable", post(disable_integration_handler))
        .route("/{id}/test", post(test_integration_handler))
        .with_state(state)
}
