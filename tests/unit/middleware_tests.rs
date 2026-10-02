//! Unit tests for middleware components
//!
//! These tests verify individual middleware components in isolation.

use axum::http::{HeaderMap, HeaderValue};
use cms_middleware::{
    admin_origin::{
        extract_origin_from_url, is_localhost_origin, normalize_origin, AdminOriginConfig,
    },
    observability::{generate_request_id, get_or_generate_request_id, REQUEST_ID_HEADER},
    rate_limit::{RateLimitClient, RateLimitConfig, RateLimitResult, RateLimiter},
    security_headers::{presets, ReferrerPolicy, SecurityHeadersConfig, XFrameOptions},
};
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

// ============================================================================
// Rate Limiting Tests
// ============================================================================

#[test]
fn test_rate_limit_config_validation() {
    // Valid config
    let config = RateLimitConfig::default();
    assert!(config.validate().is_ok());

    // Invalid: zero requests per second
    let config = RateLimitConfig {
        requests_per_second: 0,
        ..Default::default()
    };
    assert!(config.validate().is_err());

    // Invalid: burst size less than requests per second
    let config = RateLimitConfig {
        requests_per_second: 100,
        burst_size: 50,
        ..Default::default()
    };
    assert!(config.validate().is_err());

    // Invalid: zero max tracked clients
    let config = RateLimitConfig {
        max_tracked_clients: 0,
        ..Default::default()
    };
    assert!(config.validate().is_err());

    // Valid: disabled config
    let config = RateLimitConfig {
        enabled: false,
        requests_per_second: 0,
        burst_size: 0,
        max_tracked_clients: 100,
        client_ttl: Duration::from_secs(60),
    };
    // When disabled, rate and burst can be zero
    assert!(config.validate().is_ok());
}

#[test]
fn test_rate_limit_config_to_quota() {
    let config = RateLimitConfig {
        requests_per_second: 100,
        burst_size: 200,
        ..Default::default()
    };

    let quota = config.to_quota();
    // Governor's Quota doesn't expose internal values easily
    // Just verify it can be created
    assert!(quota.per_second().get() == 100);
    assert!(quota.allow_burst().get() == 200);
}

#[test]
fn test_client_identification() {
    use axum::http::request::Parts;
    use std::net::SocketAddr;

    // Test IP extraction
    let mut parts = Parts::new();
    parts
        .extensions
        .insert(axum::extract::Connected::<SocketAddr>::new(
            SocketAddr::new(Ipv4Addr::new(192, 168, 1, 1).into(), 12345),
        ));

    let client = RateLimitClient::from_request(&parts);
    assert!(
        matches!(client, RateLimitClient::Ip(ip) if ip == IpAddr::from(Ipv4Addr::new(192, 168, 1, 1)))
    );

    // Test API key extraction
    let mut parts = Parts::new();
    parts.headers.insert(
        axum::http::header::HeaderName::from_static("x-api-key"),
        HeaderValue::from_static("my-api-key"),
    );

    let client = RateLimitClient::from_request(&parts);
    assert!(matches!(client, RateLimitClient::ApiKey(key) if key == "my-api-key"));

    // Test anonymous fallback
    let parts = Parts::new();
    let client = RateLimitClient::from_request(&parts);
    assert!(matches!(client, RateLimitClient::Anonymous));
}

#[test]
fn test_client_display() {
    let client = RateLimitClient::User("user-123".to_string());
    assert_eq!(client.display(), "user:user-123");

    let client = RateLimitClient::ApiKey("api-key-12345678".to_string());
    assert_eq!(client.display(), "apikey:api-key...");

    let client = RateLimitClient::Ip(IpAddr::from(Ipv4Addr::new(192, 168, 1, 1)));
    assert_eq!(client.display(), "192.168.1.1");

    let client = RateLimitClient::Anonymous;
    assert_eq!(client.display(), "anonymous");
}

#[test]
fn test_rate_limiter_allows_requests() {
    let config = RateLimitConfig {
        requests_per_second: 100,
        burst_size: 10,
        enabled: true,
        max_tracked_clients: 1000,
        client_ttl: Duration::from_secs(60),
    };

    let limiter = RateLimiter::new(config).unwrap();
    let client = RateLimitClient::Anonymous;

    // Should allow requests up to burst size
    for _ in 0..10 {
        let result = limiter.check_rate_limit(client.clone());
        assert!(result.is_allowed());
    }
}

#[test]
fn test_rate_limiter_blocks_after_burst() {
    let config = RateLimitConfig {
        requests_per_second: 100,
        burst_size: 5,
        enabled: true,
        max_tracked_clients: 1000,
        client_ttl: Duration::from_secs(60),
    };

    let limiter = RateLimiter::new(config).unwrap();
    let client = RateLimitClient::Anonymous;

    // Use up burst
    for _ in 0..5 {
        limiter.check_rate_limit(client.clone());
    }

    // Next request should be blocked
    let result = limiter.check_rate_limit(client.clone());
    assert!(!result.is_allowed());
    assert!(result.retry_after().is_some());
}

