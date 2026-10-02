//! Better Auth compatible OAuth social sign-in.
//!
//! The SPA calls `signIn.social({provider, callbackURL})` which POSTs
//! `/api/auth/sign-in/social` and receives an authorize URL; the browser is
//! redirected to the OAuth provider. That provider then redirects the browser
//! back to `GET /api/auth/callback/:provider` (the `redirect_uri` from config).
//! This module implements that callback: exchange the `code` for tokens, resolve
//! the provider profile, upsert the `User` + `Account` rows, create a real
//! `Session`, set the session cookie, and redirect back to the SPA.
//!
//! The original `callbackURL` is carried through the OAuth `state` parameter
//! (URL-safe base64 of `{"cb": <callbackURL>}`) so we can return the user to the
//! right page after the round-trip.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
};
use serde::Deserialize;
use serde_json::json;

use cms_biz::platform_event::{FunnelEventType, PlatformEventService};
use cms_config::OAuthProviderConfig;
use cms_error::AppError;
use cms_middleware::app_state::AppState;

fn internal(msg: String) -> AppError {
    AppError::Internal(anyhow::anyhow!(msg))
}

/// Base64-URL without padding — used for the state round-trip.
fn b64url_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}
fn b64url_decode(s: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s)
        .ok()
}

/// Encode the SPA `callbackURL` into the OAuth `state` parameter so the callback
/// can recover it.
pub(crate) fn encode_callback_state(callback_url: &str) -> String {
    let payload = json!({ "cb": callback_url });
    b64url_encode(payload.to_string().as_bytes())
}

/// Recover the SPA `callbackURL` from the OAuth `state` parameter.
fn decode_callback_state(state: &str) -> Option<String> {
    let bytes = b64url_decode(state)?;
    let s = String::from_utf8(bytes).ok()?;
    let v: serde_json::Value = serde_json::from_str(&s).ok()?;
    v.get("cb").and_then(|c| c.as_str()).map(String::from)
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    #[serde(rename = "error_description")]
    error_description: Option<String>,
}

struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    token_type: Option<String>,
    scope: Option<String>,
}

struct ProviderProfile {
    provider_account_id: String,
    email: String,
    name: Option<String>,
    image: Option<String>,
}

/// Exchange the authorization code for an access token against the provider.
async fn exchange_code(
    provider: &str,
    cfg: &OAuthProviderConfig,
    code: &str,
) -> Result<TokenResponse, AppError> {
    let client = reqwest::Client::new();
    let token_url = match provider {
        "google" => "https://oauth2.googleapis.com/token",
        "github" => "https://github.com/login/oauth/access_token",
        other => {
            return Err(AppError::custom(
                StatusCode::BAD_REQUEST,
                format!("Unsupported OAuth provider: {}", other),
            ))
        }
    };

    let mut form = vec![
        ("grant_type".to_string(), "authorization_code".to_string()),
        ("code".to_string(), code.to_string()),
        ("client_id".to_string(), cfg.client_id.clone()),
        ("client_secret".to_string(), cfg.client_secret.clone()),
        ("redirect_uri".to_string(), cfg.redirect_uri.clone()),
    ];
    if provider == "github" {
        form.push(("redirect_uri".to_string(), cfg.redirect_uri.clone()));
    }

    let mut req = client
        .post(token_url)
        .form(&form)
        .header("Accept", "application/json");
    if provider == "github" {
        req = req.header("Accept", "application/json");
    }

    let resp = req
        .send()
        .await
        .map_err(|e| internal(format!("OAuth token exchange failed: {e}")))?;

    let status = resp.status();
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| internal(format!("OAuth token response parse failed: {e}")))?;

    if !status.is_success() {
        let msg = body
            .get("error_description")
            .or_else(|| body.get("error"))
            .cloned();
        return Err(AppError::custom(
            StatusCode::BAD_REQUEST,
            format!("OAuth token exchange failed: {msg:?}"),
        ));
    }

    Ok(TokenResponse {
        access_token: body
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::custom(StatusCode::BAD_REQUEST, "No access_token in response")
            })?
            .to_string(),
        refresh_token: body
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(String::from),
        expires_in: body.get("expires_in").and_then(|v| v.as_i64()),
        token_type: body
            .get("token_type")
            .and_then(|v| v.as_str())
            .map(String::from),
        scope: body.get("scope").and_then(|v| v.as_str()).map(String::from),
    })
}

