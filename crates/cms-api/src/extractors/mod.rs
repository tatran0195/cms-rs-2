//! Common extractors for API handlers

use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use cms_error::AppError;

/// User ID extractor from session or header
#[derive(Debug, Clone)]
pub struct UserId(pub String);

use crate::auth::AuthExtractor;

impl<S> FromRequestParts<S> for UserId
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Use the secure AuthExtractor to get the authenticated user
        if let Ok(auth) = AuthExtractor::from_request_parts(parts, _state).await {
            return Ok(UserId(auth.user.id));
        }

        Err(AppError::Unauthorized)
    }
}

/// Organization ID extractor from path or query
#[derive(Debug, Clone)]
pub struct OrgId(pub String);

impl<S> FromRequestParts<S> for OrgId
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get from X-Org-ID header
        if let Some(org_id) = parts.headers.get("X-Org-ID") {
            return Ok(OrgId(
                org_id
                    .to_str()
                    .map_err(|_| AppError::Unauthorized)?
                    .to_string(),
            ));
        }

        Err(AppError::Unauthorized)
    }
}

/// Project ID extractor from path or header
#[derive(Debug, Clone)]
pub struct ProjectId(pub String);

impl<S> FromRequestParts<S> for ProjectId
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get from X-Project-ID header
        if let Some(project_id) = parts.headers.get("X-Project-ID") {
            return Ok(ProjectId(
                project_id
                    .to_str()
                    .map_err(|_| AppError::Unauthorized)?
                    .to_string(),
            ));
        }

        Err(AppError::Unauthorized)
    }
}
