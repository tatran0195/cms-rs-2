//! Admin origin isolation middleware
//!
//! This module provides middleware to enforce admin origin isolation.
//! This prevents CSRF attacks by ensuring admin requests come from trusted origins.
//!
//! # Security Notes
//!
//! - **DO NOT** trust the Referer header for security decisions (CSRF vulnerability)
//! - Only the Origin header is used for validation
//! - Origin normalization is applied (lowercase, trailing slash removal)
//! - All origins must be explicitly allowed in configuration
//!
//! # Deployment Considerations
//!
//! For single-machine AWS Windows deployment, this provides basic CSRF protection.
//! For enhanced security in production, consider:
//! - Adding CSRF tokens to admin forms
//! - Using SameSite cookies
//! - Implementing double-submit cookie pattern

use std::{collections::HashSet, sync::Arc};

use axum::{
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
};

/// Admin origin configuration with validation
#[derive(Debug, Clone)]
pub struct AdminOriginConfig {
    /// Allowed admin origins (normalized: lowercase, no trailing slash)
    pub allowed_origins: HashSet<String>,
    /// Whether to enforce origin checking (disable for testing only)
    pub enforce: bool,
    /// Whether to allow localhost for development
    pub allow_localhost: bool,
}

impl Default for AdminOriginConfig {
    fn default() -> Self {
        let mut origins = HashSet::new();
        origins.insert("https://admin.cms.com".to_string());
        origins.insert("https://app.cms.com".to_string());
        origins.insert("https://cms.com".to_string());

        Self {
            allowed_origins: origins,
            enforce: true,
            allow_localhost: true,
        }
    }
}

impl AdminOriginConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate all origins are valid URLs
        for origin in &self.allowed_origins {
            if origin.is_empty() {
                return Err("Allowed origins cannot be empty".to_string());
            }

            // Validate origin format: scheme://host[:port]
            // Must start with http:// or https://
            if !origin.starts_with("http://") && !origin.starts_with("https://") {
                return Err(format!(
                    "Invalid origin format: {}. Must start with http:// or https://",
                    origin
                ));
            }

            // Must not contain path or query
            let without_scheme = origin.split("://").nth(1).unwrap_or("");
            if without_scheme.contains('/')
                || without_scheme.contains('?')
                || without_scheme.contains('#')
            {
                return Err(format!(
                    "Invalid origin: {}. Must not contain path, query, or fragment",
                    origin
                ));
            }
        }

        Ok(())
    }

    /// Add an allowed origin (automatically normalized)
    pub fn add_origin(&mut self, origin: String) -> Result<(), String> {
        let normalized = normalize_origin(&origin)?;
        self.allowed_origins.insert(normalized);
        Ok(())
    }

    /// Check if an origin is allowed
    ///
    /// # Security
    /// - Only validates against explicitly allowed origins
    /// - Does NOT trust Referer header (CSRF vulnerability)
    /// - Normalizes origins before comparison
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        if !self.enforce {
            return true;
        }

        let normalized_origin = match normalize_origin(origin) {
            Ok(o) => o,
            Err(_) => return false,
        };

        // Allow localhost for development
        if self.allow_localhost && is_localhost_origin(&normalized_origin) {
            return true;
        }

        self.allowed_origins.contains(&normalized_origin)
    }
}

/// Normalize an origin string for consistent comparison
///
/// - Converts to lowercase
/// - Removes trailing slash
/// - Removes default ports (80 for http, 443 for https)
///
/// # Examples
/// - `https://Example.com:443/` -> `https://example.com`
/// - `http://localhost:8080` -> `http://localhost:8080` (non-default port kept)
pub fn normalize_origin(origin: &str) -> Result<String, String> {
    if origin.is_empty() {
        return Err("Origin cannot be empty".to_string());
    }

    let origin_lower = origin.to_lowercase();

    // Remove trailing slash
    let origin_trimmed = origin_lower.trim_end_matches('/');

    // Parse scheme and host
    let (scheme, host_port) = match origin_trimmed.split_once("://") {
        Some((scheme, host_port)) => (scheme, host_port),
        None => {
            return Err(format!(
                "Invalid origin: missing scheme: {}",
                origin_trimmed
            ))
        }
    };

    // Remove default ports
    let host = if let Some((host, port)) = host_port.split_once(':') {
        match (scheme, port) {
            ("http", "80") | ("https", "443") => host.to_string(),
            _ => format!("{}:{}", host, port),
        }
    } else {
        host_port.to_string()
    };

    Ok(format!("{}://{}", scheme, host))
}

