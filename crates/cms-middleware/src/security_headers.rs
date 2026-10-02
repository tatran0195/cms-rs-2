//! Security headers middleware
//!
//! This module provides security headers using Tower's `SetResponseHeaderLayer`.
//!
//! # Production Considerations
//!
//! - CSP is configured appropriately for API responses (allows JSON)
//! - HSTS is enabled by default with 1-year duration
//! - All headers are configurable and can be disabled
//! - Defaults follow security best practices
//!
//! # API vs Site Headers
//!
//! Note: The sites crate has its own security headers for published documentation sites.
//! This middleware is for the API endpoints, which have different requirements:
//! - API responses are JSON, not HTML
//! - API may need to be framed in iframes for embedded content
//! - API needs CORS headers for cross-origin requests

use axum::http::HeaderValue;

/// Security headers configuration with validation
#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    /// Enable HSTS (Strict-Transport-Security)
    pub enable_hsts: bool,
    /// HSTS max age in seconds (0 to disable, recommended: 31536000 = 1 year)
    pub hsts_max_age: u32,
    /// Include subdomains in HSTS
    pub hsts_include_subdomains: bool,
    /// Enable X-Content-Type-Options: nosniff
    pub enable_x_content_type_options: bool,
    /// Enable X-Frame-Options
    pub enable_x_frame_options: bool,
    /// X-Frame-Options value: DENY, SAMEORIGIN, or ALLOW-FROM uri
    pub x_frame_options: XFrameOptions,
    /// Enable X-XSS-Protection
    pub enable_x_xss_protection: bool,
    /// Enable Content-Security-Policy
    pub enable_csp: bool,
    /// CSP directive string
    /// For API: should allow JSON and necessary external resources
    pub csp: String,
    /// Enable Referrer-Policy
    pub enable_referrer_policy: bool,
    /// Referrer-Policy value
    pub referrer_policy: ReferrerPolicy,
    /// Enable Permissions-Policy
    pub enable_permissions_policy: bool,
    /// Permissions-Policy value
    pub permissions_policy: String,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            enable_hsts: true,
            hsts_max_age: 31536000, // 1 year
            hsts_include_subdomains: true,
            enable_x_content_type_options: true,
            enable_x_frame_options: true,
            x_frame_options: XFrameOptions::Deny,
            enable_x_xss_protection: true,
            enable_csp: true,
            // API-appropriate CSP: allows JSON, blocks inline scripts by default
            // This is more restrictive than needed for pure API, but safe
            csp: "default-src 'self'; frame-ancestors 'self'; base-uri 'self'; form-action 'self';"
                .to_string(),
            enable_referrer_policy: true,
            referrer_policy: ReferrerPolicy::StrictOriginWhenCrossOrigin,
            enable_permissions_policy: true,
            permissions_policy: "geolocation=(), microphone=(), camera=(), payment=()".to_string(),
        }
    }
}

impl SecurityHeadersConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.enable_hsts && self.hsts_max_age == 0 {
            return Err("HSTS max age cannot be 0 when HSTS is enabled".into());
        }

        // Validate CSP can be parsed as a header value
        if self.enable_csp && HeaderValue::from_str(&self.csp).is_err() {
            return Err("Invalid CSP header value".into());
        }

        // Validate Referrer-Policy
        if self.enable_referrer_policy
            && HeaderValue::from_str(self.referrer_policy.as_str()).is_err()
        {
            return Err("Invalid Referrer-Policy header value".into());
        }

        // Validate Permissions-Policy
        if self.enable_permissions_policy
            && HeaderValue::from_str(&self.permissions_policy).is_err()
        {
            return Err("Invalid Permissions-Policy header value".into());
        }

        Ok(())
    }
}

/// X-Frame-Options values
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XFrameOptions {
    /// Prevent framing entirely
    Deny,
    /// Allow framing only from same origin
    SameOrigin,
    /// Allow framing from specific URI (not widely supported)
    AllowFrom(String),
}

impl XFrameOptions {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Deny => "DENY",
            Self::SameOrigin => "SAMEORIGIN",
            Self::AllowFrom(_) => "ALLOW-FROM", // Note: actual URI would be needed
        }
    }
}

impl std::fmt::Display for XFrameOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Referrer-Policy values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferrerPolicy {
    NoReferrer,
    NoReferrerWhenDowngrade,
    SameOrigin,
    Origin,
    StrictOrigin,
    OriginWhenCrossOrigin,
    StrictOriginWhenCrossOrigin,
    UnsafeUrl,
}

impl ReferrerPolicy {
    fn as_str(&self) -> &'static str {
        match self {
            Self::NoReferrer => "no-referrer",
            Self::NoReferrerWhenDowngrade => "no-referrer-when-downgrade",
            Self::SameOrigin => "same-origin",
            Self::Origin => "origin",
            Self::StrictOrigin => "strict-origin",
            Self::OriginWhenCrossOrigin => "origin-when-cross-origin",
            Self::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            Self::UnsafeUrl => "unsafe-url",
        }
    }
}

impl std::fmt::Display for ReferrerPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Create security headers middleware
///
/// Returns a middleware that adds security headers to all responses.
pub fn create_security_headers_layer(
    config: SecurityHeadersConfig,
) -> Result<SecurityHeadersMiddleware, String> {
    SecurityHeadersMiddleware::new(config)
}

/// Predefined security header configurations
pub mod presets {
    use super::*;

