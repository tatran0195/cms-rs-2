//! OpenAPI entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, Timestamp};

/// OpenAPI document entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OpenApiDocument {
    pub id: Id,
    pub project_id: Id,
    pub name: String,
    pub url: String,
    pub content: Option<String>,
    pub parsed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// OpenAPI document response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OpenApiDocumentResponse {
    pub id: Id,
    pub project_id: Id,
    pub name: String,
    pub url: String,
    pub parsed_at: Option<DateTime<Utc>>,
    pub has_error: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<OpenApiDocument> for OpenApiDocumentResponse {
    fn from(doc: OpenApiDocument) -> Self {
        Self {
            id: doc.id,
            project_id: doc.project_id,
            name: doc.name,
            url: doc.url,
            parsed_at: doc.parsed_at,
            has_error: doc.error_message.is_some(),
            error_message: doc.error_message,
            created_at: doc.created_at,
            updated_at: doc.updated_at,
        }
    }
}

/// Create OpenAPI document request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateOpenApiDocumentRequest {
    pub project_id: Id,
    pub name: String,
    pub url: String,
}

/// Update OpenAPI document request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateOpenApiDocumentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Parse OpenAPI document request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ParseOpenApiDocumentRequest {
    pub id: Id,
}

/// OpenAPI parsing result
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OpenApiParsingResult {
    pub document_id: Id,
    pub parsed_successfully: bool,
    pub paths_count: Option<i32>,
    pub error_message: Option<String>,
}

/// List OpenAPI documents query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListOpenApiDocumentsQuery {
    #[serde(default)]
    pub project_id: Option<Id>,
    #[serde(default)]
    pub has_error: Option<bool>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

/// OpenAPI path info (simplified for responses)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OpenApiPathInfo {
    pub path: String,
    pub method: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

/// OpenAPI document with paths
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OpenApiDocumentWithPaths {
    #[serde(flatten)]
    pub document: OpenApiDocumentResponse,
    pub paths: Vec<OpenApiPathInfo>,
}
