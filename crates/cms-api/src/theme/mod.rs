//! Theme API module
//!
//! This module contains handlers for theme routes.

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the theme router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(list_themes_handler))
        .route("/", post(create_theme_handler))
        .route("/{id}", get(get_theme_handler))
        .route("/{id}", put(update_theme_handler))
        .route("/{id}", delete(delete_theme_handler))
        .route("/{id}/css", get(get_theme_css_handler))
        .route("/projects/{project_id}", post(set_project_theme_handler))
        .with_state(state)
}
