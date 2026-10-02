//! Authentication and Authorization entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::common::{Id, Timestamp};

/// User entity (simplified from Prisma User model)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct User {
    pub id: Id,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User create request
#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
pub struct CreateUserRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: Option<String>,
    #[serde(default)]
    #[validate(length(max = 100, message = "Name must be at most 100 characters"))]
    pub name: Option<String>,
    #[serde(default)]
    #[validate(url(message = "Invalid image URL"))]
    pub image: Option<String>,
}

/// Register request (alias for CreateUserRequest)
pub type RegisterRequest = CreateUserRequest;

/// User update request
#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 100, message = "Name must be at most 100 characters"))]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url(message = "Invalid image URL"))]
    pub image: Option<String>,
}

/// User response (with sensitive fields removed)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserResponse {
    pub id: Id,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            image: user.image,
            email_verified: user.email_verified,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

/// Session entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Session {
    pub id: Id,
    pub user_id: Id,
    pub session_token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Session response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SessionResponse {
    pub id: Id,
    pub user_id: Id,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Account entity (for OAuth providers)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Account {
    pub id: Id,
    pub user_id: Id,
    pub provider: String,
    pub provider_account_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Verification token entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VerificationToken {
    pub id: Id,
    pub identifier: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// API Key entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApiKey {
    pub id: Id,
    pub user_id: Id,
    pub name: String,
    pub key: String, // This is the hashed key, not the plaintext
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<DateTime<Utc>>,
}

/// API Key create request
#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
pub struct CreateApiKeyRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,
}

/// API Key response (without the actual key)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApiKeyResponse {
    pub id: Id,
    pub user_id: Id,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<DateTime<Utc>>,
}

impl From<ApiKey> for ApiKeyResponse {
    fn from(key: ApiKey) -> Self {
        Self {
            id: key.id,
            user_id: key.user_id,
            name: key.name,
            created_at: key.created_at,
            updated_at: key.updated_at,
            last_used_at: key.last_used_at,
        }
    }
}

/// API Key with secret (only returned on creation)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApiKeyWithSecretResponse {
    #[serde(flatten)]
    pub key: ApiKeyResponse,
    pub secret: String, // The plaintext key, only shown once
}

/// Login request
#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

/// Login response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LoginResponse {
    pub user: UserResponse,
    pub session: SessionResponse,
}

/// OAuth login request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct OAuthLoginRequest {
    pub provider: String,
    pub code: String,
    pub redirect_uri: String,
}

/// Token refresh request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Token refresh response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

/// Authenticated user information (from session or API key)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuthenticatedUser {
    pub user_id: Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<Id>,
    pub is_api_key: bool,
}

/// Reader JWT claims
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReaderJwtClaims {
    pub reader_id: Id,
    pub project_id: Id,
    pub audience_id: Id,
    pub exp: i64,
    pub nbf: i64,
    pub jti: String, // JWT ID for replay protection
}

/// JWT access provider entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct JwtAccessProvider {
    pub id: Id,
    pub name: String,
    pub issuer: String,
    pub audience: String,
    pub secret: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// JWT replay tracking entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct JwtReplay {
    pub id: Id,
    pub jwt_id: String,
    pub provider_id: Id,
    pub used_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_user_response_conversion() {
        let user = User {
            id: "user-1".to_string(),
            email: "test@example.com".to_string(),
            name: Some("Test User".to_string()),
            image: Some("https://example.com/avatar.png".to_string()),
            email_verified: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: UserResponse = user.into();
        assert_eq!(response.id, "user-1");
        assert_eq!(response.email, "test@example.com");
        assert_eq!(response.name, Some("Test User".to_string()));
    }
}
