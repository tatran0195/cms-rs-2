//! CMS Authentication
//!
//! This crate provides authentication services for CMS, keeping identity
//! in-process (not delegating to an external service like GoTrue).
//!
//! It includes:
//! - Session management
//! - API key authentication
//! - Reader JWT authentication
//! - OAuth provider integration

pub mod api_key;
pub mod jwt;
pub mod oauth;
pub mod password;
pub mod session;

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
    RequestPartsExt,
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use cms_config::AuthConfig;
use cms_db::{auth::ApiKeyQueries, PgPool};
use cms_entity::auth::{ApiKey, ApiKeyResponse, ApiKeyWithSecretResponse, User};
use cms_error::AppError;
use jsonwebtoken::{DecodingKey, EncodingKey, Validation};
use serde::{Deserialize, Serialize};
pub use session::*;

/// Authentication service
#[derive(Clone)]
pub struct AuthService {
    config: AuthConfig,
    pool: PgPool,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl AuthService {
    /// Create a new AuthService with a database pool
    pub fn new(config: AuthConfig, pool: PgPool) -> Self {
        let encoding_key = EncodingKey::from_secret(config.session_secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.session_secret.as_bytes());

        Self {
            config,
            pool,
            encoding_key,
            decoding_key,
        }
    }

    /// Create with database pool (alias for new)
    pub fn with_pool(config: AuthConfig, pool: PgPool) -> Self {
        Self::new(config, pool)
    }

    /// Get the JWT validation
    pub fn jwt_validation(&self) -> Validation {
        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.set_issuer(&["cms".to_string()]);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = 5; // 5 seconds leeway
        validation
    }

    /// Get the encoding key
    pub fn encoding_key(&self) -> &EncodingKey {
        &self.encoding_key
    }

    /// Get the decoding key
    pub fn decoding_key(&self) -> &DecodingKey {
        &self.decoding_key
    }

    /// Get the configuration
    pub fn config(&self) -> &AuthConfig {
        &self.config
    }
}

/// Authenticated user extractor for Axum
///
/// This extractor validates session cookies and provides the authenticated user
/// to handlers.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub org_id: Option<String>,
}

/// API key principal extractor
#[derive(Debug, Clone)]
pub struct ApiKeyPrincipal {
    pub key_id: String,
    pub org_id: Option<String>,
}

/// Reader principal extractor (for reader-access JWTs)
#[derive(Debug, Clone)]
pub struct ReaderPrincipal {
    pub reader_id: String,
    pub project_id: String,
}

/// Extract authenticated user from request
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get the session cookie
        let jar = CookieJar::from_headers(&parts.headers);
        let session_cookie = jar
            .get("session")
            .map(|c| c.value().to_string())
            .ok_or(AppError::SessionRequired)?;

        // Get the AuthService from extensions
        let auth_service = parts
            .extensions
            .get::<Arc<AuthService>>()
            .cloned()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("AuthService not available")))?;

        // Validate the session
        let session =
            cms_db::auth::SessionQueries::get_by_token(&auth_service.pool, &session_cookie)
                .await
                .map_err(|_| AppError::InvalidSession)?
                .ok_or(AppError::InvalidSession)?;

        if session.expires_at < Utc::now() {
            return Err(AppError::TokenExpired);
        }

        // Get the user
        let user = cms_db::auth::UserQueries::get_by_id(&auth_service.pool, &session.user_id)
            .await
            .map_err(|_| AppError::InvalidSession)?
            .ok_or(AppError::InvalidSession)?;

        Ok(AuthenticatedUser {
            user_id: user.id,
            org_id: None,
        })
    }
}

/// Extract API key principal from request
impl<S> FromRequestParts<S> for ApiKeyPrincipal
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get the API key from Authorization header
        let auth_value = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::InvalidApiKey)?;

        let api_key = auth_value
            .strip_prefix("Bearer ")
            .ok_or(AppError::InvalidApiKey)?;

        // Get the AuthService from extensions
        let auth_service = parts
            .extensions
            .get::<Arc<AuthService>>()
            .cloned()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("AuthService not available")))?;

        // Look up the API key by hashed value
        let hashed = api_key::hash_key(api_key);
        let key = ApiKeyQueries::get_by_key(&auth_service.pool, &hashed)
            .await
            .map_err(|_| AppError::InvalidApiKey)?
            .ok_or(AppError::InvalidApiKey)?;

        // Update last used time (best-effort, don't fail on error)
        let _ = ApiKeyQueries::update_last_used(&auth_service.pool, &key.id).await;

        Ok(ApiKeyPrincipal {
            key_id: key.id,
            org_id: None,
        })
    }
}

use sha2::{Digest, Sha256};

/// Helper to hash API keys using SHA-256 (simpler and faster than Argon2 for key lookup)
pub fn hash_api_key(api_key: &str, prefix: &str) -> Result<String, AppError> {
    let combined = format!("{}-{}", prefix, api_key);
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    let result = hasher.finalize();
    Ok(format!("sha256:{}", hex::encode(result)))
}

/// Verify an API key against a hash
pub fn verify_api_key(api_key: &str, hash: &str, prefix: &str) -> Result<bool, AppError> {
    let expected_hash = hash_api_key(api_key, prefix)?;
    Ok(expected_hash == hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_api_key() {
        let prefix = "test_prefix";
        let api_key = "test_api_key_123";

        let hash = hash_api_key(api_key, prefix).unwrap();
        let is_valid = verify_api_key(api_key, &hash, prefix).unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_invalid_api_key() {
        let prefix = "test_prefix";
        let api_key = "test_api_key_123";
        let wrong_key = "wrong_key";

        let hash = hash_api_key(api_key, prefix).unwrap();
        let is_valid = verify_api_key(wrong_key, &hash, prefix).unwrap();

        assert!(!is_valid);
    }
}
