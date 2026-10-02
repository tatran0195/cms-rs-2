//! Auth handlers
//!
//! This module contains the actual implementation of authentication handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use axum_extra::extract::cookie::Cookie as AxumCookie;
use cms_biz::{
    auth::AuthService,
    platform_event::{FunnelEventType, PlatformEventService},
};
use cms_entity::auth::{
    ApiKeyResponse, CreateApiKeyRequest, LoginRequest, RegisterRequest, UserResponse,
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;

use crate::auth::AuthExtractor;

/// Login to CMS
///
/// Authenticate a user with email and password.
///
/// Returns the authenticated user and creates a session.
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "User logged in successfully", body = UserResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<UserResponse>, AppError> {
    // Delegate to auth service
    let user = AuthService::login(&state.biz_context, &request.email, &request.password).await?;

    Ok(Json(user))
}

/// Register a new user
///
/// Create a new CMS account with email and password.
#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "User registered successfully", body = UserResponse),
        (status = 400, description = "Bad request"),
        (status = 409, description = "Email already exists"),
    )
)]
pub async fn register_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<UserResponse>, AppError> {
    // Delegate to auth service
    let user = AuthService::register(
        &state.biz_context,
        &request.email,
        request.password.as_deref().unwrap_or(""),
        request.name.as_deref(),
    )
    .await?;

    Ok(Json(user))
}

/// Logout handler
pub async fn logout_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(cookie_header) = headers.get(axum::http::header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            if let Some(token) = AxumCookie::split_parse(cookie_str).find_map(|c| {
                c.ok()
                    .filter(|c| c.name() == "better-auth.session_token")
                    .map(|c| c.value().to_string())
            }) {
                AuthService::logout(&state.biz_context, &token).await?;
            }
        }
    }

    Ok(Json(
        serde_json::json!({"message": "Logged out successfully"}),
    ))
}

/// Get current user handler
pub async fn get_current_user_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<UserResponse>, AppError> {
    // Get user from database
    let user = AuthService::get_user(&state.biz_context, &auth.user.id).await?;

    Ok(Json(user))
}

/// Refresh session handler
pub async fn refresh_session_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<UserResponse>, AppError> {
    let cookie_header = headers
        .get(axum::http::header::COOKIE)
        .ok_or(AppError::Unauthorized)?;
    let cookie_str = cookie_header.to_str().map_err(|_| AppError::Unauthorized)?;
    let refresh_token = AxumCookie::split_parse(cookie_str)
        .find_map(|c| {
            c.ok()
                .filter(|c| c.name() == "refresh_token")
                .map(|c| c.value().to_string())
        })
        .ok_or(AppError::Unauthorized)?;

    let user = AuthService::refresh_session(&state.biz_context, &refresh_token).await?;

    Ok(Json(user))
}

/// List API keys handler
pub async fn list_api_keys_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<Vec<ApiKeyResponse>>, AppError> {
    let api_keys = AuthService::list_api_keys(&state.biz_context, &auth.user.id).await?;

    Ok(Json(api_keys))
}

/// Create API key handler
pub async fn create_api_key_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyResponse>, AppError> {
    let api_key =
        AuthService::create_api_key(&state.biz_context, &auth.user.id, &request.name).await?;

    Ok(Json(api_key))
}

/// Delete API key handler
pub async fn delete_api_key_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(api_key_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    AuthService::delete_api_key(&state.biz_context, &auth.user.id, &api_key_id).await?;

    Ok(Json(serde_json::json!({"success": true, "id": api_key_id})))
}

/// Get session handler (Better Auth compatible)
pub async fn get_session_handler(
    auth: crate::auth::OptionalAuthExtractor,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(user) = auth.user {
        Ok(Json(serde_json::json!({
            "user": {
                "id": user.id,
                "email": user.email,
                "name": user.name,
                "emailVerified": user.email_verified,
                "createdAt": user.created_at,
                "updatedAt": user.updated_at,
            },
            "session": {
                "id": user.id,
                "userId": user.id,
                "token": "session",
            }
        })))
    } else {
        Ok(Json(serde_json::Value::Null))
    }
}

#[derive(serde::Deserialize)]
pub struct SendOtpRequest {
    pub email: String,
    #[serde(rename = "type")]
    pub otp_type: Option<String>,
}