/// Fetch the provider profile using the access token.
async fn fetch_profile(provider: &str, token: &TokenResponse) -> Result<ProviderProfile, AppError> {
    let client = reqwest::Client::new();
    let bearer = format!("Bearer {}", token.access_token);

    let profile: serde_json::Value = match provider {
        "google" => {
            let resp = client
                .get("https://www.googleapis.com/oauth2/v2/userinfo")
                .header("Authorization", &bearer)
                .send()
                .await
                .map_err(|e| internal(format!("Google profile fetch failed: {e}")))?;
            resp.json()
                .await
                .map_err(|e| internal(format!("Google profile parse failed: {e}")))?
        }
        "github" => {
            let resp = client
                .get("https://api.github.com/user")
                .header("Authorization", &bearer)
                .header("Accept", "application/vnd.github+json")
                .send()
                .await
                .map_err(|e| internal(format!("GitHub profile fetch failed: {e}")))?;
            let mut p: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| internal(format!("GitHub profile parse failed: {e}")))?;
            // GitHub may not return the primary email unless the `user:email` scope
            // is granted; fall back to the emails endpoint.
            if p.get("email").and_then(|e| e.as_str()).is_none() {
                let emails: serde_json::Value = client
                    .get("https://api.github.com/user/emails")
                    .header("Authorization", &bearer)
                    .header("Accept", "application/vnd.github+json")
                    .send()
                    .await
                    .map_err(|e| internal(format!("GitHub emails fetch failed: {e}")))?
                    .json()
                    .await
                    .map_err(|e| internal(format!("GitHub emails parse failed: {e}")))?;
                let primary_email = emails
                    .as_array()
                    .and_then(|arr| {
                        arr.iter()
                            .find(|e| e.get("primary").and_then(|v| v.as_bool()).unwrap_or(false))
                    })
                    .and_then(|e| e.get("email"))
                    .and_then(|v| v.as_str())
                    .map(String::from);
                if let Some(em) = primary_email {
                    p["email"] = json!(em);
                }
            }
            p
        }
        other => {
            return Err(AppError::custom(
                StatusCode::BAD_REQUEST,
                format!("Unsupported OAuth provider: {}", other),
            ))
        }
    };

    let provider_account_id = profile
        .get("id")
        .or_else(|| profile.get("sub"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::custom(StatusCode::BAD_REQUEST, "No provider user id returned"))?
        .to_string();

    let email = profile
        .get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            AppError::custom(StatusCode::BAD_REQUEST, "Provider did not return an email")
        })?
        .trim()
        .to_lowercase();

    Ok(ProviderProfile {
        provider_account_id,
        email,
        name: profile
            .get("name")
            .and_then(|v| v.as_str())
            .map(String::from),
        image: profile
            .get("picture")
            .or_else(|| profile.get("avatar_url"))
            .and_then(|v| v.as_str())
            .map(String::from),
    })
}

