//! CMS Middleware
//!
//! This crate contains cross-cutting Tower layers for the CMS API.
//! Middleware is organized as Tower layers.
//!
//! # Production-Ready Features
//!
//! All middleware components use established ecosystem crates:
//! - Rate limiting: `governor` crate (battle-tested, widely adopted)
//! - Metrics: `metrics` crate (lightweight, maintained by tracing authors)
//! - Security headers: `tower-http` with security-headers feature
//! - Admin origin: Custom domain-specific logic (CSRF protection)
//!
//! # Architecture Decisions
//!
//! ## Rate Limiting with Governor
//!
//! We use the `governor` crate instead of custom implementation because:
//! - Battle-tested in production (2M+ downloads)
//! - Handles edge cases (clock skew, timing attacks, etc.)
//! - Provides multiple algorithms (token bucket, fixed window, sliding window)
//! - Thread-safe and designed for async
//! - Standard interface with state inspection
//!
//! ## Metrics with metrics crate
//!
//! We use the `metrics` crate instead of custom implementation because:
//! - Extremely lightweight (~5KB compiled)
//! - Standard interface that works with various backends
//! - Maintained by the same team as `tracing`
//! - Zero-cost abstractions
//! - Future extensibility (can add Prometheus without code changes)
//!
//! ## Security Headers with tower-http
//!
//! We use `tower-http`'s built-in security headers feature:
//! - Already a dependency
//! - Well-tested
//! - Configurable

pub mod admin_origin;
pub mod app_state;
pub mod locale;
pub mod observability;
pub mod rate_limit;
pub mod security_headers;

// Re-export commonly used types
pub use admin_origin::{
    extract_origin_from_request, normalize_origin, AdminOriginConfig, AdminOriginExtractor,
    AdminOriginLayer, AdminOriginRejection, AdminOriginValidation,
};
pub use app_state::AppState;
pub use locale::{Locale, LocaleExtractor, LocaleLayer, SetLocaleMiddleware};
pub use observability::{
    add_request_id_to_error, ensure_request_id, generate_request_id, get_or_generate_request_id,
    init_tracing, init_tracing_json, ObservabilityConfig, ObservabilityLayer, ObservabilityService,
    REQUEST_ID_HEADER, RESPONSE_TIME_HEADER,
};
pub use rate_limit::{
    RateLimitClient, RateLimitConfig, RateLimitLayer, RateLimitMetrics, RateLimitMiddleware,
    RateLimitResult, RateLimiter,
};
pub use security_headers::{
    presets, ReferrerPolicy, SecurityHeadersConfig, SecurityHeadersLayer,
    SecurityHeadersMiddleware, XFrameOptions,
};