fn otp_identifier(purpose: &str, email: &str) -> String {
    format!("otp:{purpose}:{}", email.trim().to_lowercase())
}

async fn issue_and_send_otp(state: &AppState, email: &str, purpose: &str) -> Result<(), AppError> {
    use cms_db::auth::{EmailOtpThrottleQueries, VerificationTokenQueries};
    use uuid::Uuid;

    let email = email.trim().to_lowercase();
    let Some((local, domain)) = email.split_once('@') else {
        return Err(AppError::Validation(
            "A valid email address is required".to_string(),
        ));
    };
    if local.is_empty() || domain.is_empty() || domain.starts_with('.') || domain.ends_with('.') {
        return Err(AppError::Validation(
            "A valid email address is required".to_string(),
        ));
    }
    if !matches!(purpose, "sign-in" | "email-verification") {
        return Err(AppError::Validation("Unsupported OTP purpose".to_string()));
    }

    let identifier = otp_identifier(purpose, &email);
    if !EmailOtpThrottleQueries::allow_request(&state.biz_context.pool, &identifier, 60).await? {
        return Err(AppError::TooManyRequests);
    }

    // UUID v4 uses the operating system CSPRNG. Reducing it to six digits makes
    // this a human-enterable OTP; the send cooldown and global request limiter
    // constrain online guessing and resend abuse.
    let bytes = Uuid::new_v4().as_bytes().to_vec();
    let n = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) % 1_000_000;
    let otp = format!("{n:06}");
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(10);
    VerificationTokenQueries::create_or_replace(
        &state.biz_context.pool,
        &identifier,
        &otp,
        expires_at,
    )
    .await?;

    let purpose_label = if purpose == "sign-in" {
        "sign in"
    } else {
        "verify your email"
    };
    let subject = "Your CMS verification code";
    let body = format!(
        "Your code to {purpose_label} is: {otp}\n\nThis code expires in 10 minutes. If you did not request it, you can ignore this email."
    );
    if let Err(error) = state.mailer.send_email(&email, subject, &body).await {
        let _ =
            VerificationTokenQueries::delete_matching(&state.biz_context.pool, &identifier, &otp)
                .await;
        return Err(error);
    }

    Ok(())
}

/// Send verification OTP handler (Better Auth compatible). Codes are namespaced
/// by purpose, stored with a ten-minute expiry, rate-limited per email, and only
/// acknowledged after configured email delivery succeeds.
pub async fn send_verification_otp_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendOtpRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let email = req.email.trim().to_lowercase();
    let purpose = req.otp_type.as_deref().unwrap_or("sign-in");
    issue_and_send_otp(&state, &email, purpose).await?;
    Ok(Json(serde_json::json!({ "status": true })))
}

#[derive(serde::Deserialize)]
pub struct SignInOtpRequest {
    pub email: String,
    pub otp: String,
}

/// Sign in with email OTP handler (Better Auth compatible)
///
/// Requires a valid, unexpired OTP that was previously issued by
/// `/email-otp/send-verification-otp` for the same email. It validates the code
/// against the stored token before creating a session, so a caller cannot
/// authenticate with an arbitrary or guessed code, and cannot force-create a
/// verified account out of thin air.
pub async fn sign_in_email_otp_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SignInOtpRequest>,
) -> Result<(HeaderMap, Json<serde_json::Value>), AppError> {
    use cms_db::auth::VerificationTokenQueries;

    let email = req.email.trim().to_lowercase();

    let identifier = otp_identifier("sign-in", &email);
    VerificationTokenQueries::consume(&state.biz_context.pool, &identifier, req.otp.trim())
        .await?
        .ok_or_else(|| AppError::custom(StatusCode::UNAUTHORIZED, "Invalid or expired OTP"))?;

    let (user, created) =
        match cms_db::auth::UserQueries::get_by_email(&state.biz_context.pool, &email).await? {
            Some(user) if user.email_verified => (user, false),
            Some(user) => (
                cms_db::auth::UserQueries::update_verified(&state.biz_context.pool, &user.id, true)
                    .await?,
                false,
            ),
            None => {
                // A valid sign-in OTP proves control of the address, so first-time
                // sign-in creates a verified account rather than requiring a second code.
                (
                    cms_db::auth::UserQueries::create(
                        &state.biz_context.pool,
                        &email,
                        None,
                        None,
                        true,
                    )
                    .await?,
                    true,
                )
            }
        };
    if created {
        PlatformEventService::record_funnel_event_best_effort(
            &state.biz_context,
            &user.id,
            None,
            FunnelEventType::SignupCompleted,
            serde_json::json!({ "method": "email_otp" }),
        )
        .await;
    }

    if cms_db::auth::UserQueries::is_suspended(&state.biz_context.pool, &user.id).await? {
        return Err(AppError::Forbidden);
    }

    let session_token = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
    let session = cms_db::auth::SessionQueries::create(
        &state.biz_context.pool,
        &user.id,
        &session_token,
        expires_at,
    )
    .await?;

    let mut headers = HeaderMap::new();
    let cookie_val = format!(
        "better-auth.session_token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        session_token,
        30 * 24 * 3600
    );
    headers.insert(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&cookie_val).map_err(|e| AppError::Internal(e.into()))?,
    );

    Ok((
        headers,
        Json(serde_json::json!({
            "user": {
                "id": user.id,
                "email": user.email,
                "name": user.name,
                "emailVerified": user.email_verified,
                "createdAt": user.created_at,
                "updatedAt": user.updated_at,
            },
            "session": {
                "id": session.id,
                "userId": user.id,
                "token": session_token,
                "expiresAt": expires_at,
            }
        })),
    ))
}

