//! CMS Error Types
//!
//! This crate provides a single AppError enum that represents all possible
//! errors in the CMS system, following pattern of a single
//! error type with one conversion point to HTTP responses.
//!
//! The error variants map directly to CMS's existing closed ErrorCode union:
//! - http:* - HTTP-level errors
//! - auth:* - Authentication errors
//! - mcp:* - MCP server errors
//! - validation:* - Input validation errors
//! - database:* - Database errors
//! - storage:* - Object storage errors
//! - search:* - Search errors
//! - ai:* - AI provider errors
//! - import:* - Import errors
//! - provider:* - Third-party provider errors
//! - usage:* - Usage/billing errors
//! - entitlement:* - Entitlement errors
//! - addon:* - Addon errors
//! - integration:* - Integration errors

use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::json;
use thiserror::Error;

/// The root error type for the CMS application.
///
/// All errors in the system should be converted to this type.
/// Handlers return Result<T, AppError> and the IntoResponse impl
/// converts them to appropriate HTTP responses.
#[derive(Debug, Error)]
pub enum AppError {
    // === HTTP Errors ===
    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Method Not Allowed")]
    MethodNotAllowed,

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Too Many Requests")]
    TooManyRequests,

    #[error("Payload Too Large")]
    PayloadTooLarge,

    // === Authentication Errors ===
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Invalid or expired session")]
    InvalidSession,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Session required")]
    SessionRequired,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Token expired")]
    TokenExpired,

    // === Authorization Errors ===
    #[error("Insufficient role: {0}")]
    InsufficientRole(String),

    #[error("Access denied: {0}")]
    AccessDenied(String),

    #[error("Resource access forbidden")]
    ResourceAccessForbidden,

    // === Validation Errors ===
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    // === Database Errors ===
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Database connection failed")]
    DatabaseConnectionFailed,

    #[error("Transaction failed")]
    TransactionFailed,

    // === Storage Errors ===
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Storage not configured")]
    StorageNotConfigured,

    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    // === Search Errors ===
    #[error("Search unavailable: {0}")]
    SearchUnavailable(String),

    #[error("Search error: {0}")]
    SearchError(String),

    #[error("Indexing error: {0}")]
    IndexingError(String),

    // === AI/Provider Errors ===
    #[error("AI provider error: {0}")]
    AiProvider(String),

    #[error("AI provider unavailable")]
    AiProviderUnavailable,

    #[error("Model not found")]
    ModelNotFound,

    // === Import Errors ===
    #[error("Invalid import document: {0}")]
    InvalidImportDocument(String),

    #[error("Import failed: {0}")]
    ImportFailed(String),

    #[error("Git operation failed: {0}")]
    GitOperationFailed(String),

    // === Usage/Entitlement Errors ===
    #[error("Entitlement disabled: {0}")]
    EntitlementDisabled(String),

    #[error("Usage limit exceeded: {0}")]
    UsageLimitExceeded(String),

    #[error("Feature not available on plan")]
    FeatureNotAvailable,

    // === MCP Errors ===
    #[error("MCP disabled")]
    McpDisabled,

    #[error("MCP error: {0}")]
    McpError(String),

    // === Integration Errors ===
    #[error("Integration error: {0}")]
    IntegrationError(String),

    #[error("Webhook delivery failed")]
    WebhookDeliveryFailed,

    // === Addon Errors ===
    #[error("Addon error: {0}")]
    AddonError(String),

    // === Provider Errors ===
    #[error("Provider error: {0}")]
    ProviderError(String),

    // === Internal Errors ===
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),

    // === Custom Error with Status Code ===
    #[error("{message}")]
    Custom { status: StatusCode, message: String },
}

/// Error response structure for JSON API responses
#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorDetails,
}

#[derive(Serialize)]
struct ErrorDetails {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl AppError {
    /// Create a new custom error with a specific status code
    pub fn custom(status: StatusCode, message: impl Into<String>) -> Self {
        AppError::Custom {
            status,
            message: message.into(),
        }
    }

    /// Get the appropriate HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            // HTTP Errors
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            AppError::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,

            // Authentication Errors
            AppError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            AppError::InvalidSession => StatusCode::UNAUTHORIZED,
            AppError::InvalidApiKey => StatusCode::UNAUTHORIZED,
            AppError::SessionRequired => StatusCode::UNAUTHORIZED,
            AppError::InvalidToken(_) => StatusCode::UNAUTHORIZED,
            AppError::TokenExpired => StatusCode::UNAUTHORIZED,

