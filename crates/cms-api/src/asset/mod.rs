//! Asset API module
//!
//! This module contains handlers for asset routes.

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the asset router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(list_assets_handler))
        .route("/upload", post(upload_asset_handler))
        .route(
            "/upload-multipart/{project_id}",
            post(upload_asset_multipart_handler),
        )
        .route("/{id}", get(get_asset_handler))
        .route("/{id}", put(update_asset_handler))
        .route("/{id}", delete(delete_asset_handler))
        .with_state(state)
}