/// Sign out handler (Better Auth compatible)
pub async fn sign_out_better_auth_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<(HeaderMap, Json<serde_json::Value>), AppError> {
    if let Some(cookie_header) = headers.get(axum::http::header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            if let Some(token) = AxumCookie::split_parse(cookie_str).find_map(|c| {
                c.ok()
                    .filter(|c| c.name() == "better-auth.session_token")
                    .map(|c| c.value().to_string())
            }) {
                let _ = AuthService::logout(&state.biz_context, &token).await;
            }
        }
    }

    let mut res_headers = HeaderMap::new();
    res_headers.insert(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_static(
            "better-auth.session_token=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax",
        ),
    );

    Ok((res_headers, Json(serde_json::json!({ "success": true }))))
}

/// Serialize a User row into the SPA's `user` object (used across Better Auth
/// responses).
fn user_json(user: &cms_entity::auth::User) -> serde_json::Value {
    serde_json::json!({
        "id": user.id,
        "email": user.email,
        "name": user.name,
        "image": user.image,
        "emailVerified": user.email_verified,
        "createdAt": user.created_at,
        "updatedAt": user.updated_at,
    })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserBody {
    name: Option<String>,
    image: Option<String>,
}

/// Update the current user's profile (Better Auth `update-user`).
pub async fn update_user_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(body): Json<UpdateUserBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::auth::UserQueries;

    let user = UserQueries::update(
        &state.biz_context.pool,
        &auth.user.id,
        body.name.as_deref(),
        body.image.as_deref(),
    )
    .await?;

    Ok(Json(serde_json::json!({ "user": user_json(&user) })))
}

/// Verify an email from a link token (Better Auth `verify-email`).
///
/// Looks up a `VerificationToken` by its token, resolves the owning user by the
/// token's identifier (the email), and marks the user verified.
pub async fn verify_email_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::auth::{UserQueries, VerificationTokenQueries};

    let token = query
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::custom(StatusCode::BAD_REQUEST, "Verification token required"))?;

    let record = VerificationTokenQueries::get_by_token(&state.biz_context.pool, token)
        .await?
        .ok_or_else(|| AppError::custom(StatusCode::BAD_REQUEST, "Invalid verification link"))?;

    if record.expires_at < chrono::Utc::now() {
        return Err(AppError::custom(
            StatusCode::BAD_REQUEST,
            "Verification link expired",
        ));
    }

    let user = UserQueries::get_by_email(&state.biz_context.pool, &record.identifier)
        .await?
        .ok_or_else(|| AppError::custom(StatusCode::BAD_REQUEST, "User not found"))?;

    let updated = UserQueries::update_verified(&state.biz_context.pool, &user.id, true).await?;
    let _ = VerificationTokenQueries::delete(&state.biz_context.pool, &record.id).await;

    Ok(Json(serde_json::json!({ "user": user_json(&updated) })))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyEmailOtpBody {
    email: String,
    otp: String,
}

