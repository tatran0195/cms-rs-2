//! OpenAPI Business Logic
//!
//! This module contains business logic for OpenAPI document management.

use chrono::Utc;
use cms_authz::Authz;
use cms_db::openapi::OpenApiDocumentQueries;
use cms_entity::{
    common::Id,
    openapi::{
        CreateOpenApiDocumentRequest, OpenApiDocument, OpenApiDocumentResponse,
        OpenApiParsingResult, ParseOpenApiDocumentRequest, UpdateOpenApiDocumentRequest,
    },
};

use crate::{AppError, BizContext};

/// OpenAPI service
pub struct OpenApiService;

impl OpenApiService {
    /// Create a new OpenAPI document
    pub async fn create_document(
        ctx: &BizContext,
        user_id: &str,
        request: CreateOpenApiDocumentRequest,
    ) -> Result<OpenApiDocumentResponse, AppError> {
        // Check if user has access to the project
        ctx.authz
            .require_project_access(user_id, &request.project_id)
            .await?;

        // Check if a document with this URL already exists for this project
        let existing =
            OpenApiDocumentQueries::get_by_url(&ctx.pool, &request.url, &request.project_id)
                .await?;

        if existing.is_some() {
            return Err(AppError::Conflict(
                "An OpenAPI document with this URL already exists for this project".to_string(),
            ));
        }

        let document = OpenApiDocumentQueries::create(
            &ctx.pool,
            &request.project_id,
            &request.name,
            &request.url,
        )
        .await?;

        Ok(document.into())
    }

    /// Get OpenAPI document by ID
    pub async fn get_document(
        ctx: &BizContext,
        user_id: &str,
        document_id: &str,
    ) -> Result<OpenApiDocumentResponse, AppError> {
        let document = OpenApiDocumentQueries::get_by_id(&ctx.pool, document_id)
            .await?
            .ok_or_else(|| AppError::NotFound("OpenAPI document not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_access(user_id, &document.project_id)
            .await?;

        Ok(document.into())
    }

    /// List OpenAPI documents for a project
    pub async fn list_documents(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<Vec<OpenApiDocumentResponse>, AppError> {
        // Check if user has access to the project
        ctx.authz
            .require_project_access(user_id, project_id)
            .await?;

        let documents = OpenApiDocumentQueries::get_by_project(&ctx.pool, project_id).await?;

        Ok(documents.into_iter().map(|d| d.into()).collect())
    }

    /// Update an OpenAPI document
    pub async fn update_document(
        ctx: &BizContext,
        user_id: &str,
        document_id: &str,
        request: UpdateOpenApiDocumentRequest,
    ) -> Result<OpenApiDocumentResponse, AppError> {
        let document = OpenApiDocumentQueries::get_by_id(&ctx.pool, document_id)
            .await?
            .ok_or_else(|| AppError::NotFound("OpenAPI document not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_access(user_id, &document.project_id)
            .await?;

        let updated = OpenApiDocumentQueries::update(
            &ctx.pool,
            document_id,
            request.name.as_deref(),
            request.url.as_deref(),
        )
        .await?;

        Ok(updated.into())
    }

    /// Delete an OpenAPI document
    pub async fn delete_document(
        ctx: &BizContext,
        user_id: &str,
        document_id: &str,
    ) -> Result<bool, AppError> {
        let document = OpenApiDocumentQueries::get_by_id(&ctx.pool, document_id)
            .await?
            .ok_or_else(|| AppError::NotFound("OpenAPI document not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_access(user_id, &document.project_id)
            .await?;

        OpenApiDocumentQueries::delete(&ctx.pool, document_id).await
    }

    /// Parse an OpenAPI document (fetch and parse the content)
    pub async fn parse_document(
        ctx: &BizContext,
        user_id: &str,
        request: ParseOpenApiDocumentRequest,
    ) -> Result<OpenApiParsingResult, AppError> {
        let document = OpenApiDocumentQueries::get_by_id(&ctx.pool, &request.id)
            .await?
            .ok_or_else(|| AppError::NotFound("OpenAPI document not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_access(user_id, &document.project_id)
            .await?;

        // Fetch the OpenAPI document from the URL
        // Note: This is a placeholder - actual implementation would use reqwest or similar
        let content = match fetch_openapi_content(&document.url).await {
            Ok(content) => content,
            Err(e) => {
                let error_msg = format!("Failed to fetch OpenAPI document: {}", e);
                OpenApiDocumentQueries::update_error(&ctx.pool, &request.id, Some(&error_msg))
                    .await?;
                return Ok(OpenApiParsingResult {
                    document_id: request.id,
                    parsed_successfully: false,
                    paths_count: None,
                    error_message: Some(error_msg),
                });
            }
        };

        // Parse the OpenAPI content
        // Note: This is a placeholder - actual implementation would use openapi parser
        let paths_count = count_openapi_paths(&content);

        // Update the document with parsed content
        OpenApiDocumentQueries::update_parsed(
            &ctx.pool,
            &request.id,
            Some(&content),
            Some(Utc::now()),
            None,
        )
        .await?;

        Ok(OpenApiParsingResult {
            document_id: request.id,
            parsed_successfully: true,
            paths_count: Some(paths_count),
            error_message: None,
        })
    }

    /// Get OpenAPI document with paths
    pub async fn get_document_with_paths(
        ctx: &BizContext,
        user_id: &str,
        document_id: &str,
    ) -> Result<OpenApiDocumentResponse, AppError> {
        let document = OpenApiDocumentQueries::get_by_id(&ctx.pool, document_id)
            .await?
            .ok_or_else(|| AppError::NotFound("OpenAPI document not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_access(user_id, &document.project_id)
            .await?;

        Ok(document.into())
    }

    /// Get OpenAPI document content
    pub async fn get_document_content(
        ctx: &BizContext,
        user_id: &str,
        document_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        let document = OpenApiDocumentQueries::get_by_id(&ctx.pool, document_id)
            .await?
            .ok_or_else(|| AppError::NotFound("OpenAPI document not found".to_string()))?;

        ctx.authz
            .require_project_access(user_id, &document.project_id)
            .await?;

        if let Some(content_str) = document.content {
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content_str) {
                return Ok(json_val);
            }
        }

        // Return empty valid spec structure if not yet parsed
        Ok(
            serde_json::json!({ "openapi": "3.0.0", "info": { "title": document.name, "version": "1.0.0" }, "paths": {} }),
        )
    }
}

/// Fetch OpenAPI content from a URL via HTTP
async fn fetch_openapi_content(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .get(url)
        .header("User-Agent", "cms-CMS/1.0")
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error status: {}", response.status()));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if text.len() > 5 * 1024 * 1024 {
        return Err("OpenAPI document exceeds maximum size limit (5MB)".to_string());
    }

    Ok(text)
}

/// Parse OpenAPI content to count endpoints defined in `paths`
fn count_openapi_paths(content: &str) -> i32 {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(paths) = val.get("paths").and_then(|p| p.as_object()) {
            return paths.len() as i32;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_openapi_paths() {
        let json_spec = r#"{
            "openapi": "3.0.0",
            "info": { "title": "Test", "version": "1.0" },
            "paths": {
                "/users": {},
                "/users/{id}": {},
                "/posts": {}
            }
        }"#;

        assert_eq!(count_openapi_paths(json_spec), 3);
    }
}
