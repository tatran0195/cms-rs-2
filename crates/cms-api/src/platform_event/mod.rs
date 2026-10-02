//! Platform Event API module
//!
//! This module contains handlers for platform event routes.

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the platform event router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(list_platform_events_handler))
        .route("/", post(create_platform_event_handler))
        .route("/{id}", get(get_platform_event_handler))
        .with_state(state)
}
