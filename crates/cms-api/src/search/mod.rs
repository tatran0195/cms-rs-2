//! Search API module
//!
//! This module contains handlers for search routes.

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the search router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(search_handler))
        .route("/reindex", post(reindex_handler))
        .route("/index-runs", get(list_search_index_runs_handler))
        .route("/index-runs/{id}", get(get_search_index_run_handler))
        .route("/status/{project_id}", get(get_search_status_handler))
        .with_state(state)
}
