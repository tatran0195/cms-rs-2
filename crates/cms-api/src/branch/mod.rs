//! Branch API module
//!
//! This module contains handlers for branch routes.

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the branch router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(list_branches_handler))
        .route("/", post(create_branch_handler))
        .route("/{id}", get(get_branch_handler))
        .route("/{id}", put(update_branch_handler))
        .route("/{id}", delete(delete_branch_handler))
        .with_state(state)
}
