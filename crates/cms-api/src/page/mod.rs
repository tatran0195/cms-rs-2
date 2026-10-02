//! Page API module
//!
//! This module contains handlers for page routes.

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the page router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(list_pages_handler))
        .route("/", post(create_page_handler))
        .route("/{id}", get(get_page_handler))
        .route("/{id}", put(update_page_handler))
        .route("/{id}", delete(delete_page_handler))
        .with_state(state)
}