#[test]
fn test_rate_limiter_disabled() {
    let config = RateLimitConfig {
        enabled: false,
        ..Default::default()
    };

    let limiter = RateLimiter::new(config).unwrap();
    let client = RateLimitClient::Anonymous;

    // All requests should be allowed when disabled
    for _ in 0..100 {
        let result = limiter.check_rate_limit(client.clone());
        assert!(result.is_allowed());
    }
}

#[test]
fn test_rate_limiter_metrics() {
    let config = RateLimitConfig {
        requests_per_second: 100,
        burst_size: 1,
        enabled: true,
        max_tracked_clients: 1000,
        client_ttl: Duration::from_secs(60),
    };

    let limiter = RateLimiter::new(config).unwrap();
    let client = RateLimitClient::Anonymous;

    // Make some requests
    limiter.check_rate_limit(client.clone()); // Allowed
    limiter.check_rate_limit(client.clone()); // Blocked (burst=1)
    limiter.check_rate_limit(client.clone()); // Blocked

    let metrics = limiter.metrics();
    assert_eq!(metrics.total_requests, 3);
    assert_eq!(metrics.allowed_requests, 1);
    assert_eq!(metrics.rejected_requests, 2);
}

#[test]
fn test_rate_limiter_client_isolation() {
    let config = RateLimitConfig {
        requests_per_second: 100,
        burst_size: 1,
        enabled: true,
        max_tracked_clients: 1000,
        client_ttl: Duration::from_secs(60),
    };

    let limiter = RateLimiter::new(config).unwrap();

    let client1 = RateLimitClient::User("user-1".to_string());
    let client2 = RateLimitClient::User("user-2".to_string());

    // Client 1 uses their burst
    limiter.check_rate_limit(client1.clone()); // Allowed
    assert!(!limiter.check_rate_limit(client1.clone()).is_allowed()); // Blocked

    // Client 2 should still be allowed (separate rate limit)
    assert!(limiter.check_rate_limit(client2.clone()).is_allowed());
}

// ============================================================================
// Security Headers Tests
// ============================================================================

#[test]
fn test_security_headers_default_config() {
    let config = SecurityHeadersConfig::default();
    assert!(config.validate().is_ok());
    assert!(config.enable_hsts);
    assert!(config.enable_x_content_type_options);
    assert!(config.enable_x_frame_options);
    assert!(config.enable_x_xss_protection);
    assert!(config.enable_csp);
    assert!(config.enable_referrer_policy);
    assert!(config.enable_permissions_policy);
}

#[test]
fn test_security_headers_api_preset() {
    let config = presets::api();
    assert!(config.validate().is_ok());
    assert!(config.enable_hsts);
    assert!(config.enable_csp);
    assert!(config.csp.contains("default-src 'self'"));
    assert!(config.csp.contains("frame-ancestors 'none'"));
}

#[test]
fn test_security_headers_published_site_preset() {
    let config = presets::published_site();
    assert!(config.validate().is_ok());
    assert!(config.enable_csp);
    // Published sites have more permissive CSP
    assert!(config.csp.contains("'unsafe-inline'"));
}

#[test]
fn test_security_headers_development_preset() {
    let config = presets::development();
    assert!(config.validate().is_ok());
    assert!(!config.enable_hsts);
    assert!(!config.enable_csp);
}

#[test]
fn test_security_headers_invalid_csp() {
    let mut config = SecurityHeadersConfig::default();
    config.csp = "invalid\x00header".to_string();
    assert!(config.validate().is_err());
}

#[test]
fn test_security_headers_zero_hsts_max_age() {
    let mut config = SecurityHeadersConfig::default();
    config.hsts_max_age = 0;
    assert!(config.validate().is_err());
}

#[test]
fn test_x_frame_options_values() {
    assert_eq!(XFrameOptions::Deny.as_str(), "DENY");
    assert_eq!(XFrameOptions::SameOrigin.as_str(), "SAMEORIGIN");
    assert_eq!(
        XFrameOptions::AllowFrom("example.com".to_string()).as_str(),
        "ALLOW-FROM"
    );
}

#[test]
fn test_referrer_policy_values() {
    assert_eq!(ReferrerPolicy::NoReferrer.as_str(), "no-referrer");
    assert_eq!(
        ReferrerPolicy::StrictOriginWhenCrossOrigin.as_str(),
        "strict-origin-when-cross-origin"
    );
    assert_eq!(ReferrerPolicy::UnsafeUrl.as_str(), "unsafe-url");
}

// ============================================================================
// Admin Origin Tests
// ============================================================================

#[test]
fn test_normalize_origin_basic() {
    assert_eq!(
        normalize_origin("https://Example.com").unwrap(),
        "https://example.com"
    );

    assert_eq!(
        normalize_origin("https://example.com/").unwrap(),
        "https://example.com"
    );
}

