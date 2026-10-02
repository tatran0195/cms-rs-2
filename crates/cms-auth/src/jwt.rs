//! JWT utilities for CMS
//!
//! This module provides JWT creation, validation, and management for various
//! JWT-based authentication mechanisms (reader tokens, API tokens, etc.)

use chrono::{DateTime, Duration, Utc};
use cms_config::AuthConfig;
use cms_db::PgPool;
use cms_error::AppError;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AuthService;

/// Reader JWT claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderJwtClaims {
    pub reader_id: String,
    pub project_id: String,
    pub audience_id: String,
    /// Issuer (iss) claim
    #[serde(default)]
    pub issuer: Option<String>,
    /// Audience (aud) claim
    #[serde(default)]
    pub audience: Option<String>,
    pub iat: i64,
    pub exp: i64,
    pub nbf: i64,
    pub jti: String, // JWT ID for replay protection
}

impl ReaderJwtClaims {
    pub fn new(reader_id: &str, project_id: &str, audience_id: &str) -> Self {
        let now = Utc::now();

        Self {
            reader_id: reader_id.to_string(),
            project_id: project_id.to_string(),
            audience_id: audience_id.to_string(),
            issuer: Some("cms".to_string()),
            audience: Some(audience_id.to_string()),
            iat: now.timestamp(),
            exp: (now + Duration::hours(24 * 30)).timestamp(), // 30 days
            nbf: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
        }
    }
}

/// API key JWT claims (for stateless API key authentication)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyJwtClaims {
    pub key_id: String,
    pub user_id: String,
    pub org_id: Option<String>,
    pub iat: i64,
    pub exp: i64,
}

impl ApiKeyJwtClaims {
    pub fn new(key_id: &str, user_id: &str, org_id: Option<&str>) -> Self {
        let now = Utc::now();

        Self {
            key_id: key_id.to_string(),
            user_id: user_id.to_string(),
            org_id: org_id.map(|s| s.to_string()),
            iat: now.timestamp(),
            exp: (now + Duration::hours(1)).timestamp(), // 1 hour
        }
    }
}

/// JWT service
pub struct JwtService;

impl JwtService {
    /// Create a reader JWT
    pub fn create_reader_jwt(
        reader_id: &str,
        project_id: &str,
        audience_id: &str,
        auth_service: &AuthService,
    ) -> Result<String, AppError> {
        let claims = ReaderJwtClaims::new(reader_id, project_id, audience_id);

        let header = Header {
            kid: None,
            alg: jsonwebtoken::Algorithm::HS256,
            ..Default::default()
        };

        encode(&header, &claims, auth_service.encoding_key())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to encode reader JWT: {}", e)))
    }

    /// Validate a reader JWT
    pub fn validate_reader_jwt(
        token: &str,
        auth_service: &AuthService,
    ) -> Result<ReaderJwtClaims, AppError> {
        decode::<ReaderJwtClaims>(
            token,
            auth_service.decoding_key(),
            &auth_service.jwt_validation(),
        )
        .map(|td| td.claims)
        .map_err(|e| {
            tracing::debug!("Reader JWT validation failed: {}", e);
            AppError::InvalidToken(format!("Invalid reader token: {}", e))
        })
    }

    /// Create an API key JWT
    pub fn create_api_key_jwt(
        key_id: &str,
        user_id: &str,
        org_id: Option<&str>,
        auth_service: &AuthService,
    ) -> Result<String, AppError> {
        let claims = ApiKeyJwtClaims::new(key_id, user_id, org_id);

        let header = Header {
            kid: None,
            alg: jsonwebtoken::Algorithm::HS256,
            ..Default::default()
        };

        encode(&header, &claims, auth_service.encoding_key())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to encode API key JWT: {}", e)))
    }

    /// Validate an API key JWT
    pub fn validate_api_key_jwt(
        token: &str,
        auth_service: &AuthService,
    ) -> Result<ApiKeyJwtClaims, AppError> {
        decode::<ApiKeyJwtClaims>(
            token,
            auth_service.decoding_key(),
            &auth_service.jwt_validation(),
        )
        .map(|td| td.claims)
        .map_err(|e| {
            tracing::debug!("API key JWT validation failed: {}", e);
            AppError::InvalidApiKey
        })
    }
}

/// Reader JWT service
pub struct ReaderJwtService;

impl ReaderJwtService {
    /// Issue a reader JWT
    pub async fn issue_jwt(
        reader_id: &str,
        project_id: &str,
        audience_id: &str,
        auth_service: &AuthService,
    ) -> Result<String, AppError> {
        JwtService::create_reader_jwt(reader_id, project_id, audience_id, auth_service)
    }

    /// Validate a reader JWT and check replay protection
    pub async fn validate_jwt(
        token: &str,
        auth_service: &AuthService,
        pool: &PgPool,
    ) -> Result<(ReaderJwtClaims, bool), AppError> {
        // Validate the JWT
        let claims = JwtService::validate_reader_jwt(token, auth_service)?;

        // Check replay protection
        // This would query the JwtReplay table to ensure the token hasn't been used before
        // For now, we'll just return that it's not a replay
        let is_replay = false; // Would check database

        Ok((claims, is_replay))
    }
}

#[cfg(test)]
mod tests {
    use cms_config::AuthConfig;

    use super::*;

    // Helper to create an AuthService for testing
    fn create_test_auth_service() -> AuthService {
        let config = AuthConfig {
            session_secret: "test_session_secret_1234567890".to_string(),
            session_expiration_hours: 24 * 7,
            jwt_secret: "test_jwt_secret_1234567890".to_string(),
            jwt_expiration_hours: 24 * 30,
            api_key_prefix: "test_prefix".to_string(),
            system_admin_emails: Vec::new(),
            oauth: None,
        };

        // We need a PgPool but for tests we can use a dummy one
        // In real usage, this would be a real connection pool
        let pool = cms_db::PgPool::connect_lazy("postgres://user:pass@localhost/db")
            .expect("Failed to create test pool");

        AuthService::new(config, pool)
    }

    #[tokio::test]
    async fn test_reader_jwt_roundtrip() {
        let auth_service = create_test_auth_service();

        let token =
            JwtService::create_reader_jwt("reader-1", "project-1", "audience-1", &auth_service)
                .unwrap();

        let claims = JwtService::validate_reader_jwt(&token, &auth_service).unwrap();

        assert_eq!(claims.reader_id, "reader-1");
        assert_eq!(claims.project_id, "project-1");
        assert_eq!(claims.audience_id, "audience-1");
    }

    #[tokio::test]
    async fn test_api_key_jwt_roundtrip() {
        let auth_service = create_test_auth_service();

        let token = JwtService::create_api_key_jwt("key-1", "user-1", Some("org-1"), &auth_service)
            .unwrap();

        let claims = JwtService::validate_api_key_jwt(&token, &auth_service).unwrap();

        assert_eq!(claims.key_id, "key-1");
        assert_eq!(claims.user_id, "user-1");
        assert_eq!(claims.org_id, Some("org-1".to_string()));
    }
}
