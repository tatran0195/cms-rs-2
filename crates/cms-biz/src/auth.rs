//! Authentication Business Logic Extensions
//!
//! This module contains additional authentication business logic
//! that doesn't fit in the main auth crate.

use chrono::{DateTime, Utc};
use cms_db::auth::{AccountQueries, ApiKeyQueries, SessionQueries, UserQueries};
use cms_entity::auth::{ApiKeyResponse, CreateUserRequest, UserResponse};
use uuid::Uuid;

use crate::{
    platform_event::{FunnelEventType, PlatformEventService},
    AppError, BizContext,
};

/// Authentication service for API handlers
pub struct AuthService;

impl AuthService {
    async fn ensure_active_account(ctx: &BizContext, user_id: &str) -> Result<(), AppError> {
        if UserQueries::is_suspended(&ctx.pool, user_id).await? {
            return Err(AppError::Forbidden);
        }
        Ok(())
    }

    pub async fn login(
        ctx: &BizContext,
        email: &str,
        password: &str,
    ) -> Result<UserResponse, AppError> {
        let user = UserQueries::get_by_email(&ctx.pool, email)
            .await?
            .ok_or(AppError::InvalidCredentials)?;

        let account = AccountQueries::get_by_provider(&ctx.pool, "credentials", email).await?;
        if let Some(account) = account {
            if let Some(ref hash) = account.access_token {
                let is_valid = cms_auth::password::verify_password(password, hash).unwrap_or(false);
                if !is_valid {
                    return Err(AppError::InvalidCredentials);
                }
            } else {
                return Err(AppError::InvalidCredentials);
            }
        } else {
            return Err(AppError::InvalidCredentials);
        }

        Self::ensure_active_account(ctx, &user.id).await?;
        Ok(user.into())
    }

    pub async fn register(
        ctx: &BizContext,
        email: &str,
        password: &str,
        name: Option<&str>,
    ) -> Result<UserResponse, AppError> {
        let existing = UserQueries::get_by_email(&ctx.pool, email).await?;
        if existing.is_some() {
            return Err(AppError::Conflict("Email already exists".to_string()));
        }
        let user = UserQueries::create(&ctx.pool, email, name, None, false).await?;

        if !password.is_empty() {
            let hashed = cms_auth::password::hash_password(password)?;
            AccountQueries::create(
                &ctx.pool,
                &user.id,
                "credentials",
                email,
                Some(&hashed),
                None,
                None,
                None,
                None,
            )
            .await?;
        }

        PlatformEventService::record_funnel_event_best_effort(
            ctx,
            &user.id,
            None,
            FunnelEventType::SignupCompleted,
            serde_json::json!({ "method": "password" }),
        )
        .await;

        Ok(user.into())
    }

    pub async fn logout(ctx: &BizContext, session_token: &str) -> Result<(), AppError> {
        if let Some(session) = SessionQueries::get_by_token(&ctx.pool, session_token).await? {
            SessionQueries::delete(&ctx.pool, &session.id).await?;
        }
        Ok(())
    }

