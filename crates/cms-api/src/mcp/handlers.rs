//! MCP handlers
//!
//! This module contains the actual implementation of MCP-related handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::mcp::McpService;
use cms_entity::{
    common::{Id, PaginatedResponse},
    mcp::{ListMcpAuditEventsQuery, McpAuditEventResponse},
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List MCP audit events
///
/// Returns a paginated list of MCP audit events filtered by various criteria.
#[utoipa::path(
    get,
    path = "/mcp/audit-events",
    tag = "mcp",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("organization_id", Query, description = "Filter by organization ID"),
        ("project_id", Query, description = "Filter by project ID"),
        ("user_id", Query, description = "Filter by user ID"),
        ("operation", Query, description = "Filter by operation type"),
        ("start_date", Query, description = "Filter by start date"),
        ("end_date", Query, description = "Filter by end date"),
        ("limit", Query, description = "Number of items per page"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of MCP audit events", body = PaginatedResponse<McpAuditEventResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_mcp_audit_events_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListMcpAuditEventsQuery>,
) -> Result<Json<PaginatedResponse<McpAuditEventResponse>>, AppError> {
    let result = McpService::list_audit_events(
        &state.biz_context,
        &auth.user.id,
        query.organization_id.as_deref(),
        query.project_id.as_deref(),
        query.user_id.as_deref(),
        query.operation.as_deref(),
        query.start_date,
        query.end_date,
        query.limit,
        query.offset,
    )
    .await?;

    Ok(Json(result))
}

/// Get MCP server information
///
/// Returns information about the MCP server.
#[utoipa::path(
    get,
    path = "/mcp/server-info",
    tag = "mcp",
    responses(
        (status = 200, description = "MCP server information", body = serde_json::Value),
    )
)]
pub async fn get_mcp_server_info_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let info = McpService::get_server_info(&state.biz_context).await?;

    Ok(Json(serde_json::json!(info)))
}

/// List available MCP tools
///
/// Returns a list of all available MCP tools.
#[utoipa::path(
    get,
    path = "/mcp/tools",
    tag = "mcp",
    responses(
        (status = 200, description = "List of MCP tools", body = serde_json::Value),
    )
)]
pub async fn list_mcp_tools_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let tools = McpService::list_tools(&state.biz_context).await?;

    Ok(Json(serde_json::json!(tools)))
}

/// Call an MCP tool
///
/// Calls a specific MCP tool with the provided arguments.
#[utoipa::path(
    post,
    path = "/mcp/tools/{tool_name}",
    tag = "mcp",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("tool_name", Path, description = "The name of the tool to call"),
    ),
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Tool execution result", body = serde_json::Value),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Tool not found"),
    )
)]
pub async fn call_mcp_tool_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(tool_name): Path<String>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result =
        McpService::call_tool(&state.biz_context, &auth.user.id, &tool_name, request).await?;

    Ok(Json(result))
}

/// List MCP resources
///
/// Returns a list of all available MCP resources.
#[utoipa::path(
    get,
    path = "/mcp/resources",
    tag = "mcp",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    responses(
        (status = 200, description = "List of MCP resources", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn list_mcp_resources_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<serde_json::Value>, AppError> {
    let resources = McpService::list_resources(&state.biz_context, &auth.user.id).await?;

    Ok(Json(serde_json::json!(resources)))
}

/// Read an MCP resource
///
/// Reads the content of a specific MCP resource.
#[utoipa::path(
    get,
    path = "/mcp/resources/{uri}",
    tag = "mcp",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("uri", Path, description = "The URI of the resource to read"),
    ),
    responses(
        (status = 200, description = "Resource content", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Resource not found"),
    )
)]
pub async fn read_mcp_resource_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(uri): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let content = McpService::read_resource(&state.biz_context, &auth.user.id, &uri).await?;

    Ok(Json(serde_json::json!(content)))
}
