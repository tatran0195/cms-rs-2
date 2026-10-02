//! API middleware
//!
//! This module applies middleware layers to the API router.
//!
//! # Production Considerations
//!
//! - Rate limiting is per-client (not global) to prevent one user blocking all users
//! - Security headers are configurable with appropriate defaults for API endpoints
//! - Observability includes request tracing, logging, timing, and metrics
//! - All middleware configurations are validated before use

use std::sync::Arc;

use axum::Router;
use cms_middleware::{
    app_state::AppState,
    observability::{
        init_observability, ObservabilityConfig, ObservabilityLayer, REQUEST_ID_HEADER,
    },
    rate_limit::{create_per_client_rate_limit_layer, RateLimitConfig},
    security_headers::{create_security_headers_layer, SecurityHeadersConfig},
};

use crate::create_api_router;

/// Apply all middleware to the API router
///
/// Uses configuration from AppState to configure middleware.
pub fn apply_api_middleware(router: Router, state: Arc<AppState>) -> Router {
    let mut router = router;

    // Initialize observability (only once)
    init_observability();

    // Apply observability layer with request tracing, logging, timing, and metrics
    let observability_config = ObservabilityConfig::default();
    router = router.layer(ObservabilityLayer::with_config(observability_config));

    // Apply security headers from config
    let security_headers_config = SecurityHeadersConfig {
        enable_hsts: state.config.security_headers.enable_hsts,
        hsts_max_age: state.config.security_headers.hsts_max_age,
        hsts_include_subdomains: state.config.security_headers.hsts_include_subdomains,
        enable_x_content_type_options: state.config.security_headers.enable_x_content_type_options,
        enable_x_frame_options: state.config.security_headers.enable_x_frame_options,
        x_frame_options: match state.config.security_headers.x_frame_options.as_str() {
            "DENY" => cms_middleware::security_headers::XFrameOptions::Deny,
            "SAMEORIGIN" => cms_middleware::security_headers::XFrameOptions::SameOrigin,
            s if s.starts_with("ALLOW-FROM") => {
                let uri = s.trim_start_matches("ALLOW-FROM").trim();
                cms_middleware::security_headers::XFrameOptions::AllowFrom(uri.to_string())
            }
            _ => cms_middleware::security_headers::XFrameOptions::Deny,
        },
        enable_x_xss_protection: state.config.security_headers.enable_x_xss_protection,
        enable_csp: state.config.security_headers.enable_csp,
        csp: state.config.security_headers.csp.clone(),
        enable_referrer_policy: state.config.security_headers.enable_referrer_policy,
        referrer_policy: match state.config.security_headers.referrer_policy.as_str() {
            "no-referrer" => cms_middleware::security_headers::ReferrerPolicy::NoReferrer,
            "no-referrer-when-downgrade" => {
                cms_middleware::security_headers::ReferrerPolicy::NoReferrerWhenDowngrade
            }
            "same-origin" => cms_middleware::security_headers::ReferrerPolicy::SameOrigin,
            "origin" => cms_middleware::security_headers::ReferrerPolicy::Origin,
            "strict-origin" => cms_middleware::security_headers::ReferrerPolicy::StrictOrigin,
            "origin-when-cross-origin" => {
                cms_middleware::security_headers::ReferrerPolicy::OriginWhenCrossOrigin
            }
            "strict-origin-when-cross-origin" => {
                cms_middleware::security_headers::ReferrerPolicy::StrictOriginWhenCrossOrigin
            }
            "unsafe-url" => cms_middleware::security_headers::ReferrerPolicy::UnsafeUrl,
            _ => cms_middleware::security_headers::ReferrerPolicy::StrictOriginWhenCrossOrigin,
        },
        enable_permissions_policy: state.config.security_headers.enable_permissions_policy,
        permissions_policy: state.config.security_headers.permissions_policy.clone(),
    };

    if let Ok(sec_mw) =
        cms_middleware::security_headers::SecurityHeadersMiddleware::new(security_headers_config)
    {
        router = router.layer(axum::middleware::map_response(
            move |res: axum::response::Response| {
                let mw = sec_mw.clone();
                async move { mw.apply_security_headers(res) }
            },
        ));
    }

    // Apply rate limiting from config
    let rate_limit_config = RateLimitConfig {
        requests_per_second: state.config.rate_limit.requests_per_second,
        burst_size: state.config.rate_limit.burst_size,
        enabled: state.config.rate_limit.enabled,
        max_tracked_clients: state.config.rate_limit.max_tracked_clients,
        client_ttl: std::time::Duration::from_secs(state.config.rate_limit.client_ttl_secs),
    };

    if rate_limit_config.enabled {
        if let Ok(limiter) = cms_middleware::rate_limit::RateLimiter::new(rate_limit_config) {
            let limiter = Arc::new(limiter);
            router = router.layer(axum::middleware::from_fn(
                move |req: axum::extract::Request, next: axum::middleware::Next| {
                    let limiter = limiter.clone();
                    async move {
                        let (parts, body) = req.into_parts();
                        let client =
                            cms_middleware::rate_limit::RateLimitClient::from_request(&parts);
                        let req = axum::extract::Request::from_parts(parts, body);
                        match limiter.check_rate_limit(client) {
                            cms_middleware::rate_limit::RateLimitResult::Allowed => {
                                next.run(req).await
                            }
                            cms_middleware::rate_limit::RateLimitResult::Rejected {
                                retry_after,
                            } => cms_middleware::rate_limit::rate_limit_exceeded_response(
                                retry_after,
                            ),
                        }
                    }
                },
            ));
        }
    }

    // Add AppState to extensions for extractors
    router = router.layer(axum::Extension(state));

    router
}

