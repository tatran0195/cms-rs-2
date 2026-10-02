//! Org API module
//!
//! This module contains handlers for organization routes.

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the org router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(list_orgs_handler))
        .route("/", post(create_org_handler))
        .route("/{id}", get(get_org_handler))
        .route("/{id}", put(update_org_handler))
        .route("/{id}", delete(delete_org_handler))
        // Member routes
        .route("/{id}/members", get(list_members_handler))
        .route("/{id}/members", post(add_member_handler))
        .route("/{id}/members/{member_id}", put(update_member_role_handler))
        .route("/{id}/members/{member_id}", delete(remove_member_handler))
        // Invitation routes
        .route("/{id}/invitations", get(list_invitations_handler))
        .route("/{id}/invitations", post(create_invitation_handler))
        .route(
            "/{id}/invitations/{invitation_id}",
            delete(revoke_invitation_handler),
        )
        .with_state(state)
}
