//! MCP (Model Context Protocol) entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, Timestamp};

/// MCP audit event entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct McpAuditEvent {
    pub id: Id,
    pub organization_id: Option<Id>,
    pub project_id: Option<Id>,
    pub user_id: Option<Id>,
    pub operation: String,
    pub request_id: Option<String>,
    pub response_status: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// MCP audit event response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct McpAuditEventResponse {
    pub id: Id,
    pub organization_id: Option<Id>,
    pub project_id: Option<Id>,
    pub user_id: Option<Id>,
    pub operation: String,
    pub request_id: Option<String>,
    pub response_status: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<McpAuditEvent> for McpAuditEventResponse {
    fn from(event: McpAuditEvent) -> Self {
        Self {
            id: event.id,
            organization_id: event.organization_id,
            project_id: event.project_id,
            user_id: event.user_id,
            operation: event.operation,
            request_id: event.request_id,
            response_status: event.response_status,
            error_message: event.error_message,
            created_at: event.created_at,
        }
    }
}

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP tool response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct McpToolResponse {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP resource definition
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// MCP resource response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct McpResourceResponse {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// MCP server info
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub capabilities: McpCapabilities,
}

/// MCP capabilities
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct McpCapabilities {
    pub tools: Vec<McpTool>,
    #[serde(default)]
    pub prompts: Vec<McpPrompt>,
    #[serde(default)]
    pub resources: Vec<McpResource>,
}

/// MCP prompt definition
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct McpPrompt {
    pub name: String,
    pub description: Option<String>,
}

/// MCP tool call request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct McpToolCallRequest {
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

/// MCP tool call response / result
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct McpToolCallResponse {
    pub tool_name: String,
    pub result: serde_json::Value,
    pub is_error: bool,
    pub error_message: Option<String>,
}

/// MCP tool result – either success content or an error
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpToolResultEnum {
    Content {
        content: String,
        content_type: String,
    },
    Error {
        message: String,
    },
}

impl McpToolResultEnum {
    pub fn into_response(self, tool_name: &str) -> McpToolCallResponse {
        match self {
            McpToolResultEnum::Content {
                content,
                content_type: _,
            } => McpToolCallResponse {
                tool_name: tool_name.to_string(),
                result: serde_json::json!({ "content": content }),
                is_error: false,
                error_message: None,
            },
            McpToolResultEnum::Error { message } => McpToolCallResponse {
                tool_name: tool_name.to_string(),
                result: serde_json::Value::Null,
                is_error: true,
                error_message: Some(message),
            },
        }
    }
}

/// MCP resource read request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct McpResourceReadRequest {
    pub uri: String,
}

/// MCP resource read response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct McpResourceReadResponse {
    pub uri: String,
    pub content: String,
    pub mime_type: Option<String>,
    pub is_error: bool,
    pub error_message: Option<String>,
}

/// List MCP audit events query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListMcpAuditEventsQuery {
    #[serde(default)]
    pub organization_id: Option<Id>,
    #[serde(default)]
    pub project_id: Option<Id>,
    #[serde(default)]
    pub user_id: Option<Id>,
    #[serde(default)]
    pub operation: Option<String>,
    #[serde(default)]
    pub start_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub end_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

/// MCP operation types
pub mod operation_types {
    pub const TOOL_CALL: &str = "tool.call";
    pub const TOOL_LIST: &str = "tool.list";
    pub const RESOURCE_READ: &str = "resource.read";
    pub const RESOURCE_LIST: &str = "resource.list";
    pub const SERVER_INFO: &str = "server.info";
}

/// MCP request (alias for McpToolCallRequest for API compatibility)
pub type McpRequest = McpToolCallRequest;

/// MCP response (alias for McpToolCallResponse for API compatibility)
pub type McpResponse = McpToolCallResponse;

/// MCP tool result used in biz layer
pub type McpToolResult = McpToolResultEnum;
