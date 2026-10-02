//! OpenAPI API module
//!
//! This module contains handlers for OpenAPI routes.

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod docs;
pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the OpenAPI router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/documents", get(list_openapi_documents_handler))
        .route("/documents", post(create_openapi_document_handler))
        .route("/documents/{id}", get(get_openapi_document_handler))
        .route("/documents/{id}", put(update_openapi_document_handler))
        .route("/documents/{id}", delete(delete_openapi_document_handler))
        .route(
            "/documents/{id}/parse",
            post(parse_openapi_document_handler),
        )
        .route("/documents/{id}/content", get(get_openapi_content_handler))
        .with_state(state)
}