/// Check if an origin is localhost
fn is_localhost_origin(origin: &str) -> bool {
    // Check for localhost
    if origin.contains("localhost") {
        return true;
    }

    // Check for 127.0.0.1
    if origin.contains("127.0.0.1") {
        return true;
    }

    // Check for ::1 (IPv6 localhost)
    if origin.contains("::1") {
        return true;
    }

    // Check for 0.0.0.0
    if origin.contains("0.0.0.0") {
        return true;
    }

    false
}

/// Admin origin validation result
#[derive(Debug, Clone)]
pub struct AdminOriginValidation {
    /// The validated origin (normalized)
    pub origin: Option<String>,
    /// Whether the origin is allowed
    pub is_allowed: bool,
}

/// Admin origin extractor
///
/// Extracts and validates the Origin header from the request.
///
/// # Security
/// - Only uses the Origin header (not Referer)
/// - Returns Forbidden if origin is not allowed
/// - Does not trust Referer header to prevent CSRF
#[derive(Debug, Clone)]
pub struct AdminOriginExtractor {
    config: Arc<AdminOriginConfig>,
}

impl AdminOriginExtractor {
    pub fn new(config: Arc<AdminOriginConfig>) -> Self {
        Self { config }
    }
}

/// Admin origin validation
///
/// Validates the Origin header against allowed origins.
///
/// # Security
/// - **DOES NOT** use Referer header (CSRF vulnerability)
/// - Only Origin header is checked
/// - Origin is normalized before validation
pub async fn validate_admin_origin(
    parts: &mut Parts,
    config: &AdminOriginConfig,
) -> Result<AdminOriginValidation, StatusCode> {
    // Get Origin header ONLY - do NOT use Referer
    // Referer header can be spoofed and is not reliable for security decisions
    let origin = parts
        .headers
        .get(header::ORIGIN)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let final_origin = origin.as_ref().and_then(|o| {
        // Try to normalize the origin
        normalize_origin(o).ok()
    });

    let is_allowed = final_origin
        .as_ref()
        .map(|o| config.is_origin_allowed(o))
        .unwrap_or(false);

    if !is_allowed && config.enforce {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(AdminOriginValidation {
        origin: final_origin,
        is_allowed,
    })
}

/// Admin origin middleware layer
pub struct AdminOriginLayer {
    config: Arc<AdminOriginConfig>,
}

impl AdminOriginLayer {
    pub fn new(config: AdminOriginConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Get the configuration
    pub fn config(&self) -> Arc<AdminOriginConfig> {
        self.config.clone()
    }
}

/// Admin origin rejection
#[derive(Debug, Clone)]
pub struct AdminOriginRejection;

impl IntoResponse for AdminOriginRejection {
    fn into_response(self) -> Response {
        (
            StatusCode::FORBIDDEN,
            [(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("text/plain"),
            )],
            "Admin origin validation failed: Origin header is required and must be an allowed \
             origin",
        )
            .into_response()
    }
}

/// Extract origin from request parts (for use in handlers)
///
/// Returns the normalized origin if present and valid, or None
pub fn extract_origin_from_request(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get(header::ORIGIN)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| normalize_origin(s).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_origin() {
        // Basic normalization
        assert_eq!(
            normalize_origin("https://Example.com").unwrap(),
            "https://example.com"
        );

        // Remove trailing slash
        assert_eq!(
            normalize_origin("https://example.com/").unwrap(),
            "https://example.com"
        );

        // Remove default https port
        assert_eq!(
            normalize_origin("https://example.com:443").unwrap(),
            "https://example.com"
        );

        // Remove default http port
        assert_eq!(
            normalize_origin("http://example.com:80").unwrap(),
            "http://example.com"
        );

        // Keep non-default port
        assert_eq!(
            normalize_origin("http://example.com:8080").unwrap(),
            "http://example.com:8080"
        );

        // Localhost
        assert_eq!(
            normalize_origin("http://localhost:3000").unwrap(),
            "http://localhost:3000"
        );

        // IPv6 localhost
        assert_eq!(
            normalize_origin("http://[::1]:3000").unwrap(),
            "http://[::1]:3000"
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
                let mut set = HashSet::new();
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
            allowed_origins: HashSet::new(),
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
            allowed_origins: HashSet::new(),
            enforce: false,
            allow_localhost: false,
        };

        // When enforce is false, all origins are allowed
        assert!(config.is_origin_allowed("https://any-origin.com"));
    }
}
