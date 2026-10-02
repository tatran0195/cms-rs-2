//! Authentication middleware
//!
//! This module provides middleware for extracting and validating authentication
//! from requests (sessions, JWT tokens, API keys).

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::Cookie;
use base64::prelude::*;
use cms_biz::auth::AuthService;
use cms_entity::auth::{ApiKey, UserResponse};
use cms_error::AppError;
use cms_middleware::app_state::AppState;

/// Authentication extractor
///
/// This extractor tries the user in the following order:
/// 1. Session cookie (session_token)
/// 2. Bearer token (JWT)
/// 3. API key (X-API-Key header)
/// 4. Basic auth (for development)
#[derive(Debug, Clone)]
pub struct AuthExtractor {
    pub user: UserResponse,
    pub auth_method: AuthMethod,
}

/// Authentication method used
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    Session,
    Jwt,
    ApiKey,
    Basic,
    None,
}

impl<S> FromRequestParts<S> for AuthExtractor
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get AppState from extensions
        let state = parts
            .extensions
            .get::<Arc<AppState>>()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("AppState not found in extensions")))?
            .clone();

        // Try session cookie first
        if let Ok(user) = extract_from_session(parts, &state).await {
            return Ok(AuthExtractor {
                user,
                auth_method: AuthMethod::Session,
            });
        }

        // Try Bearer token (JWT)
        if let Ok(user) = extract_from_jwt(parts, &state).await {
            return Ok(AuthExtractor {
                user,
                auth_method: AuthMethod::Jwt,
            });
        }

        // Try API key
        if let Ok(user) = extract_from_api_key(parts, &state).await {
            return Ok(AuthExtractor {
                user,
                auth_method: AuthMethod::ApiKey,
            });
        }

        // Try Basic auth (for development)
        if let Ok(user) = extract_from_basic_auth(parts, &state).await {
            return Ok(AuthExtractor {
                user,
                auth_method: AuthMethod::Basic,
            });
        }

        Err(AppError::Unauthorized)
    }
}

/// Extract user from session cookie
async fn extract_from_session(
    parts: &mut Parts,
    state: &Arc<AppState>,
) -> Result<UserResponse, AppError> {
    let cookie_header = parts
        .headers
        .get(header::COOKIE)
        .ok_or(AppError::Unauthorized)?;

    let cookie_str = cookie_header.to_str().map_err(|_| AppError::Unauthorized)?;

    let session_token = Cookie::split_parse(cookie_str)
        .find_map(|c| {
            c.ok()
                .filter(|c| c.name() == "better-auth.session_token")
                .map(|c| c.value().to_string())
        })
        .ok_or(AppError::Unauthorized)?;

    // Validate session and get user
    AuthService::get_user_by_session(&state.biz_context, &session_token).await
}

/// Extract user from JWT Bearer token
async fn extract_from_jwt(
    parts: &mut Parts,
    state: &Arc<AppState>,
) -> Result<UserResponse, AppError> {
    // Get Authorization header
    let auth_header = parts
        .headers
        .get(header::AUTHORIZATION)
        .ok_or(AppError::Unauthorized)?;

    let auth_value = auth_header.to_str().map_err(|_| AppError::Unauthorized)?;

    // Check for Bearer scheme
    if !auth_value.starts_with("Bearer ") {
        return Err(AppError::Unauthorized);
    }

    let token = &auth_value[7..];

    // Validate JWT and get user
    AuthService::get_user_by_jwt(&state.biz_context, token).await
}

/// Extract user from API key
async fn extract_from_api_key(
    parts: &mut Parts,
    state: &Arc<AppState>,
) -> Result<UserResponse, AppError> {
    // Get API key from header
    let api_key_header = parts
        .headers
        .get("X-API-Key")
        .ok_or(AppError::Unauthorized)?;

    let api_key = api_key_header
        .to_str()
        .map_err(|_| AppError::Unauthorized)?;

    // Validate API key and get user
    AuthService::get_user_by_api_key(&state.biz_context, api_key).await
}

/// Extract user from Basic auth
async fn extract_from_basic_auth(
    parts: &mut Parts,
    state: &Arc<AppState>,
) -> Result<UserResponse, AppError> {
    // Get Authorization header
    let auth_header = parts
        .headers
        .get(header::AUTHORIZATION)
        .ok_or(AppError::Unauthorized)?;

    let auth_value = auth_header.to_str().map_err(|_| AppError::Unauthorized)?;

    // Check for Basic scheme
    if !auth_value.starts_with("Basic ") {
        return Err(AppError::Unauthorized);
    }

    // Decode base64 credentials
    let encoded = &auth_value[6..];
    let decoded = BASE64_STANDARD
        .decode(encoded.trim().as_bytes())
        .map_err(|_| AppError::Unauthorized)?;

    let credentials = String::from_utf8(decoded).map_err(|_| AppError::Unauthorized)?;

    // Split into username:password
    let parts: Vec<&str> = credentials.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(AppError::Unauthorized);
    }

    let email = parts[0];
    let password = parts[1];

    // Authenticate user
    AuthService::login(&state.biz_context, email, password).await
}

