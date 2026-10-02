//! MCP API module
//!
//! This module contains handlers for MCP protocol routes.

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the MCP router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/audit-events", get(list_mcp_audit_events_handler))
        .route("/server-info", get(get_mcp_server_info_handler))
        .route("/tools", get(list_mcp_tools_handler))
        .route("/tools/{tool_name}", post(call_mcp_tool_handler))
        .route("/resources", get(list_mcp_resources_handler))
        .route("/resources/{uri}", get(read_mcp_resource_handler))
        .with_state(state)
}
