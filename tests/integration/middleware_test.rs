//! Integration tests for middleware components
//!
//! These tests verify that the middleware stack works correctly together.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
    Router,
};
use cms_middleware::{
    app_state::AppState,
    rate_limit::{RateLimitConfig, RateLimitClient, RateLimitResult, create_per_client_rate_limit_layer},
    security_headers::{SecurityHeadersConfig, create_security_headers_layer, presets},
    observability::{ObservabilityConfig, ObservabilityLayer, REQUEST_ID_HEADER},
};
use std::sync::Arc;
use tower::ServiceExt;

/// Create a test router with all middleware
fn create_test_router() -> Router {
    // Create a simple handler
    async fn test_handler() -> &'static str {
        "OK"
    }
    
    let router = Router::new()
        .route("/test", axum::routing::get(test_handler));
    
    // Apply middleware
    let observability_config = ObservabilityConfig::default();
    let router = router.layer(ObservabilityLayer::with_config(observability_config));
    
    // Apply security headers
    let security_config = presets::api();
    let security_layer = create_security_headers_layer(security_config).unwrap();
    let router = router.layer(security_layer);
    
    // Apply rate limiting
    let rate_limit_config = RateLimitConfig {
        requests_per_second: 100,
        burst_size: 10,
        enabled: true,
        max_tracked_clients: 1000,
        client_ttl: std::time::Duration::from_secs(60),
    };
    let rate_limit_layer = create_per_client_rate_limit_layer(rate_limit_config).unwrap();
    router.layer(rate_limit_layer)
}

#[tokio::test]
async fn test_middleware_stack_responds() {
    let router = create_test_router();
    
    let response = router
        .oneshot(Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_rate_limiting_blocks_requests() {
    let rate_limit_config = RateLimitConfig {
        requests_per_second: 1,
        burst_size: 1,
        enabled: true,
        max_tracked_clients: 100,
        client_ttl: std::time::Duration::from_secs(60),
    };
    
    let rate_limit_layer = create_per_client_rate_limit_layer(rate_limit_config).unwrap();
    
    async fn test_handler() -> &'static str {
        "OK"
    }
    
    let router = Router::new()
        .route("/test", axum::routing::get(test_handler))
        .layer(rate_limit_layer);
    
    // First request should succeed
    let response1 = router
        .clone()
        .oneshot(Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response1.status(), StatusCode::OK);
    
    // Second request should be rate limited
    let response2 = router
        .oneshot(Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response2.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn test_security_headers_present() {
    let security_config = presets::api();
    let security_layer = create_security_headers_layer(security_config).unwrap();
    
    async fn test_handler() -> &'static str {
        "OK"
    }
    
    let router = Router::new()
        .route("/test", axum::routing::get(test_handler))
        .layer(security_layer);
    
    let response = router
        .oneshot(Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    let headers = response.headers();
    
    // Check for security headers
    assert!(headers.contains_key("x-content-type-options"));
    assert!(headers.contains_key("x-frame-options"));
    assert!(headers.contains_key("x-xss-protection"));
    assert!(headers.contains_key("content-security-policy"));
    assert!(headers.contains_key("referrer-policy"));
    assert!(headers.contains_key("permissions-policy"));
}

#[tokio::test]
async fn test_observability_adds_request_id() {
    let observability_config = ObservabilityConfig {
        enable_request_id: true,
        ..Default::default()
    };
    
    let observability_layer = ObservabilityLayer::with_config(observability_config);
    
    async fn test_handler() -> &'static str {
        "OK"
    }
    
    let router = Router::new()
        .route("/test", axum::routing::get(test_handler))
        .layer(observability_layer);
    
    let response = router
        .oneshot(Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    let headers = response.headers();
    assert!(headers.contains_key(REQUEST_ID_HEADER));
}

#[tokio::test]
async fn test_observability_adds_response_time() {
    let observability_config = ObservabilityConfig {
        enable_response_time_header: true,
        ..Default::default()
    };
    
    let observability_layer = ObservabilityLayer::with_config(observability_config);
    
    async fn test_handler() -> &'static str {
        "OK"
    }
    
    let router = Router::new()
        .route("/test", axum::routing::get(test_handler))
        .layer(observability_layer);
    
    let response = router
        .oneshot(Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    let headers = response.headers();
    assert!(headers.contains_key("X-Response-Time"));
}

#[tokio::test]
async fn test_disabled_rate_limiting() {
    let rate_limit_config = RateLimitConfig {
        requests_per_second: 1,
        burst_size: 1,
        enabled: false, // Disabled
        max_tracked_clients: 100,
        client_ttl: std::time::Duration::from_secs(60),
    };
    
    let rate_limit_layer = create_per_client_rate_limit_layer(rate_limit_config).unwrap();
    
    async fn test_handler() -> &'static str {
        "OK"
    }
    
    let router = Router::new()
        .route("/test", axum::routing::get(test_handler))
        .layer(rate_limit_layer);
    
    // Multiple requests should all succeed when rate limiting is disabled
    for _ in 0..5 {
        let response = router
            .clone()
            .oneshot(Request::builder()
                .uri("/test")
                .body(Body::empty())
                .unwrap())
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn test_rate_limiting_retry_after_header() {
    let rate_limit_config = RateLimitConfig {
        requests_per_second: 1,
        burst_size: 1,
        enabled: true,
        max_tracked_clients: 100,
        client_ttl: std::time::Duration::from_secs(60),
    };
    
    let rate_limit_layer = create_per_client_rate_limit_layer(rate_limit_config).unwrap();
    
    async fn test_handler() -> &'static str {
        "OK"
    }
    
    let router = Router::new()
        .route("/test", axum::routing::get(test_handler))
        .layer(rate_limit_layer);
    
    // First request succeeds
    router
        .clone()
        .oneshot(Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    // Second request should have Retry-After header
    let response = router
        .oneshot(Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers().contains_key("Retry-After"));
}