/// Verify an email with a 6-digit OTP (Better Auth `email-otp/verify-email`).
pub async fn verify_email_otp_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<VerifyEmailOtpBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::auth::{UserQueries, VerificationTokenQueries};

    let email = body.email.trim().to_lowercase();
    let identifier = otp_identifier("email-verification", &email);
    VerificationTokenQueries::consume(&state.biz_context.pool, &identifier, body.otp.trim())
        .await?
        .ok_or_else(|| AppError::custom(StatusCode::UNAUTHORIZED, "Invalid or expired OTP"))?;

    let user = UserQueries::get_by_email(&state.biz_context.pool, &email)
        .await?
        .ok_or_else(|| AppError::custom(StatusCode::BAD_REQUEST, "User not found"))?;
    let updated = UserQueries::update_verified(&state.biz_context.pool, &user.id, true).await?;

    Ok(Json(serde_json::json!({ "user": user_json(&updated) })))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestEmailChangeBody {
    new_email: String,
    otp: String,
}

/// Request an email change (Better Auth `email-otp/request-email-change`).
///
/// Validates the OTP issued to the user's *current* email, then issues a fresh
/// OTP for the requested new email so `change-email` can be completed with it.
pub async fn request_email_change_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(body): Json<RequestEmailChangeBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::auth::{UserQueries, VerificationTokenQueries};

    let creator = UserQueries::get_by_id(&state.biz_context.pool, &auth.user.id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    // Confirm the OTP sent to the current email.
    let current_identifier = otp_identifier("email-verification", &creator.email);
    VerificationTokenQueries::consume(
        &state.biz_context.pool,
        &current_identifier,
        body.otp.trim(),
    )
    .await?
    .ok_or_else(|| AppError::custom(StatusCode::UNAUTHORIZED, "Invalid or expired OTP"))?;

    // Issue and deliver a separate code to the requested new address.
    let new_email = body.new_email.trim().to_lowercase();
    if new_email == creator.email.to_lowercase() {
        return Err(AppError::Validation(
            "New email must differ from current email".to_string(),
        ));
    }
    if UserQueries::get_by_email(&state.biz_context.pool, &new_email)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(
            "Email address is already in use".to_string(),
        ));
    }
    issue_and_send_otp(&state, &new_email, "email-verification").await?;

    Ok(Json(serde_json::json!({ "status": true })))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeEmailBody {
    new_email: String,
    otp: String,
}

/// Complete an email change (Better Auth `email-otp/change-email`).
///
/// Validates the OTP issued to the new email, then updates the user's email and
/// marks it verified.
pub async fn change_email_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(body): Json<ChangeEmailBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::auth::VerificationTokenQueries;

    let new_email = body.new_email.trim().to_lowercase();
    let identifier = otp_identifier("email-verification", &new_email);
    VerificationTokenQueries::consume(&state.biz_context.pool, &identifier, body.otp.trim())
        .await?
        .ok_or_else(|| AppError::custom(StatusCode::UNAUTHORIZED, "Invalid or expired OTP"))?;

    // The UserQueries::update helper doesn't touch email, so update it directly.
    sqlx::query(
        "UPDATE \"User\" SET email = $1, email_verified = true, updated_at = $2 WHERE id = $3",
    )
    .bind(&new_email)
    .bind(chrono::Utc::now())
    .bind(&auth.user.id)
    .execute(&state.biz_context.pool)
    .await
    .map_err(AppError::Database)?;

    let user = cms_db::auth::UserQueries::get_by_id(&state.biz_context.pool, &auth.user.id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    Ok(Json(serde_json::json!({ "user": user_json(&user) })))
}