#[test]
fn test_normalize_origin_default_ports() {
    assert_eq!(
        normalize_origin("https://example.com:443").unwrap(),
        "https://example.com"
    );

    assert_eq!(
        normalize_origin("http://example.com:80").unwrap(),
        "http://example.com"
    );

    // Non-default ports should be kept
    assert_eq!(
        normalize_origin("http://example.com:8080").unwrap(),
        "http://example.com:8080"
    );
}

#[test]
fn test_normalize_origin_localhost() {
    assert_eq!(
        normalize_origin("http://localhost:3000").unwrap(),
        "http://localhost:3000"
    );

    assert_eq!(
        normalize_origin("http://127.0.0.1:8080").unwrap(),
        "http://127.0.0.1:8080"
    );
}

#[test]
fn test_is_localhost_origin() {
    assert!(is_localhost_origin("http://localhost"));
    assert!(is_localhost_origin("http://localhost:3000"));
    assert!(is_localhost_origin("https://127.0.0.1"));
    assert!(is_localhost_origin("http://127.0.0.1:8080"));
    assert!(is_localhost_origin("http://[::1]"));
    assert!(is_localhost_origin("http://0.0.0.0:3000"));

    assert!(!is_localhost_origin("https://example.com"));
}

#[test]
fn test_extract_origin_from_url() {
    assert_eq!(
        extract_origin_from_url("https://example.com/path"),
        Some("example.com".to_string())
    );

    assert_eq!(
        extract_origin_from_url("http://localhost:3000/path"),
        Some("localhost:3000".to_string())
    );

    assert_eq!(
        extract_origin_from_url("example.com/path"),
        Some("example.com".to_string())
    );
}

#[test]
fn test_admin_origin_config_validation() {
    // Valid config
    let config = AdminOriginConfig::default();
    assert!(config.validate().is_ok());

    // Invalid: origin with path
    let mut config = AdminOriginConfig::default();
    config
        .allowed_origins
        .insert("https://example.com/path".to_string());
    assert!(config.validate().is_err());

    // Invalid: origin without scheme
    let mut config = AdminOriginConfig::default();
    config.allowed_origins.insert("example.com".to_string());
    assert!(config.validate().is_err());

    // Invalid: empty origin
    let mut config = AdminOriginConfig::default();
    config.allowed_origins.insert(String::new());
    assert!(config.validate().is_err());
}

#[test]
fn test_admin_origin_config_is_allowed() {
    let config = AdminOriginConfig {
        allowed_origins: {
            let mut set = std::collections::HashSet::new();
            set.insert("https://admin.cms.com".to_string());
            set
        },
        enforce: true,
        allow_localhost: false,
    };

    assert!(config.is_origin_allowed("https://admin.cms.com"));
    assert!(config.is_origin_allowed("https://ADMIN.CMS.COM")); // Case insensitive
    assert!(config.is_origin_allowed("https://admin.cms.com/")); // Trailing slash

    assert!(!config.is_origin_allowed("https://evil.com"));
    assert!(!config.is_origin_allowed("https://admin.cms.com.evil.com"));
}

#[test]
fn test_admin_origin_config_localhost() {
    let config = AdminOriginConfig {
        allowed_origins: std::collections::HashSet::new(),
        enforce: true,
        allow_localhost: true,
    };

    assert!(config.is_origin_allowed("http://localhost:3000"));
    assert!(config.is_origin_allowed("http://127.0.0.1:8080"));

    assert!(!config.is_origin_allowed("https://example.com"));
}

#[test]
fn test_admin_origin_config_disabled() {
    let config = AdminOriginConfig {
        allowed_origins: std::collections::HashSet::new(),
        enforce: false,
        allow_localhost: false,
    };

    // When enforce is false, all origins are allowed
    assert!(config.is_origin_allowed("https://any-origin.com"));
}

// ============================================================================
// Observability Tests
// ============================================================================

#[test]
fn test_generate_request_id() {
    let id1 = generate_request_id();
    let id2 = generate_request_id();

    assert!(!id1.is_empty());
    assert_ne!(id1, id2);

    // Should be valid UUIDs
    assert!(uuid::Uuid::parse_str(&id1).is_ok());
    assert!(uuid::Uuid::parse_str(&id2).is_ok());
}

#[test]
fn test_get_or_generate_request_id() {
    let mut headers = HeaderMap::new();

    // No request ID - should generate
    let id1 = get_or_generate_request_id(&headers);
    assert!(!id1.is_empty());

    // With request ID - should return it
    headers.insert(REQUEST_ID_HEADER, HeaderValue::from_static("test-id-123"));
    let id2 = get_or_generate_request_id(&headers);
    assert_eq!(id2, "test-id-123");
}

#[test]
fn test_request_id_header_constant() {
    assert_eq!(REQUEST_ID_HEADER, "X-Request-ID");
}