    /// Security headers for API endpoints
    ///
    /// - Allows JSON responses
    /// - Blocks framing (can be relaxed if needed)
    /// - Strict CSP for API
    pub fn api() -> SecurityHeadersConfig {
        SecurityHeadersConfig {
            enable_hsts: true,
            hsts_max_age: 31536000,
            hsts_include_subdomains: true,
            enable_x_content_type_options: true,
            enable_x_frame_options: true,
            x_frame_options: XFrameOptions::Deny,
            enable_x_xss_protection: true,
            enable_csp: true,
            // API CSP: very restrictive, only self
            csp: "default-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self';"
                .to_string(),
            enable_referrer_policy: true,
            referrer_policy: ReferrerPolicy::StrictOriginWhenCrossOrigin,
            enable_permissions_policy: true,
            permissions_policy: "geolocation=(), microphone=(), camera=(), payment=()".to_string(),
        }
    }

    /// Security headers for published documentation sites
    ///
    /// - More permissive CSP for markdown content
    /// - Allows framing for embedded content
    pub fn published_site() -> SecurityHeadersConfig {
        SecurityHeadersConfig {
            enable_hsts: true,
            hsts_max_age: 31536000,
            hsts_include_subdomains: true,
            enable_x_content_type_options: true,
            enable_x_frame_options: false, // Allow framing for embedded docs
            x_frame_options: XFrameOptions::SameOrigin,
            enable_x_xss_protection: true,
            enable_csp: true,
            // More permissive CSP for published sites
            csp: "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' \
                  'unsafe-inline'; img-src 'self' data: https:; font-src 'self' https:; \
                  connect-src 'self' https:; frame-src https:; object-src 'none'; base-uri \
                  'self'; form-action 'self';"
                .to_string(),
            enable_referrer_policy: true,
            referrer_policy: ReferrerPolicy::StrictOriginWhenCrossOrigin,
            enable_permissions_policy: true,
            permissions_policy: "geolocation=(), microphone=(), camera=()".to_string(),
        }
    }

    /// Minimal security headers for development
    pub fn development() -> SecurityHeadersConfig {
        SecurityHeadersConfig {
            enable_hsts: false, // Don't enforce HSTS in dev
            hsts_max_age: 0,
            hsts_include_subdomains: false,
            enable_x_content_type_options: true,
            enable_x_frame_options: false,
            x_frame_options: XFrameOptions::Deny,
            enable_x_xss_protection: true,
            enable_csp: false, // CSP can break dev tools
            csp: String::new(),
            enable_referrer_policy: true,
            referrer_policy: ReferrerPolicy::NoReferrerWhenDowngrade,
            enable_permissions_policy: false,
            permissions_policy: String::new(),
        }
    }
}

/// Security headers middleware (legacy, kept for compatibility)
///
/// Note: This is kept for backward compatibility. New code should use
/// `create_security_headers_layer` directly with Tower.
#[derive(Debug, Clone)]
pub struct SecurityHeadersMiddleware {
    config: SecurityHeadersConfig,
}

impl SecurityHeadersMiddleware {
    pub fn new(config: SecurityHeadersConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Apply security headers to a response
    pub fn apply_security_headers(
        &self,
        mut response: axum::response::Response,
    ) -> axum::response::Response {
        use axum::http::HeaderValue;

        // HSTS
        if self.config.enable_hsts {
            let mut hsts_value = format!("max-age={}", self.config.hsts_max_age);
            if self.config.hsts_include_subdomains {
                hsts_value.push_str("; includeSubDomains");
            }
            if let Ok(header_value) = HeaderValue::from_str(&hsts_value) {
                response
                    .headers_mut()
                    .insert("strict-transport-security", header_value);
            }
        }

        // X-Content-Type-Options
        if self.config.enable_x_content_type_options {
            response.headers_mut().insert(
                "x-content-type-options",
                HeaderValue::from_static("nosniff"),
            );
        }

        // X-Frame-Options
        if self.config.enable_x_frame_options {
            let value = match &self.config.x_frame_options {
                XFrameOptions::Deny => "DENY",
                XFrameOptions::SameOrigin => "SAMEORIGIN",
                XFrameOptions::AllowFrom(uri) => &format!("ALLOW-FROM {}", uri),
            };
            if let Ok(header_value) = HeaderValue::from_str(value) {
                response
                    .headers_mut()
                    .insert("x-frame-options", header_value);
            }
        }

        // X-XSS-Protection
        if self.config.enable_x_xss_protection {
            response.headers_mut().insert(
                "x-xss-protection",
                HeaderValue::from_static("1; mode=block"),
            );
        }

        // Content-Security-Policy
        if self.config.enable_csp {
            if let Ok(header_value) = HeaderValue::from_str(&self.config.csp) {
                response
                    .headers_mut()
                    .insert("content-security-policy", header_value);
            }
        }

        // Referrer-Policy
        if self.config.enable_referrer_policy {
            if let Ok(header_value) = HeaderValue::from_str(self.config.referrer_policy.as_str()) {
                response
                    .headers_mut()
                    .insert("referrer-policy", header_value);
            }
        }

        // Permissions-Policy
        if self.config.enable_permissions_policy {
            if let Ok(header_value) = HeaderValue::from_str(&self.config.permissions_policy) {
                response
                    .headers_mut()
                    .insert("permissions-policy", header_value);
            }
        }

        response
    }
}

/// Type alias for convenience
pub type SecurityHeadersLayer = SecurityHeadersMiddleware;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validation() {
        let config = SecurityHeadersConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_api_preset() {
        let config = presets::api();
        assert!(config.validate().is_ok());
        assert!(config.enable_hsts);
        assert!(config.enable_csp);
        assert!(config.csp.contains("default-src 'self'"));
    }

    #[test]
    fn test_development_preset() {
        let config = presets::development();
        assert!(config.validate().is_ok());
        assert!(!config.enable_hsts);
        assert!(!config.enable_csp);
    }
}
