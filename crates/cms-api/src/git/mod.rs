//! Git API module
//!
//! This module contains handlers for Git-related routes.

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the Git router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        // Connection routes
        .route("/connections", post(create_git_connection_handler))
        .route("/connections/{id}", get(get_git_connection_handler))
        .route(
            "/projects/{project_id}/connections",
            get(list_git_connections_handler),
        )
        .route("/connections/{id}", put(update_git_connection_handler))
        .route("/connections/{id}", delete(delete_git_connection_handler))
        // Sync routes
        .route("/connections/{id}/sync", post(trigger_git_sync_handler))
        .route(
            "/connections/{id}/sync",
            get(list_git_sync_operations_handler),
        )
        .route("/sync/{id}", get(get_git_sync_operation_handler))
        .route("/connections/{id}/status", get(get_git_sync_status_handler))
        .with_state(state)
}
