//! Workspace and Members routers
//!
//! Routers for /api/app/workspace and /api/app/members.

use std::sync::Arc;

use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;
use handlers::*;

/// Create the workspace router
pub fn workspace_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(get_workspace_settings_handler))
        .route("/", patch(update_workspace_settings_handler))
        .route("/analytics", get(get_workspace_analytics_handler))
        .with_state(state)
}

/// Create the members router
pub fn members_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(list_workspace_members_handler))
        .route("/invite", post(invite_workspace_member_handler))
        .route("/{id}", delete(remove_workspace_member_handler))
        .route("/{id}/role", patch(update_workspace_member_role_handler))
        .route(
            "/invitations/{id}",
            delete(cancel_workspace_invitation_handler),
        )
        .with_state(state)
}
