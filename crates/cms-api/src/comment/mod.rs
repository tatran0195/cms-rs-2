//! Comment API module
//!
//! This module contains handlers for comment routes.

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the comment router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(list_comments_handler))
        .route("/", post(create_comment_handler))
        .route("/{id}", get(get_comment_handler))
        .route("/{id}", put(update_comment_handler))
        .route("/{id}", delete(delete_comment_handler))
        .route("/{id}/resolve", post(resolve_comment_handler))
        .route("/pages/{page_id}", get(list_page_comments_handler))
        .with_state(state)
}