/// Optional authentication extractor
///
/// This extractor attempts authentication but doesn't fail if not present.
/// Useful for public routes that can optionally use authentication.
#[derive(Debug, Clone)]
pub struct OptionalAuthExtractor {
    pub user: Option<UserResponse>,
    pub auth_method: Option<AuthMethod>,
}

impl<S> FromRequestParts<S> for OptionalAuthExtractor
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get AppState from extensions
        let state = parts
            .extensions
            .get::<Arc<AppState>>()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("AppState not found in extensions")))?
            .clone();

        // Try all authentication methods

        // Try session cookie
        if let Ok(user) = extract_from_session(parts, &state).await {
            return Ok(OptionalAuthExtractor {
                user: Some(user),
                auth_method: Some(AuthMethod::Session),
            });
        }

        // Try Bearer token (JWT)
        if let Ok(user) = extract_from_jwt(parts, &state).await {
            return Ok(OptionalAuthExtractor {
                user: Some(user),
                auth_method: Some(AuthMethod::Jwt),
            });
        }

        // Try API key
        if let Ok(user) = extract_from_api_key(parts, &state).await {
            return Ok(OptionalAuthExtractor {
                user: Some(user),
                auth_method: Some(AuthMethod::ApiKey),
            });
        }

        // Try Basic auth
        if let Ok(user) = extract_from_basic_auth(parts, &state).await {
            return Ok(OptionalAuthExtractor {
                user: Some(user),
                auth_method: Some(AuthMethod::Basic),
            });
        }

        // No authentication found
        Ok(OptionalAuthExtractor {
            user: None,
            auth_method: None,
        })
    }
}

/// Authentication middleware for requiring authentication
pub struct RequireAuthMiddleware;

impl Default for RequireAuthMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl RequireAuthMiddleware {
    pub fn new() -> Self {
        Self
    }
}

/// Authentication rejection
#[derive(Debug, Clone)]
pub struct AuthRejection;

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, "Authentication required").into_response()
    }
}

/// Require specific role middleware
pub struct RequireRoleMiddleware {
    pub role: cms_entity::common::MemberRole,
}

impl RequireRoleMiddleware {
    pub fn new(role: cms_entity::common::MemberRole) -> Self {
        Self { role }
    }
}

/// Role requirement rejection
#[derive(Debug, Clone)]
pub struct RoleRejection;

impl IntoResponse for RoleRejection {
    fn into_response(self) -> Response {
        (StatusCode::FORBIDDEN, "Insufficient permissions").into_response()
    }
}

/// Check if user has required role in organization
pub async fn require_org_role(
    user_id: &str,
    org_id: &str,
    required_role: cms_entity::common::MemberRole,
    state: &Arc<AppState>,
) -> Result<(), AppError> {
    match required_role {
        cms_entity::common::MemberRole::Owner => {
            state
                .biz_context
                .authz
                .require_org_owner(user_id, org_id)
                .await
        }
        cms_entity::common::MemberRole::Admin => {
            state
                .biz_context
                .authz
                .require_org_admin(user_id, org_id)
                .await
        }
        _ => {
            state
                .biz_context
                .authz
                .require_org_member(user_id, org_id)
                .await
        }
    }
}

/// Check if user has required role in project
pub async fn require_project_role(
    user_id: &str,
    project_id: &str,
    required_role: cms_entity::common::MemberRole,
    state: &Arc<AppState>,
) -> Result<(), AppError> {
    state
        .biz_context
        .authz
        .require_project_role(user_id, project_id, required_role)
        .await
}

/// Check if user is owner of organization
pub async fn require_org_owner(
    user_id: &str,
    org_id: &str,
    state: &Arc<AppState>,
) -> Result<(), AppError> {
    state
        .biz_context
        .authz
        .require_org_owner(user_id, org_id)
        .await
}

/// Check if user is owner of project
pub async fn require_project_owner(
    user_id: &str,
    project_id: &str,
    state: &Arc<AppState>,
) -> Result<(), AppError> {
    state
        .biz_context
        .authz
        .require_project_role(user_id, project_id, cms_entity::common::MemberRole::Owner)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_method_equality() {
        assert_eq!(AuthMethod::Session, AuthMethod::Session);
        assert_ne!(AuthMethod::Session, AuthMethod::Jwt);
    }
}