    pub async fn get_user_by_session(
        ctx: &BizContext,
        session_token: &str,
    ) -> Result<UserResponse, AppError> {
        let session = SessionQueries::get_by_token(&ctx.pool, session_token)
            .await?
            .ok_or(AppError::InvalidSession)?;
        if session.expires_at < Utc::now() {
            return Err(AppError::InvalidSession);
        }
        let user = UserQueries::get_by_id(&ctx.pool, &session.user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
        Self::ensure_active_account(ctx, &user.id).await?;
        Ok(user.into())
    }

    pub async fn get_user_by_jwt(ctx: &BizContext, token: &str) -> Result<UserResponse, AppError> {
        if let Some(session) = SessionQueries::get_by_token(&ctx.pool, token).await? {
            if session.expires_at < Utc::now() {
                return Err(AppError::TokenExpired);
            }
            let user = UserQueries::get_by_id(&ctx.pool, &session.user_id)
                .await?
                .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
            Self::ensure_active_account(ctx, &user.id).await?;
            return Ok(user.into());
        }

        let user = UserQueries::get_by_id(&ctx.pool, token)
            .await?
            .ok_or(AppError::InvalidCredentials)?;
        Self::ensure_active_account(ctx, &user.id).await?;
        Ok(user.into())
    }

    pub async fn get_user_by_api_key(
        ctx: &BizContext,
        api_key: &str,
    ) -> Result<UserResponse, AppError> {
        let hashed = cms_auth::api_key::hash_key(api_key);
        let key = ApiKeyQueries::get_by_key(&ctx.pool, &hashed)
            .await?
            .ok_or(AppError::InvalidApiKey)?;
        let user = UserQueries::get_by_id(&ctx.pool, &key.user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
        Self::ensure_active_account(ctx, &user.id).await?;
        let _ = ApiKeyQueries::update_last_used(&ctx.pool, &key.id).await;
        Ok(user.into())
    }

    pub async fn list_api_keys(
        ctx: &BizContext,
        user_id: &str,
    ) -> Result<Vec<ApiKeyResponse>, AppError> {
        ApiKeyQueries::get_all_for_user(&ctx.pool, user_id).await
    }

    pub async fn get_user(ctx: &BizContext, user_id: &str) -> Result<UserResponse, AppError> {
        let user = UserQueries::get_by_id(&ctx.pool, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
        Ok(user.into())
    }

    pub async fn refresh_session(
        ctx: &BizContext,
        refresh_token: &str,
    ) -> Result<UserResponse, AppError> {
        let session = SessionQueries::get_by_token(&ctx.pool, refresh_token)
            .await?
            .ok_or(AppError::InvalidSession)?;
        let user = UserQueries::get_by_id(&ctx.pool, &session.user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
        Self::ensure_active_account(ctx, &user.id).await?;
        Ok(user.into())
    }

    pub async fn create_api_key(
        ctx: &BizContext,
        user_id: &str,
        name: &str,
    ) -> Result<ApiKeyResponse, AppError> {
        let raw_key = format!("nbl_{}", Uuid::new_v4().to_string().replace("-", ""));
        let hashed = cms_auth::api_key::hash_key(&raw_key);
        let key = ApiKeyQueries::create(&ctx.pool, user_id, name, &hashed).await?;
        Ok(key.into())
    }

    pub async fn delete_api_key(
        ctx: &BizContext,
        user_id: &str,
        key_id: &str,
    ) -> Result<(), AppError> {
        let key = ApiKeyQueries::get_by_id(&ctx.pool, key_id)
            .await?
            .ok_or_else(|| AppError::NotFound("API key not found".to_string()))?;
        if key.user_id != user_id {
            return Err(AppError::Forbidden);
        }
        ApiKeyQueries::delete(&ctx.pool, key_id).await?;
        Ok(())
    }
}

/// Auth extensions service
pub struct AuthExtensionsService;

impl AuthExtensionsService {
    /// Create a user with additional validation
    pub async fn create_user_with_validation(
        ctx: &BizContext,
        request: CreateUserRequest,
    ) -> Result<UserResponse, AppError> {
        // Validate email format
        if !Self::is_valid_email(&request.email) {
            return Err(AppError::Validation("Invalid email format".to_string()));
        }

        // Check if email is already registered
        let existing = UserQueries::get_by_email(&ctx.pool, &request.email).await?;
        if existing.is_some() {
            return Err(AppError::Conflict("Email already registered".to_string()));
        }

        // Create the user
        let user = UserQueries::create(
            &ctx.pool,
            &request.email,
            request.name.as_deref(),
            request.image.as_deref(),
            false, // Email not verified yet
        )
        .await?;

        Ok(user.into())
    }

    /// Validate email format
    fn is_valid_email(email: &str) -> bool {
        // Simple email validation
        email.contains('@') && email.contains('.')
    }

    /// Generate a verification token
    pub async fn generate_verification_token(
        ctx: &BizContext,
        email: &str,
    ) -> Result<String, AppError> {
        use chrono::Duration;
        use cms_db::auth::VerificationTokenQueries;
        use uuid::Uuid;

        let token = Uuid::new_v4().to_string();
        let expires_at = chrono::Utc::now() + Duration::hours(24);

        VerificationTokenQueries::create(&ctx.pool, email, &token, expires_at).await?;

        Ok(token)
    }

    /// Verify email with token
    pub async fn verify_email(ctx: &BizContext, token: &str) -> Result<UserResponse, AppError> {
        use cms_db::auth::VerificationTokenQueries;

        let verification_token = VerificationTokenQueries::get_by_token(&ctx.pool, token)
            .await?
            .ok_or_else(|| AppError::InvalidToken("Verification token not found".to_string()))?;

        if verification_token.expires_at < chrono::Utc::now() {
            return Err(AppError::TokenExpired);
        }

        // Mark user as verified
        let user = UserQueries::get_by_email(&ctx.pool, &verification_token.identifier)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let updated = UserQueries::update_verified(&ctx.pool, &user.id, true).await?;

        // Delete the verification token
        VerificationTokenQueries::delete(&ctx.pool, &verification_token.id).await?;

        Ok(updated.into())
    }
}