            // Authorization Errors
            AppError::InsufficientRole(_) => StatusCode::FORBIDDEN,
            AppError::AccessDenied(_) => StatusCode::FORBIDDEN,
            AppError::ResourceAccessForbidden => StatusCode::FORBIDDEN,

            // Validation Errors
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,

            // Database Errors
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::DatabaseConnectionFailed => StatusCode::SERVICE_UNAVAILABLE,
            AppError::TransactionFailed => StatusCode::INTERNAL_SERVER_ERROR,

            // Storage Errors
            AppError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::StorageNotConfigured => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ObjectNotFound(_) => StatusCode::NOT_FOUND,

            // Search Errors
            AppError::SearchUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::SearchError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::IndexingError(_) => StatusCode::INTERNAL_SERVER_ERROR,

            // AI/Provider Errors
            AppError::AiProvider(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::AiProviderUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            AppError::ModelNotFound => StatusCode::NOT_FOUND,

            // Import Errors
            AppError::InvalidImportDocument(_) => StatusCode::BAD_REQUEST,
            AppError::ImportFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::GitOperationFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,

            // Usage/Entitlement Errors
            AppError::EntitlementDisabled(_) => StatusCode::FORBIDDEN,
            AppError::UsageLimitExceeded(_) => StatusCode::PAYMENT_REQUIRED,
            AppError::FeatureNotAvailable => StatusCode::FORBIDDEN,

            // MCP Errors
            AppError::McpDisabled => StatusCode::SERVICE_UNAVAILABLE,
            AppError::McpError(_) => StatusCode::INTERNAL_SERVER_ERROR,

            // Integration Errors
            AppError::IntegrationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::WebhookDeliveryFailed => StatusCode::INTERNAL_SERVER_ERROR,

            // Addon Errors
            AppError::AddonError(_) => StatusCode::INTERNAL_SERVER_ERROR,

            // Provider Errors
            AppError::ProviderError(_) => StatusCode::INTERNAL_SERVER_ERROR,

            // Internal Errors
            AppError::Serialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,

            // Custom Error
            AppError::Custom { status, .. } => *status,
        }
    }

    /// Get the error code string for this error
    pub fn error_code(&self) -> String {
        match self {
            AppError::BadRequest(_) => "http:bad_request".to_string(),
            AppError::Unauthorized => "http:unauthorized".to_string(),
            AppError::Forbidden => "http:forbidden".to_string(),
            AppError::NotFound(_) => "http:not_found".to_string(),
            AppError::MethodNotAllowed => "http:method_not_allowed".to_string(),
            AppError::Conflict(_) => "http:conflict".to_string(),
            AppError::TooManyRequests => "http:too_many_requests".to_string(),
            AppError::PayloadTooLarge => "http:payload_too_large".to_string(),

            AppError::InvalidCredentials => "auth:invalid_credentials".to_string(),
            AppError::InvalidSession => "auth:invalid_session".to_string(),
            AppError::InvalidApiKey => "auth:invalid_api_key".to_string(),
            AppError::SessionRequired => "auth:session_required".to_string(),
            AppError::InvalidToken(_) => "auth:invalid_token".to_string(),
            AppError::TokenExpired => "auth:token_expired".to_string(),

            AppError::InsufficientRole(_) => "auth:insufficient_role".to_string(),
            AppError::AccessDenied(_) => "auth:access_denied".to_string(),
            AppError::ResourceAccessForbidden => "auth:resource_access_forbidden".to_string(),

            AppError::Validation(_) => "validation:failed".to_string(),
            AppError::InvalidInput(_) => "validation:invalid_input".to_string(),

            AppError::Database(_) => "database:error".to_string(),
            AppError::DatabaseConnectionFailed => "database:connection_failed".to_string(),
            AppError::TransactionFailed => "database:transaction_failed".to_string(),

            AppError::Storage(_) => "storage:error".to_string(),
            AppError::StorageNotConfigured => "storage:not_configured".to_string(),
            AppError::ObjectNotFound(_) => "storage:object_not_found".to_string(),

            AppError::SearchUnavailable(_) => "search:unavailable".to_string(),
            AppError::SearchError(_) => "search:error".to_string(),
            AppError::IndexingError(_) => "search:indexing_error".to_string(),

            AppError::AiProvider(_) => "ai:provider_error".to_string(),
            AppError::AiProviderUnavailable => "ai:provider_unavailable".to_string(),
            AppError::ModelNotFound => "ai:model_not_found".to_string(),

            AppError::InvalidImportDocument(_) => "import:invalid_document".to_string(),
            AppError::ImportFailed(_) => "import:failed".to_string(),
            AppError::GitOperationFailed(_) => "import:git_operation_failed".to_string(),

            AppError::EntitlementDisabled(_) => "entitlement:disabled".to_string(),
            AppError::UsageLimitExceeded(_) => "usage:limit_exceeded".to_string(),
            AppError::FeatureNotAvailable => "entitlement:feature_not_available".to_string(),

            AppError::McpDisabled => "mcp:disabled".to_string(),
            AppError::McpError(_) => "mcp:error".to_string(),

            AppError::IntegrationError(_) => "integration:error".to_string(),
            AppError::WebhookDeliveryFailed => "integration:webhook_delivery_failed".to_string(),

            AppError::AddonError(_) => "addon:error".to_string(),

            AppError::ProviderError(_) => "provider:error".to_string(),

            AppError::Serialization(_) => "internal:serialization_error".to_string(),
            AppError::Internal(_) => "internal:error".to_string(),

            AppError::Custom { .. } => "internal:error".to_string(),
        }
    }

