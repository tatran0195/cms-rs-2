//! MCP Server Business Logic
//!
//! This module contains business logic for the Model Context Protocol (MCP) server.
//! MCP allows AI agents to query CMS's documentation programmatically.

use cms_db::{
    branch::BranchQueries, mcp::McpAuditEventQueries, page::PageQueries, project::ProjectQueries,
};
use cms_entity::{
    common::MemberRole,
    mcp::{McpCapabilities, McpPrompt, McpRequest, McpResource, McpResponse, McpTool},
};

use crate::{AppError, BizContext};

/// MCP service
pub struct McpService;

impl McpService {
    /// Get MCP capabilities
    pub async fn get_capabilities(
        ctx: &BizContext,
        user_id: Option<&str>,
        _org_id: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<McpCapabilities, AppError> {
        if let Some(pid) = project_id {
            let project = ProjectQueries::get_by_id(&ctx.pool, pid)
                .await?
                .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;
            if let Some(uid) = user_id {
                ctx.authz
                    .require_project_role(uid, pid, MemberRole::Viewer)
                    .await?;
            } else if !project.is_public {
                return Err(AppError::Forbidden);
            }
        }

        Ok(McpCapabilities {
            tools: vec![
                McpTool {
                    name: "search".to_string(),
                    description: "Search project documentation".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": { "type": "string" },
                            "project_id": { "type": "string" },
                            "limit": { "type": "integer", "default": 10 }
                        },
                        "required": ["query", "project_id"]
                    }),
                },
                McpTool {
                    name: "get_page".to_string(),
                    description: "Get a specific page by path".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "project_id": { "type": "string" },
                            "path": { "type": "string" }
                        },
                        "required": ["project_id", "path"]
                    }),
                },
                McpTool {
                    name: "list_pages".to_string(),
                    description: "List all pages in a project".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "project_id": { "type": "string" },
                            "branch_id": { "type": "string" },
                            "limit": { "type": "integer", "default": 100 }
                        },
                        "required": ["project_id"]
                    }),
                },
                McpTool {
                    name: "get_project".to_string(),
                    description: "Get project information".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "project_id": { "type": "string" }
                        },
                        "required": ["project_id"]
                    }),
                },
            ],
            prompts: vec![],
            resources: vec![],
        })
    }

    /// Execute an MCP tool
    pub async fn execute_tool(
        ctx: &BizContext,
        user_id: Option<&str>,
        org_id: Option<&str>,
        request: McpRequest,
    ) -> Result<McpResponse, AppError> {
        if let Some(uid) = user_id {
            let _ = McpAuditEventQueries::create(
                &ctx.pool,
                org_id,
                None,
                Some(uid),
                &request.tool_name,
                None,
                None,
                None,
            )
            .await;
        }

        match request.tool_name.as_str() {
            "search" => {
                Self::execute_search(ctx, &request.tool_name, user_id, &request.arguments).await
            }
            "get_page" => {
                Self::execute_get_page(ctx, &request.tool_name, user_id, &request.arguments).await
            }
            "list_pages" => {
                Self::execute_list_pages(ctx, &request.tool_name, user_id, &request.arguments).await
            }
            "get_project" => {
                Self::execute_get_project(ctx, &request.tool_name, user_id, &request.arguments)
                    .await
            }
            _ => Ok(McpResponse {
                tool_name: request.tool_name.clone(),
                result: serde_json::Value::Null,
                is_error: true,
                error_message: Some(format!("Unknown MCP tool: {}", request.tool_name)),
            }),
        }
    }

    async fn execute_search(
        ctx: &BizContext,
        tool_name: &str,
        user_id: Option<&str>,
        arguments: &serde_json::Value,
    ) -> Result<McpResponse, AppError> {
        let query = arguments
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::InvalidInput("Missing query parameter".to_string()))?;

        let project_id = arguments
            .get("project_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::InvalidInput("Missing project_id parameter".to_string()))?;

        let limit = arguments
            .get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(10);

        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        if let Some(uid) = user_id {
            ctx.authz
                .require_project_role(uid, project_id, MemberRole::Viewer)
                .await?;
        } else if !project.is_public {
            return Err(AppError::Forbidden);
        }

        // Find branch
        let default_branch = BranchQueries::get_default(&ctx.pool, project_id).await?;
        let branch_id = default_branch.map(|b| b.id).unwrap_or_default();

        let pages = if !branch_id.is_empty() {
            PageQueries::get_by_project_and_branch(
                &ctx.pool,
                project_id,
                &branch_id,
                None,
                Some(true),
                Some(query),
                Some(limit),
                None,
            )
            .await?
        } else {
            vec![]
        };

        let mut content = format!("# Search Results for \"{}\"\n\n", query);
        if pages.is_empty() {
            content.push_str("No matching documentation pages found.\n");
        } else {
            for page in pages {
                content.push_str(&format!("## [{}]({})\n", page.title, page.path));
                if let Some(desc) = page.description {
                    content.push_str(&format!("{}\n\n", desc));
                } else {
                    content.push('\n');
                }
            }
        }

        Ok(McpResponse {
            tool_name: tool_name.to_string(),
            result: serde_json::json!({
                "content": content,
                "content_type": "text/markdown"
            }),
            is_error: false,
            error_message: None,
        })
    }

    async fn execute_get_page(
        ctx: &BizContext,
        tool_name: &str,
        user_id: Option<&str>,
        arguments: &serde_json::Value,
    ) -> Result<McpResponse, AppError> {
        let project_id = arguments
            .get("project_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::InvalidInput("Missing project_id parameter".to_string()))?;
        let path = arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::InvalidInput("Missing path parameter".to_string()))?;

        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        if let Some(uid) = user_id {
            ctx.authz
                .require_project_role(uid, project_id, MemberRole::Viewer)
                .await?;
        } else if !project.is_public {
            return Err(AppError::Forbidden);
        }

        let default_branch = BranchQueries::get_default(&ctx.pool, project_id).await?;
        let branch_id = default_branch.map(|b| b.id).unwrap_or_default();

        let page = PageQueries::get_by_path(&ctx.pool, project_id, &branch_id, path)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Page not found at path: {}", path)))?;

        let content = format!(
            "# {}\n\n*Path: {}*\n\n{}\n\n---\n*Last Updated: {}*",
            page.title, page.path, page.content, page.updated_at
        );

        Ok(McpResponse {
            tool_name: tool_name.to_string(),
            result: serde_json::json!({
                "content": content,
                "content_type": "text/markdown"
            }),
            is_error: false,
            error_message: None,
        })
    }

    /// Execute list_pages MCP tool
    async fn execute_list_pages(
        ctx: &BizContext,
        tool_name: &str,
        user_id: Option<&str>,
        args: &serde_json::Value,
    ) -> Result<McpResponse, AppError> {
        let project_id = args
            .get("project_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::BadRequest("project_id argument is required".to_string()))?;

        let branch_id = args.get("branch_id").and_then(|v| v.as_str());
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(50);

        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        if let Some(uid) = user_id {
            ctx.authz
                .require_project_role(uid, project_id, MemberRole::Viewer)
                .await?;
        } else if !project.is_public {
            return Err(AppError::Forbidden);
        }

        let pages: Vec<(String, String)> = if let Some(bid) = branch_id {
            PageQueries::get_by_project_and_branch(
                &ctx.pool,
                project_id,
                bid,
                None,
                None,
                None,
                Some(limit),
                None,
            )
            .await?
            .into_iter()
            .map(|p| (p.title, p.path))
            .collect()
        } else {
            PageQueries::get_by_project(&ctx.pool, project_id)
                .await?
                .into_iter()
                .map(|p| (p.title, p.path))
                .collect()
        };

        let mut content = String::from("# Pages\n\n");
        if pages.is_empty() {
            content.push_str("No pages found in this project.\n");
        } else {
            for (title, path) in pages {
                content.push_str(&format!("- [{}]({})\n", title, path));
            }
        }

        Ok(McpResponse {
            tool_name: tool_name.to_string(),
            result: serde_json::json!({
                "content": content,
                "content_type": "text/markdown"
            }),
            is_error: false,
            error_message: None,
        })
    }

    async fn execute_get_project(
        ctx: &BizContext,
        tool_name: &str,
        user_id: Option<&str>,
        arguments: &serde_json::Value,
    ) -> Result<McpResponse, AppError> {
        let project_id = arguments
            .get("project_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::InvalidInput("Missing project_id parameter".to_string()))?;

        let project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        if let Some(uid) = user_id {
            ctx.authz
                .require_project_role(uid, project_id, MemberRole::Viewer)
                .await?;
        } else if !project.is_public {
            return Err(AppError::Forbidden);
        }

        let content = format!(
            "# {}\n\n{}\n\n---\n\n*Created: {}*\n*Updated: {}*",
            project.name,
            project.description.as_deref().unwrap_or(""),
            project.created_at,
            project.updated_at
        );

        Ok(McpResponse {
            tool_name: tool_name.to_string(),
            result: serde_json::json!({ "content": content, "content_type": "text/markdown" }),
            is_error: false,
            error_message: None,
        })
    }

    /// List audit events
    #[allow(clippy::too_many_arguments)]
    pub async fn list_audit_events(
        ctx: &BizContext,
        _user_id: &str,
        organization_id: Option<&str>,
        _project_id: Option<&str>,
        _user_id_filter: Option<&str>,
        _operation: Option<&str>,
        _start_date: Option<chrono::DateTime<chrono::Utc>>,
        _end_date: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<
        cms_entity::common::PaginatedResponse<cms_entity::mcp::McpAuditEventResponse>,
        AppError,
    > {
        let events = McpAuditEventQueries::get_by_organization(
            &ctx.pool,
            organization_id.unwrap_or(""),
            limit,
            offset,
        )
        .await?;
        let total = events.len() as u64;
        Ok(cms_entity::common::PaginatedResponse::new(
            events.into_iter().map(|e| e.into()).collect(),
            total,
            1,
            20,
        ))
    }

    /// Get server info
    pub async fn get_server_info(_ctx: &BizContext) -> Result<serde_json::Value, AppError> {
        Ok(serde_json::json!({
            "name": "cms-mcp",
            "version": "1.0.0",
            "protocol_version": "2024-11-05",
        }))
    }

    /// List tools
    pub async fn list_tools(ctx: &BizContext) -> Result<serde_json::Value, AppError> {
        let caps = Self::get_capabilities(ctx, None, None, None).await?;
        Ok(serde_json::json!(caps.tools))
    }

    /// Call tool
    pub async fn call_tool(
        ctx: &BizContext,
        user_id: &str,
        tool_name: &str,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let response = Self::execute_tool(
            ctx,
            Some(user_id),
            None,
            McpRequest {
                tool_name: tool_name.to_string(),
                arguments: request,
            },
        )
        .await?;
        Ok(serde_json::json!(response))
    }

    /// List resources
    pub async fn list_resources(
        ctx: &BizContext,
        user_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        let projects = ProjectQueries::list_by_user(&ctx.pool, user_id)
            .await
            .unwrap_or_default();
        let mut resources = Vec::new();

        for project in projects {
            let pages = PageQueries::get_by_project(&ctx.pool, &project.id)
                .await
                .unwrap_or_default();
            for page in pages {
                resources.push(serde_json::json!({
                    "uri": format!("cms://projects/{}/pages{}", project.id, page.path),
                    "name": page.title,
                    "mimeType": "text/markdown",
                    "description": format!("Page '{}' in project '{}'", page.title, project.name)
                }));
            }
        }

        Ok(serde_json::json!(resources))
    }

    /// Read resource
    pub async fn read_resource(
        ctx: &BizContext,
        user_id: &str,
        uri: &str,
    ) -> Result<serde_json::Value, AppError> {
        // Expected URI format: cms://projects/{project_id}/pages/{path}
        let stripped = uri.strip_prefix("cms://projects/").ok_or_else(|| {
            AppError::InvalidInput(
                "Invalid resource URI scheme. Expected cms://projects/{project_id}/pages/{path}"
                    .to_string(),
            )
        })?;

        let parts: Vec<&str> = stripped.splitn(2, "/pages").collect();
        if parts.len() != 2 {
            return Err(AppError::InvalidInput(
                "Invalid resource URI format".to_string(),
            ));
        }

        let project_id = parts[0];
        let page_path = parts[1];

        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;

        let default_branch = BranchQueries::get_default(&ctx.pool, project_id).await?;
        let branch_id = default_branch.map(|b| b.id).unwrap_or_default();

        let page = PageQueries::get_by_path(&ctx.pool, project_id, &branch_id, page_path)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Page not found at URI: {}", uri)))?;

        Ok(serde_json::json!({
            "uri": uri,
            "contents": [{
                "uri": uri,
                "mimeType": "text/markdown",
                "text": page.content
            }]
        }))
    }
}
