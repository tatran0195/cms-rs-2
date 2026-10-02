//! Reader Access API module
//!
//! This module contains handlers for reader access routes.

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the reader access router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        // Audience routes
        .route("/audiences", post(create_audience_handler))
        .route(
            "/projects/{project_id}/audiences",
            get(list_audiences_handler),
        )
        .route("/audiences/{id}", get(get_audience_handler))
        .route("/audiences/{id}", put(update_audience_handler))
        .route("/audiences/{id}", delete(delete_audience_handler))
        // Audience grant routes
        .route("/audience-grants", post(create_audience_grant_handler))
        .route(
            "/audiences/{audience_id}/grants",
            get(list_audience_grants_handler),
        )
        .route(
            "/audience-grants/{id}",
            delete(delete_audience_grant_handler),
        )
        // Invitation routes
        .route("/invitations", post(create_reader_invitation_handler))
        .route(
            "/invitations/accept",
            post(accept_reader_invitation_handler),
        )
        .with_state(state)
}