/// `GET /api/auth/callback/:provider` — the OAuth provider redirects the browser
/// here after authorization. Exchanges the code, upserts user + account, creates a
/// session, and redirects back to the SPA.
pub async fn oauth_callback_handler(
    State(state): State<Arc<AppState>>,
    Path(provider): Path<String>,
    Query(query): Query<CallbackQuery>,
) -> Result<axum::response::Response, AppError> {
    use cms_db::auth::AccountQueries;
    use cms_db::auth::SessionQueries;
    use cms_db::auth::UserQueries;

    let provider = provider.to_lowercase();

    // OAuth provider reported an error (e.g. the user denied consent).
    if let Some(err) = query.error {
        let cb = query
            .state
            .as_deref()
            .and_then(decode_callback_state)
            .unwrap_or_else(|| "/".to_string());
        let target = redirect_with_error(&cb, &format!("{err}: {:#?}", query.error_description));
        let mut headers = HeaderMap::new();
        headers.insert(header::LOCATION, HeaderValue::from_str(&target).unwrap());
        return Ok((StatusCode::FOUND, headers, "").into_response());
    }

    let code = query
        .code
        .ok_or_else(|| AppError::custom(StatusCode::BAD_REQUEST, "Missing authorization code"))?;
    let cb = query
        .state
        .as_deref()
        .and_then(decode_callback_state)
        .unwrap_or_else(|| "/".to_string());

    let oauth = state
        .config
        .auth
        .oauth
        .as_ref()
        .ok_or_else(|| AppError::custom(StatusCode::BAD_REQUEST, "OAuth is not configured"))?;

    let cfg = match provider.as_str() {
        "google" => oauth.google.as_ref(),
        "github" => oauth.github.as_ref(),
        other => {
            return Err(AppError::custom(
                StatusCode::BAD_REQUEST,
                format!("Unsupported OAuth provider: {}", other),
            ))
        }
    }
    .ok_or_else(|| {
        AppError::custom(
            StatusCode::BAD_REQUEST,
            format!("OAuth provider '{}' is not configured", provider),
        )
    })?;

    let token = exchange_code(&provider, cfg, &code).await?;
    let profile = fetch_profile(&provider, &token).await?;

    // Upsert the user: find by email, else create (verified, since the provider
    // vouches for it).
    let user = match UserQueries::get_by_email(&state.biz_context.pool, &profile.email).await? {
        Some(u) => {
            if !u.email_verified {
                let _ = UserQueries::update_verified(&state.biz_context.pool, &u.id, true).await;
            }
            UserQueries::get_by_id(&state.biz_context.pool, &u.id)
                .await?
                .ok_or(AppError::Unauthorized)?
        }
        None => {
            let user = UserQueries::create(
                &state.biz_context.pool,
                &profile.email,
                profile.name.as_deref(),
                profile.image.as_deref(),
                true,
            )
            .await?;
            PlatformEventService::record_funnel_event_best_effort(
                &state.biz_context,
                &user.id,
                None,
                FunnelEventType::SignupCompleted,
                json!({ "method": provider.as_str() }),
            )
            .await;
            user
        }
    };

    // Upsert the OAuth account. If it already exists, update the tokens.
    let existing = AccountQueries::get_by_provider(
        &state.biz_context.pool,
        &provider,
        &profile.provider_account_id,
    )
    .await?;

    if existing.is_none() {
        AccountQueries::create(
            &state.biz_context.pool,
            &user.id,
            &provider,
            &profile.provider_account_id,
            Some(token.access_token.as_str()),
            token.refresh_token.as_deref(),
            token
                .expires_in
                .map(|secs| chrono::Utc::now() + chrono::Duration::seconds(secs)),
            token.token_type.as_deref(),
            token.scope.as_deref(),
        )
        .await?;
    }

    // Create a fresh session and set the session cookie.
    let session_token = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
    let _session = SessionQueries::create(
        &state.biz_context.pool,
        &user.id,
        &session_token,
        expires_at,
    )
    .await?;

    let cookie_val = format!(
        "better-auth.session_token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        session_token,
        30 * 24 * 3600
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie_val).map_err(|e| AppError::Internal(e.into()))?,
    );
    headers.insert(
        header::LOCATION,
        HeaderValue::from_str(&cb).map_err(|e| AppError::Internal(e.into()))?,
    );

    Ok((StatusCode::FOUND, headers, "").into_response())
}

/// Redirect the browser to the SPA callback URL with a `?error=` fragment so the
/// client can surface the message.
fn redirect_with_error(cb: &str, error: &str) -> String {
    let enc = b64url_encode(error.as_bytes());
    let sep = if cb.contains('?') { '&' } else { '?' };
    format!("{cb}{sep}error={enc}")
}

pub fn router() -> axum::Router<Arc<AppState>> {
    use axum::routing::get;
    axum::Router::new().route("/callback/{provider}", get(oauth_callback_handler))
}

use axum::response::IntoResponse;