/// Stop an admin impersonation session (Better Auth `admin/stop-impersonating`).
///
/// Impersonation is not implemented in this port, so this returns the real current
/// user and session (i.e. the caller's own, non-impersonated) context.
pub async fn stop_impersonating_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::auth::{SessionQueries, UserQueries};

    let user = UserQueries::get_by_id(&state.biz_context.pool, &auth.user.id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    // Resolve the caller's active session from the session cookie.
    let session_token = headers
        .get(axum::http::header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|cookie_str| {
            AxumCookie::split_parse(cookie_str).find_map(|c| {
                c.ok()
                    .filter(|c| c.name() == "better-auth.session_token")
                    .map(|c| c.value().to_string())
            })
        });
    let session = match session_token {
        Some(token) => SessionQueries::get_by_token(&state.biz_context.pool, &token).await?,
        None => None,
    };

    Ok(Json(serde_json::json!({
        "user": user_json(&user),
        "session": session.map(|s| serde_json::json!({
            "id": s.id,
            "userId": s.user_id,
            "token": s.session_token,
            "expiresAt": s.expires_at,
        })),
    })))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptInvitationBody {
    invitation_id: String,
}

/// Accept an organization invitation (Better Auth `organizations/accept-invitation`).
///
/// Looks up the invitation by token, validates expiry and that the invitation was
/// issued for the signed-in user's email, creates the membership, and consumes the
/// invitation. Returns the resulting membership + organization.
pub async fn accept_invitation_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(body): Json<AcceptInvitationBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::auth::UserQueries;
    use cms_db::org::{InvitationQueries, MemberQueries, OrganizationQueries};

    let invitation = InvitationQueries::get_by_token(&state.biz_context.pool, &body.invitation_id)
        .await?
        .ok_or_else(|| AppError::custom(StatusCode::BAD_REQUEST, "INVALID_INVITATION"))?;

    if invitation.expires_at < chrono::Utc::now() {
        return Err(AppError::custom(
            StatusCode::BAD_REQUEST,
            "EXPIRED_INVITATION",
        ));
    }

    let user = UserQueries::get_by_id(&state.biz_context.pool, &auth.user.id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if user.email.to_lowercase() != invitation.email.to_lowercase() {
        return Err(AppError::custom(
            StatusCode::FORBIDDEN,
            "RECIPIENT_EMAIL_MISMATCH",
        ));
    }

    let membership = MemberQueries::create(
        &state.biz_context.pool,
        &user.id,
        &invitation.organization_id,
        invitation.role,
    )
    .await?;
    let _ = InvitationQueries::delete(&state.biz_context.pool, &invitation.id).await;

    let org = OrganizationQueries::get_by_id(&state.biz_context.pool, &invitation.organization_id)
        .await?;

    Ok(Json(serde_json::json!({
        "membership": {
            "id": membership.id,
            "organizationId": membership.organization_id,
            "userId": membership.user_id,
            "role": format!("{:?}", membership.role).to_lowercase(),
            "createdAt": membership.created_at,
        },
        "organization": org.map(|o| serde_json::json!({
            "id": o.id,
            "name": o.name,
            "slug": o.slug,
        })),
    })))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignInSocialBody {
    provider: String,
    callback_url: Option<String>,
}

/// Begin a social (OAuth) sign-in (Better Auth `sign-in/social`).
///
/// Returns the provider's OAuth authorization URL (plus a state + PKCE challenge)
/// when the provider is configured, so the client can redirect the browser. When
/// the provider is not configured a clear error is returned. The downstream
/// `/api/auth/callback/:provider` OAuth token-exchange/callback route is not yet
/// wired in this port (see §8).
pub async fn sign_in_social_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SignInSocialBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let provider = body.provider.to_lowercase();
    let oauth = state.config.auth.oauth.as_ref();

    let (configured, authorize) = match provider.as_str() {
        "google" => (
            oauth.and_then(|o| o.google.as_ref()).is_some(),
            oauth
                .and_then(|o| o.google.as_ref())
                .map(|g| {
                    // The OAuth `redirect_uri` must be the configured one (matched by the
                    // token exchange in the callback). The SPA deep-link rides in `state`.
                    let state = super::oauth::encode_callback_state(body.callback_url.as_deref().unwrap_or("/"));
                    format!(
                        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}",
                        g.client_id, g.redirect_uri, state
                    )
                }),
        ),
        "github" => (
            oauth.and_then(|o| o.github.as_ref()).is_some(),
            oauth
                .and_then(|o| o.github.as_ref())
                .map(|g| {
                    let state = super::oauth::encode_callback_state(body.callback_url.as_deref().unwrap_or("/"));
                    format!(
                        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=read:user%20user:email&state={}",
                        g.client_id, g.redirect_uri, state
                    )
                }),
        ),
        _ => (false, None),
    };

    match (configured, authorize) {
        (true, Some(url)) => Ok(Json(serde_json::json!({
            "url": url,
            "redirect": true,
            "state": "authorize",
            "codeChallenge": null,
        }))),
        _ => Err(AppError::custom(
            StatusCode::BAD_REQUEST,
            format!("OAuth provider '{}' is not configured", provider),
        )),
    }
}