    /// Get additional details for the error response
    pub fn details(&self) -> Option<serde_json::Value> {
        match self {
            AppError::NotFound(path) => Some(json!({ "path": path })),
            AppError::Conflict(details) => Some(json!({ "details": details })),
            AppError::Validation(details) => Some(json!({ "details": details })),
            AppError::InvalidInput(details) => Some(json!({ "details": details })),
            AppError::Storage(details) => Some(json!({ "details": details })),
            AppError::SearchError(details) => Some(json!({ "details": details })),
            AppError::AiProvider(details) => Some(json!({ "details": details })),
            AppError::InvalidImportDocument(details) => Some(json!({ "details": details })),
            AppError::ImportFailed(details) => Some(json!({ "details": details })),
            AppError::GitOperationFailed(details) => Some(json!({ "details": details })),
            AppError::EntitlementDisabled(details) => Some(json!({ "details": details })),
            AppError::UsageLimitExceeded(details) => Some(json!({ "details": details })),
            AppError::McpError(details) => Some(json!({ "details": details })),
            AppError::IntegrationError(details) => Some(json!({ "details": details })),
            AppError::AddonError(details) => Some(json!({ "details": details })),
            AppError::ProviderError(details) => Some(json!({ "details": details })),
            AppError::InvalidToken(details) => Some(json!({ "details": details })),
            AppError::InsufficientRole(details) => Some(json!({ "details": details })),
            AppError::AccessDenied(details) => Some(json!({ "details": details })),
            AppError::Serialization(err) => Some(json!({ "details": err.to_string() })),
            AppError::Custom { message, .. } => Some(json!({ "message": message })),
            _ => None,
        }
    }
}

// Implement IntoResponse for AppError to convert to HTTP responses
impl IntoResponse for AppError {
    fn into_response(self) -> Response<Body> {
        let status = self.status_code();
        let error_code = self.error_code();
        let message = self.to_string();
        let details = self.details();

        if status.is_server_error() {
            tracing::error!(
                status = %status,
                error_code = %error_code,
                error = %message,
                details = ?details,
                "AppError Server Error"
            );
        } else if status.is_client_error() {
            tracing::warn!(
                status = %status,
                error_code = %error_code,
                error = %message,
                "AppError Client Error"
            );
        }

        let body = ErrorResponse {
            error: ErrorDetails {
                code: error_code,
                message,
                details,
            },
        };

        let json_body = serde_json::to_string(&body).unwrap_or_else(|_| {
            json!({
                "error": {
                    "code": "internal:serialization_error",
                    "message": "Failed to serialize error response"
                }
            })
            .to_string()
        });

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Body::from(json_body))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Internal Server Error"))
                    .unwrap()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            AppError::NotFound("test".to_string()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::Unauthorized.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(AppError::Forbidden.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(
            AppError::BadRequest("test".to_string()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::Internal(anyhow::anyhow!("test")).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(AppError::InvalidApiKey.error_code(), "auth:invalid_api_key");
        assert_eq!(
            AppError::UsageLimitExceeded("test".to_string()).error_code(),
            "usage:limit_exceeded"
        );
        assert_eq!(
            AppError::SearchUnavailable("test".to_string()).error_code(),
            "search:unavailable"
        );
    }

    #[tokio::test]
    async fn test_into_response() {
        let error = AppError::NotFound("page not found".to_string());
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("http:not_found"));
        assert!(body_str.contains("page not found"));
    }
}