/// Create API router with middleware applied
pub fn create_api_router_with_middleware(state: Arc<AppState>) -> Router {
    let router = create_api_router(state.clone());
    apply_api_middleware(router, state)
}

/// Create router with CORS support
pub fn create_api_router_with_cors(state: Arc<AppState>) -> Router {
    use axum::http::{header, HeaderName, Method};
    use tower_http::cors::{Any, CorsLayer};

    let router = create_api_router(state.clone());

    // Apply CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
            Method::HEAD,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            HeaderName::from_static("x-requested-with"),
            HeaderName::from_static("x-api-key"),
        ])
        .expose_headers([
            header::CONTENT_TYPE,
            header::CONTENT_LENGTH,
            HeaderName::from_static("x-request-id"),
        ])
        .max_age(std::time::Duration::from_secs(86400));

    let router = router.layer(cors);

    // Apply other middleware
    apply_api_middleware(router, state)
}

/// Compression middleware
pub fn create_api_router_with_compression(state: Arc<AppState>) -> Router {
    use tower_http::compression::CompressionLayer;

    let router = create_api_router(state.clone());

    // Apply compression
    let compression = CompressionLayer::new().br(true).gzip(true).deflate(true);

    let router = router.layer(compression);

    // Apply other middleware
    apply_api_middleware(router, state)
}

/// Full API router with all middleware
pub fn create_full_api_router(state: Arc<AppState>) -> Router {
    use axum::http::{header, HeaderName, Method};
    use tower_http::{
        compression::CompressionLayer,
        cors::{Any, CorsLayer},
    };

    let router = create_api_router(state.clone());

    // Apply CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
            Method::HEAD,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            HeaderName::from_static("x-requested-with"),
            HeaderName::from_static("x-api-key"),
        ])
        .expose_headers([
            header::CONTENT_TYPE,
            header::CONTENT_LENGTH,
            HeaderName::from_static("x-request-id"),
        ])
        .max_age(std::time::Duration::from_secs(86400));

    let router = router.layer(cors);

    // Apply compression
    let compression = CompressionLayer::new().br(true).gzip(true).deflate(true);

    let router = router.layer(compression);

    // Apply other middleware
    apply_api_middleware(router, state)
}
