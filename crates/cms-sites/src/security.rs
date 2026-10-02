use axum::{
    http::{HeaderMap, HeaderName, HeaderValue},
    response::Response,
};

/// Security headers helper for HTML meta tags
#[derive(Debug, Clone, Default)]
pub struct SiteSecurityHeaders {
    config: SiteSecurityConfig,
}

impl SiteSecurityHeaders {
    pub fn new(config: SiteSecurityConfig) -> Self {
        Self { config }
    }

    pub fn get_headers(&self) -> Vec<(String, String)> {
        let headers = get_security_headers(&self.config);
        headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
            .collect()
    }
}

/// Security headers configuration for published sites
#[derive(Debug, Clone)]
pub struct SiteSecurityConfig {
    /// Enable HSTS
    pub enable_hsts: bool,
    /// HSTS max age in seconds
    pub hsts_max_age: u32,
    /// Enable X-Content-Type-Options
    pub enable_x_content_type_options: bool,
    /// Enable X-Frame-Options
    pub enable_x_frame_options: bool,
    /// X-Frame-Options value
    pub x_frame_options: String,
    /// Enable X-XSS-Protection
    pub enable_x_xss_protection: bool,
    /// Enable Content-Security-Policy
    pub enable_csp: bool,
    /// CSP directive string
    pub csp: String,
    /// Enable Referrer-Policy
    pub enable_referrer_policy: bool,
    /// Referrer-Policy value
    pub referrer_policy: String,
    /// Enable Permissions-Policy
    pub enable_permissions_policy: bool,
    /// Permissions-Policy value
    pub permissions_policy: String,
}

impl Default for SiteSecurityConfig {
    fn default() -> Self {
        Self {
            enable_hsts: true,
            hsts_max_age: 31536000, // 1 year
            enable_x_content_type_options: true,
            enable_x_frame_options: true,
            x_frame_options: "DENY".to_string(),
            enable_x_xss_protection: true,
            enable_csp: true,
            // More permissive CSP for published sites (allows inline styles/scripts from markdown)
            csp: "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval' https:; \
                  style-src 'self' 'unsafe-inline' https:; img-src 'self' data: https:; font-src \
                  'self' https:; connect-src 'self' https:; frame-src 'none'; object-src 'none'; \
                  base-uri 'self'; form-action 'self';"
                .to_string(),
            enable_referrer_policy: true,
            referrer_policy: "strict-origin-when-cross-origin".to_string(),
            enable_permissions_policy: true,
            permissions_policy: "geolocation=(), microphone=(), camera=(), payment=()".to_string(),
        }
    }
}

/// Get security headers for published sites
pub fn get_security_headers(config: &SiteSecurityConfig) -> HeaderMap {
    let mut headers = HeaderMap::new();

    // Prevent clickjacking
    if config.enable_x_frame_options {
        if let Ok(val) = HeaderValue::from_str(&config.x_frame_options) {
            headers.insert(HeaderName::from_static("x-frame-options"), val);
        }
    }

    // Prevent MIME type sniffing
    if config.enable_x_content_type_options {
        headers.insert(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        );
    }

    // Enable XSS protection
    if config.enable_x_xss_protection {
        headers.insert(
            HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        );
    }

    // Content Security Policy
    if config.enable_csp {
        if let Ok(val) = HeaderValue::from_str(&config.csp) {
            headers.insert(HeaderName::from_static("content-security-policy"), val);
        }
    }

    // Referrer policy
    if config.enable_referrer_policy {
        if let Ok(val) = HeaderValue::from_str(&config.referrer_policy) {
            headers.insert(HeaderName::from_static("referrer-policy"), val);
        }
    }

    // Permissions Policy
    if config.enable_permissions_policy {
        if let Ok(val) = HeaderValue::from_str(&config.permissions_policy) {
            headers.insert(HeaderName::from_static("permissions-policy"), val);
        }
    }

    // HSTS (only for HTTPS)
    if config.enable_hsts {
        if let Ok(val) = HeaderValue::from_str(&format!("max-age={}", config.hsts_max_age)) {
            headers.insert(HeaderName::from_static("strict-transport-security"), val);
        }
    }

    // Additional security headers
    headers.insert(
        HeaderName::from_static("x-powered-by"),
        HeaderValue::from_static("CMS"),
    );

    headers
}

/// Check if request is secure (HTTPS)
pub fn is_secure_request(headers: &HeaderMap) -> bool {
    // Check X-Forwarded-Proto header (for reverse proxy)
    if let Some(proto) = headers.get("X-Forwarded-Proto") {
        if let Ok(proto_str) = proto.to_str() {
            return proto_str.eq_ignore_ascii_case("https");
        }
    }

    // Check if the request is directly HTTPS
    // Note: This is typically determined at the server level
    false
}

/// Security middleware for published sites
pub struct SiteSecurityMiddleware {
    config: SiteSecurityConfig,
}

impl SiteSecurityMiddleware {
    pub fn new(config: SiteSecurityConfig) -> Self {
        Self { config }
    }

    /// Apply security headers to response
    pub fn apply_security_headers(&self, response: Response) -> Response {
        let mut response = response;
        let headers = response.headers_mut();

        // Add all security headers
        headers.extend(get_security_headers(&self.config));

        // Add cache control for static content
        if !headers.contains_key("cache-control") {
            headers.insert(
                HeaderName::from_static("cache-control"),
                HeaderValue::from_static("public, max-age=300"),
            );
        }

        response
    }

    /// Get configuration
    pub fn config(&self) -> &SiteSecurityConfig {
        &self.config
    }
}

/// CORS configuration for published sites
#[derive(Debug, Clone)]
pub struct SiteCorsConfig {
    pub allow_origins: Vec<String>,
    pub allow_methods: Vec<String>,
    pub allow_headers: Vec<String>,
    pub max_age: u32,
}

impl Default for SiteCorsConfig {
    fn default() -> Self {
        Self {
            allow_origins: vec!["*"].into_iter().map(|s| s.to_string()).collect(),
            allow_methods: vec!["GET", "HEAD", "OPTIONS"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            allow_headers: vec!["*"].into_iter().map(|s| s.to_string()).collect(),
            max_age: 86400, // 24 hours
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_headers() {
        let config = SiteSecurityConfig::default();
        let headers = get_security_headers(&config);

        assert!(headers.contains_key("x-frame-options"));
        assert!(headers.contains_key("x-content-type-options"));
        assert!(headers.contains_key("x-xss-protection"));
        assert!(headers.contains_key("content-security-policy"));
        assert!(headers.contains_key("referrer-policy"));
        assert!(headers.contains_key("permissions-policy"));
        assert!(headers.contains_key("strict-transport-security"));
    }
}
