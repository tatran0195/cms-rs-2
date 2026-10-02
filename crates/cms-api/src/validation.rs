//! Request validation utilities
//!
//! This module provides extractors and utilities for validating request data.

use async_trait::async_trait;
use axum::{
    extract::{FromRequest, FromRequestParts, Path, Query, Request},
    http::request::Parts,
    response::{IntoResponse, Response},
    Json,
};
use cms_error::AppError;
use serde::{de::DeserializeOwned, Serialize};
use validator::Validate;

/// Validated JSON extractor
///
/// This extractor validates the JSON body against the Validate trait
/// before passing it to the handler.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate + 'static,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

        validate(&value)?;
        Ok(Self(value))
    }
}

impl<T> IntoResponse for ValidatedJson<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        Json(self.0).into_response()
    }
}

impl<T> std::ops::Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Validated query extractor
///
/// Similar to ValidatedJson but for query parameters.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate + 'static,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| AppError::BadRequest(format!("Invalid query parameters: {}", e)))?;

        validate(&value)?;
        Ok(Self(value))
    }
}

impl<T> std::ops::Deref for ValidatedQuery<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Validate a value and return an error if invalid
pub fn validate<T: Validate>(value: &T) -> Result<(), AppError> {
    value.validate().map_err(|e| {
        let errors: Vec<String> = e
            .field_errors()
            .into_iter()
            .map(|(field, errors)| {
                let error_messages: Vec<String> = errors
                    .iter()
                    .map(|e| {
                        e.message
                            .as_ref()
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| e.code.to_string())
                    })
                    .collect();
                format!("{}:", field) + &error_messages.join(", ")
            })
            .collect();
        AppError::Validation(errors.join("; "))
    })
}

/// Validated path extractor
///
/// For validating path parameters.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedPath<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidatedPath<T>
where
    T: DeserializeOwned + Validate + Send + 'static,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| AppError::BadRequest(format!("Invalid path parameters: {}", e)))?;

        validate(&value)?;
        Ok(Self(value))
    }
}

impl<T> std::ops::Deref for ValidatedPath<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use validator::Validate;

    use super::*;

    #[derive(Debug, Deserialize, Validate)]
    struct TestRequest {
        #[validate(email)]
        email: String,
        #[validate(length(min = 8))]
        password: String,
    }

    #[tokio::test]
    async fn test_validate_function() {
        let valid = TestRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        assert!(validate(&valid).is_ok());

        let invalid = TestRequest {
            email: "invalid-email".to_string(),
            password: "short".to_string(),
        };

        assert!(validate(&invalid).is_err());
    }
}
