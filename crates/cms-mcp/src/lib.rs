//! CMS MCP Server
//!
//! This crate provides the Model Context Protocol (MCP) server implementation.
//! MCP allows AI agents to query CMS's documentation programmatically.

use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json, Router,
};
use cms_biz::{mcp::McpService, BizContext};
use cms_entity::mcp::{McpCapabilities, McpRequest, McpResponse, McpToolResult};
use cms_error::AppError;

/// MCP router
pub fn mcp_router(ctx: Arc<BizContext>) -> Router {
    Router::new()
        .route("/mcp/capabilities", axum::routing::get(get_capabilities))
        .route("/mcp/tools", axum::routing::post(execute_tool))
        .with_state(ctx)
}

/// Get MCP capabilities
async fn get_capabilities(
    State(ctx): State<Arc<BizContext>>,
) -> Result<Json<McpCapabilities>, AppError> {
    let capabilities = McpService::get_capabilities(&ctx, None, None, None).await?;
    Ok(Json(capabilities))
}

/// Execute an MCP tool
async fn execute_tool(
    State(ctx): State<Arc<BizContext>>,
    Json(request): Json<McpRequest>,
) -> Result<Json<McpResponse>, AppError> {
    let response = McpService::execute_tool(&ctx, None, None, request).await?;
    Ok(Json(response))
}

/// MCP server implementation
pub struct McpServer;

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServer {
    /// Create a new MCP server
    pub fn new() -> Self {
        Self
    }

    /// Handle an MCP request
    pub async fn handle_request(
        &self,
        ctx: Arc<BizContext>,
        request: McpRequest,
    ) -> Result<McpResponse, AppError> {
        McpService::execute_tool(&ctx, None, None, request).await
    }
}

/// MCP client for testing
pub struct McpClient;

impl McpClient {
    pub async fn get_capabilities(
        &self,
        ctx: Arc<BizContext>,
    ) -> Result<McpCapabilities, AppError> {
        McpService::get_capabilities(&ctx, None, None, None).await
    }

    pub async fn execute_tool(
        &self,
        ctx: Arc<BizContext>,
        request: McpRequest,
    ) -> Result<McpResponse, AppError> {
        McpService::execute_tool(&ctx, None, None, request).await
    }
}

#[cfg(test)]
mod tests {
    use cms_biz::BizContext;

    use super::*;

    #[tokio::test]
    async fn test_get_capabilities() {
        let pool = cms_db::PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
        let ctx = BizContext::new(pool, Arc::new(cms_authz::NoopAuthz));

        let capabilities = McpService::get_capabilities(&ctx, None, None, None)
            .await
            .unwrap();

        assert!(!capabilities.tools.is_empty());
    }
}
