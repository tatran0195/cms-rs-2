//! Analytics API module
//!
//! This module contains handlers for analytics routes.

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the analytics router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/track", post(track_event_handler))
        .route("/events", get(list_analytics_events_handler))
        .route("/query", post(query_analytics_handler))
        .route(
            "/dashboard/{project_id}",
            get(get_analytics_dashboard_handler),
        )
        .route("/pages/{page_id}/views", get(get_page_views_handler))
        .with_state(state)
}
