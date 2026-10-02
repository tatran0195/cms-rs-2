//! CMS Configuration
//!
//! This crate provides typed configuration for the CMS application,
//! following pattern of a single Config struct with env-file split
//! for dev vs. deploy.
//!
//! Configuration is loaded from:
//! - Environment variables (prefixed with CMS_)
//! - Base config file (`config.toml` or `config/config.toml`)
//! - Environment profile config (`config/dev.toml`, `config/deploy.toml`, etc.)
//! - Custom config path (`CMS_CONFIG_PATH`)

use config::{Config as ConfigLib, Environment, File};
use serde::Deserialize;

/// Main configuration struct for the CMS application
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,

    /// Storage configuration
    #[serde(default)]
    pub storage: StorageConfig,

    /// Search configuration
    #[serde(default)]
    pub search: SearchConfig,

    /// Queue configuration
    #[serde(default)]
    pub queue: QueueConfig,

    /// Analytics configuration
    #[serde(default)]
    pub analytics: AnalyticsConfig,

    /// Authentication configuration
    #[serde(default)]
    pub auth: AuthConfig,

    /// Site serving configuration
    #[serde(default)]
    pub site: SiteConfig,

    /// DNS verification for custom domains
    #[serde(default)]
    pub domain_verification: DomainVerificationConfig,

    /// Trusted reverse-proxy signal for custom-domain TLS status reporting.
    #[serde(default)]
    pub domain_tls: DomainTlsConfig,

    /// MCP configuration
    #[serde(default)]
    pub mcp: McpConfig,

    /// Mailer configuration
    #[serde(default)]
    pub mailer: Option<MailerConfig>,

    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limit: RateLimitConfig,

    /// Security headers configuration
    #[serde(default)]
    pub security_headers: SecurityHeadersConfig,

    /// Admin origin configuration
    #[serde(default)]
    pub admin_origin: AdminOriginConfig,
}

/// Server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,

    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Whether to enable HTTPS (for direct TLS termination)
    #[serde(default)]
    pub https: bool,

    /// Path to TLS certificate (if https is true)
    #[serde(default)]
    pub tls_cert_path: Option<String>,

    /// Path to TLS key (if https is true)
    #[serde(default)]
    pub tls_key_path: Option<String>,

    /// Trusted proxy hops for X-Forwarded-For parsing
    #[serde(default = "default_trusted_proxy_hops")]
    pub trusted_proxy_hops: usize,
}

fn default_port() -> u16 {
    3000
}
fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_trusted_proxy_hops() -> usize {
    1
}

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    #[serde(default = "default_database_url")]
    pub url: String,

    /// Maximum pool size
    #[serde(default = "default_max_pool_size")]
    pub max_pool_size: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_conn_timeout")]
    pub connection_timeout: u64,

    /// Whether to enable SSL
    #[serde(default)]
    pub ssl: bool,
}

fn default_database_url() -> String {
    "postgres://postgres:postgres@localhost:5432/cms".to_string()
}
fn default_max_pool_size() -> u32 {
    20
}
fn default_conn_timeout() -> u64 {
    30
}

/// Storage configuration
#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type: "local" or "s3"
    #[serde(default = "default_storage_backend")]
    pub backend: String,

    /// Local storage root directory (for "local" backend)
    #[serde(default)]
    pub local_root: Option<String>,

    /// S3 endpoint URL
    #[serde(default)]
    pub s3_endpoint: Option<String>,

    /// S3 region
    #[serde(default)]
    pub s3_region: Option<String>,

    /// S3 bucket name
    #[serde(default)]
    pub s3_bucket: Option<String>,

    /// S3 access key
    #[serde(default)]
    pub s3_access_key: Option<String>,

    /// S3 secret key
    #[serde(default)]
    pub s3_secret_key: Option<String>,

    /// Whether to use path-style addressing
    #[serde(default)]
    pub s3_path_style: bool,
}

fn default_storage_backend() -> String {
    "local".to_string()
}

/// Search configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SearchConfig {
    /// Vector store backend: "pgvector" or "qdrant"
    #[serde(default = "default_search_backend")]
    pub backend: String,

    /// PostgreSQL connection URL for pgvector
    #[serde(default)]
    pub pgvector_url: Option<String>,

    /// Qdrant host
    #[serde(default)]
    pub qdrant_host: Option<String>,

    /// Qdrant port
    #[serde(default = "default_qdrant_port")]
    pub qdrant_port: u16,

    /// Qdrant API key
    #[serde(default)]
    pub qdrant_api_key: Option<String>,

    /// Lindera dictionary path (for Japanese tokenization)
    #[serde(default)]
    pub lindera_dict_path: Option<String>,

    /// Maximum number of search results
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

fn default_search_backend() -> String {
    "pgvector".to_string()
}
fn default_qdrant_port() -> u16 {
    6333
}
fn default_max_results() -> usize {
    50
}

/// Queue configuration
#[derive(Debug, Clone, Deserialize)]
pub struct QueueConfig {
    /// Queue backend: "memory" or "redis"
    #[serde(default = "default_queue_backend")]
    pub backend: String,

    /// Redis URL (for "redis" backend)
    #[serde(default)]
    pub redis_url: Option<String>,

    /// Number of worker threads for in-memory queue
    #[serde(default = "default_queue_workers")]
    pub workers: usize,

    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
}

fn default_queue_backend() -> String {
    "postgres".to_string()
}
fn default_queue_workers() -> usize {
    4
}
fn default_max_retries() -> usize {
    3
}

/// Analytics configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AnalyticsConfig {
    /// Analytics backend: "postgres" or "clickhouse"
    #[serde(default = "default_analytics_backend")]
    pub backend: String,

    /// ClickHouse host
    #[serde(default)]
    pub clickhouse_host: Option<String>,

    /// ClickHouse port
    #[serde(default = "default_clickhouse_port")]
    pub clickhouse_port: u16,

    /// ClickHouse database
    #[serde(default)]
    pub clickhouse_database: Option<String>,

    /// ClickHouse username
    #[serde(default)]
    pub clickhouse_username: Option<String>,

    /// ClickHouse password
    #[serde(default)]
    pub clickhouse_password: Option<String>,
}

fn default_analytics_backend() -> String {
    "postgres".to_string()
}
fn default_clickhouse_port() -> u16 {
    8123
}

/// Authentication configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    /// Session secret key
    #[serde(default = "default_session_secret")]
    pub session_secret: String,

    /// Session expiration in hours
    #[serde(default = "default_session_expiration")]
    pub session_expiration_hours: i64,

    /// JWT secret for reader tokens
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,

    /// JWT expiration in hours
    #[serde(default = "default_jwt_expiration")]
    pub jwt_expiration_hours: i64,

    /// API key prefix for hashing
    #[serde(default = "default_api_key_prefix")]
    pub api_key_prefix: String,

    /// Operator accounts allowed to use platform-wide admin APIs (comma-separated
    /// through environment configuration, or an array in TOML).
    #[serde(default)]
    pub system_admin_emails: Vec<String>,

    /// OAuth providers configuration
    #[serde(default)]
    pub oauth: Option<OAuthConfig>,
}

fn default_session_secret() -> String {
    "dev_session_secret_change_in_production".to_string()
}
fn default_jwt_secret() -> String {
    "dev_jwt_secret_change_in_production".to_string()
}
fn default_session_expiration() -> i64 {
    24 * 7
} // 7 days
fn default_jwt_expiration() -> i64 {
    24 * 30
} // 30 days
fn default_api_key_prefix() -> String {
    "cms_api_key".to_string()
}

/// OAuth configuration
#[derive(Debug, Clone, Deserialize)]
pub struct OAuthConfig {
    /// GitHub OAuth configuration
    #[serde(default)]
    pub github: Option<OAuthProviderConfig>,

    /// Google OAuth configuration
    #[serde(default)]
    pub google: Option<OAuthProviderConfig>,
}

/// OAuth provider configuration
#[derive(Debug, Clone, Deserialize)]
pub struct OAuthProviderConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

/// Site serving configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SiteConfig {
    /// Marketing brand host (e.g., cms.com)
    #[serde(default)]
    pub marketing_host: Option<String>,

    /// Self-hosted operator's host
    #[serde(default)]
    pub self_host: Option<String>,

    /// Custom domain edge secret (for Cloudflare Worker)
    #[serde(default)]
    pub edge_secret: Option<String>,

    /// Maximum age for SEO cache
    #[serde(default = "default_seo_cache_max_age")]
    pub seo_cache_max_age: usize,
}

fn default_seo_cache_max_age() -> usize {
    300
} // 5 minutes

/// Resolver settings used to prove TXT-record ownership for custom domains.
#[derive(Debug, Clone, Deserialize)]
pub struct DomainVerificationConfig {
    /// DNS-over-HTTPS JSON endpoint; must use HTTPS.
    #[serde(default = "default_dns_resolver_url")]
    pub resolver_url: String,

    /// Per-check HTTP timeout, clamped to 1–15 seconds.
    #[serde(default = "default_dns_resolver_timeout_seconds")]
    pub timeout_seconds: u64,
}

fn default_dns_resolver_url() -> String {
    "https://cloudflare-dns.com/dns-query".to_string()
}

fn default_dns_resolver_timeout_seconds() -> u64 {
    5
}

impl Default for DomainVerificationConfig {
    fn default() -> Self {
        Self {
            resolver_url: default_dns_resolver_url(),
            timeout_seconds: default_dns_resolver_timeout_seconds(),
        }
    }
}

/// Optional secret shared with a trusted TLS-terminating reverse proxy. When
/// present, a successful HTTPS request for a verified custom domain can update
/// its operational TLS status without exposing or storing private key material.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DomainTlsConfig {
    #[serde(default)]
    pub proxy_secret: Option<String>,
}

/// MCP configuration
#[derive(Debug, Clone, Deserialize)]
pub struct McpConfig {
    /// Whether MCP server is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Maximum concurrent MCP connections
    #[serde(default = "default_mcp_max_connections")]
    pub max_connections: usize,

    /// MCP rate limit per minute
    #[serde(default = "default_mcp_rate_limit")]
    pub rate_limit: usize,
}

fn default_mcp_max_connections() -> usize {
    100
}
fn default_mcp_rate_limit() -> usize {
    60
}

/// Mailer configuration
#[derive(Debug, Clone, Deserialize)]
pub struct MailerConfig {
    /// SMTP host
    #[serde(default)]
    pub smtp_host: Option<String>,

    /// SMTP port
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,

    /// SMTP username
    #[serde(default)]
    pub smtp_username: Option<String>,

    /// SMTP password
    #[serde(default)]
    pub smtp_password: Option<String>,

    /// Whether to use TLS (true = implicit TLS/SMTPS on port 465)
    #[serde(default)]
    pub smtp_use_tls: bool,

    /// Use plain unencrypted SMTP (no TLS, no STARTTLS) — for local dev tools like Mailpit
    #[serde(default)]
    pub smtp_plain_no_tls: bool,

    /// From email address
    #[serde(default)]
    pub from_email: Option<String>,

    /// From name
    #[serde(default)]
    pub from_name: Option<String>,
}

fn default_smtp_port() -> u16 {
    587
}

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled
    #[serde(default = "default_rate_limit_enabled")]
    pub enabled: bool,

    /// Requests per second limit
    #[serde(default = "default_rate_limit_rps")]
    pub requests_per_second: u32,

    /// Burst capacity
    #[serde(default = "default_rate_limit_burst")]
    pub burst_size: u32,

    /// Maximum number of tracked clients
    #[serde(default = "default_rate_limit_max_clients")]
    pub max_tracked_clients: usize,

    /// Client TTL in seconds
    #[serde(default = "default_rate_limit_ttl")]
    pub client_ttl_secs: u64,
}

fn default_rate_limit_enabled() -> bool {
    true
}
fn default_rate_limit_rps() -> u32 {
    100
}
fn default_rate_limit_burst() -> u32 {
    200
}
fn default_rate_limit_max_clients() -> usize {
    10_000
}
fn default_rate_limit_ttl() -> u64 {
    300
} // 5 minutes

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: default_rate_limit_enabled(),
            requests_per_second: default_rate_limit_rps(),
            burst_size: default_rate_limit_burst(),
            max_tracked_clients: default_rate_limit_max_clients(),
            client_ttl_secs: default_rate_limit_ttl(),
        }
    }
}

/// Security headers configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityHeadersConfig {
    /// Enable HSTS
    #[serde(default = "default_security_headers_hsts")]
    pub enable_hsts: bool,

    /// HSTS max age in seconds
    #[serde(default = "default_security_headers_hsts_max_age")]
    pub hsts_max_age: u32,

    /// Include subdomains in HSTS
    #[serde(default = "default_security_headers_hsts_include_subdomains")]
    pub hsts_include_subdomains: bool,

    /// Enable X-Content-Type-Options
    #[serde(default = "default_security_headers_x_content_type")]
    pub enable_x_content_type_options: bool,

    /// Enable X-Frame-Options
    #[serde(default = "default_security_headers_x_frame")]
    pub enable_x_frame_options: bool,

    /// X-Frame-Options value
    #[serde(default = "default_security_headers_x_frame_options")]
    pub x_frame_options: String,

    /// Enable X-XSS-Protection
    #[serde(default = "default_security_headers_x_xss")]
    pub enable_x_xss_protection: bool,

    /// Enable Content-Security-Policy
    #[serde(default = "default_security_headers_csp")]
    pub enable_csp: bool,

    /// CSP directive string
    #[serde(default = "default_security_headers_csp_value")]
    pub csp: String,

    /// Enable Referrer-Policy
    #[serde(default = "default_security_headers_referrer")]
    pub enable_referrer_policy: bool,

    /// Referrer-Policy value
    #[serde(default = "default_security_headers_referrer_policy")]
    pub referrer_policy: String,

    /// Enable Permissions-Policy
    #[serde(default = "default_security_headers_permissions")]
    pub enable_permissions_policy: bool,

    /// Permissions-Policy value
    #[serde(default = "default_security_headers_permissions_value")]
    pub permissions_policy: String,
}

fn default_security_headers_hsts() -> bool {
    true
}
fn default_security_headers_hsts_max_age() -> u32 {
    31536000
} // 1 year
fn default_security_headers_hsts_include_subdomains() -> bool {
    true
}
fn default_security_headers_x_content_type() -> bool {
    true
}
fn default_security_headers_x_frame() -> bool {
    true
}
fn default_security_headers_x_frame_options() -> String {
    "DENY".to_string()
}
fn default_security_headers_x_xss() -> bool {
    true
}
fn default_security_headers_csp() -> bool {
    true
}
fn default_security_headers_csp_value() -> String {
    "default-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self';".to_string()
}
fn default_security_headers_referrer() -> bool {
    true
}
fn default_security_headers_referrer_policy() -> String {
    "strict-origin-when-cross-origin".to_string()
}
fn default_security_headers_permissions() -> bool {
    true
}
fn default_security_headers_permissions_value() -> String {
    "geolocation=(), microphone=(), camera=(), payment=()".to_string()
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            enable_hsts: default_security_headers_hsts(),
            hsts_max_age: default_security_headers_hsts_max_age(),
            hsts_include_subdomains: default_security_headers_hsts_include_subdomains(),
            enable_x_content_type_options: default_security_headers_x_content_type(),
            enable_x_frame_options: default_security_headers_x_frame(),
            x_frame_options: default_security_headers_x_frame_options(),
            enable_x_xss_protection: default_security_headers_x_xss(),
            enable_csp: default_security_headers_csp(),
            csp: default_security_headers_csp_value(),
            enable_referrer_policy: default_security_headers_referrer(),
            referrer_policy: default_security_headers_referrer_policy(),
            enable_permissions_policy: default_security_headers_permissions(),
            permissions_policy: default_security_headers_permissions_value(),
        }
    }
}

/// Admin origin configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AdminOriginConfig {
    /// Allowed admin origins
    #[serde(default = "default_admin_origin_origins")]
    pub allowed_origins: Vec<String>,

    /// Whether to enforce origin checking
    #[serde(default = "default_admin_origin_enforce")]
    pub enforce: bool,

    /// Whether to allow localhost for development
    #[serde(default = "default_admin_origin_localhost")]
    pub allow_localhost: bool,
}

fn default_admin_origin_origins() -> Vec<String> {
    vec![
        "https://admin.cms.com".to_string(),
        "https://app.cms.com".to_string(),
        "https://cms.com".to_string(),
    ]
}

fn default_admin_origin_enforce() -> bool {
    true
}
fn default_admin_origin_localhost() -> bool {
    true
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            host: default_host(),
            https: false,
            tls_cert_path: None,
            tls_key_path: None,
            trusted_proxy_hops: default_trusted_proxy_hops(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: default_database_url(),
            max_pool_size: default_max_pool_size(),
            connection_timeout: default_conn_timeout(),
            ssl: false,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            session_secret: default_session_secret(),
            session_expiration_hours: default_session_expiration(),
            jwt_secret: default_jwt_secret(),
            jwt_expiration_hours: default_jwt_expiration(),
            api_key_prefix: default_api_key_prefix(),
            system_admin_emails: Vec::new(),
            oauth: None,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: default_storage_backend(),
            local_root: None,
            s3_endpoint: None,
            s3_region: None,
            s3_bucket: None,
            s3_access_key: None,
            s3_secret_key: None,
            s3_path_style: false,
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            backend: default_search_backend(),
            pgvector_url: None,
            qdrant_host: None,
            qdrant_port: default_qdrant_port(),
            qdrant_api_key: None,
            lindera_dict_path: None,
            max_results: default_max_results(),
        }
    }
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            backend: default_queue_backend(),
            redis_url: None,
            workers: default_queue_workers(),
            max_retries: default_max_retries(),
        }
    }
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            backend: default_analytics_backend(),
            clickhouse_host: None,
            clickhouse_port: default_clickhouse_port(),
            clickhouse_database: None,
            clickhouse_username: None,
            clickhouse_password: None,
        }
    }
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            marketing_host: None,
            self_host: None,
            edge_secret: None,
            seo_cache_max_age: default_seo_cache_max_age(),
        }
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_connections: default_mcp_max_connections(),
            rate_limit: default_mcp_rate_limit(),
        }
    }
}

impl Default for AdminOriginConfig {
    fn default() -> Self {
        Self {
            allowed_origins: default_admin_origin_origins(),
            enforce: default_admin_origin_enforce(),
            allow_localhost: default_admin_origin_localhost(),
        }
    }
}

impl Config {
    /// Load configuration from the environment and files
    ///
    /// Configuration is loaded in the following cascading order (later sources override earlier ones):
    /// 1. Default values from struct defaults (implicit via serde defaults)
    /// 2. Base configuration file: `config.toml` (or `config/config.toml` if found)
    /// 3. Environment-specific configuration file based on `CMS_ENV`:
    ///    - `config/{CMS_ENV}.toml` (e.g. `config/dev.toml`, `config/deploy.toml`)
    ///    - Fallback: `config/{CMS_ENV}.env`
    /// 4. Explicit configuration file path from `CMS_CONFIG_PATH` env var (if set)
    /// 5. Environment variables prefixed with `CMS_` (using `__` for nested keys)
    pub fn load() -> Result<Self, anyhow::Error> {
        let mut builder = ConfigLib::builder();

        // 1. Base config file (config.toml in root or config/)
        if std::path::Path::new("config.toml").exists() {
            builder = builder.add_source(File::with_name("config.toml"));
        } else if std::path::Path::new("config/config.toml").exists() {
            builder = builder.add_source(File::with_name("config/config.toml"));
        }

        // 2. Environment profile config
        let env = std::env::var("CMS_ENV").unwrap_or_else(|_| "dev".to_string());
        let env_toml = format!("config/{}.toml", env);
        let env_file = format!("config/{}.env", env);

        if std::path::Path::new(&env_toml).exists() {
            builder = builder.add_source(File::with_name(&env_toml));
        } else if std::path::Path::new(&env_file).exists() {
            builder = builder.add_source(File::with_name(&env_file));
        }

        // 3. Explicit config path override via env var
        if let Ok(custom_path) = std::env::var("CMS_CONFIG_PATH") {
            if std::path::Path::new(&custom_path).exists() {
                builder = builder.add_source(File::with_name(&custom_path));
            }
        }

        // 4. Environment variables with CMS_ prefix
        builder = builder.add_source(
            Environment::with_prefix("CMS")
                .prefix_separator("_")
                .separator("__"),
        );

        let settings = builder.build()?;

        Ok(settings.try_deserialize()?)
    }

    /// Load configuration from a specific path, with environment variable overrides
    pub fn load_from_path(path: &str) -> Result<Self, anyhow::Error> {
        let mut builder = ConfigLib::builder();

        if !std::path::Path::new(path).exists() {
            anyhow::bail!("Config file does not exist at path: {}", path);
        }

        builder = builder.add_source(File::with_name(path));

        builder = builder.add_source(
            Environment::with_prefix("CMS")
                .prefix_separator("_")
                .separator("__"),
        );

        let settings = builder.build()?;

        Ok(settings.try_deserialize()?)
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::sync::Mutex;

    use super::*;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_default_config() {
        let _guard = ENV_MUTEX.lock().unwrap();
        // Temporarily clear environment
        env::remove_var("CMS_ENV");
        env::remove_var("CMS_CONFIG_PATH");

        let config = Config::default();
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(
            config.database.url,
            "postgres://postgres:postgres@localhost:5432/cms"
        );
        assert_eq!(config.storage.backend, "local");
        assert_eq!(config.search.backend, "pgvector");
        assert_eq!(config.queue.backend, "postgres");
        assert_eq!(config.analytics.backend, "postgres");
        assert_eq!(
            config.auth.session_secret,
            "dev_session_secret_change_in_production"
        );
        assert!(!config.mcp.enabled);
        assert!(config.rate_limit.enabled);
        assert!(config.security_headers.enable_hsts);
        assert!(config.admin_origin.enforce);
    }

    #[test]
    fn test_server_config_defaults() {
        let server = ServerConfig::default();
        assert_eq!(server.port, 3000);
        assert_eq!(server.host, "0.0.0.0");
        assert!(!server.https);
        assert_eq!(server.trusted_proxy_hops, 1);
    }

    #[test]
    fn test_storage_config_defaults() {
        let storage = StorageConfig::default();
        assert_eq!(storage.backend, "local");
        assert_eq!(storage.local_root, None);
        assert_eq!(storage.s3_endpoint, None);
    }

    #[test]
    fn test_search_config_defaults() {
        let search = SearchConfig::default();
        assert_eq!(search.backend, "pgvector");
        assert_eq!(search.qdrant_port, 6333);
        assert_eq!(search.max_results, 50);
    }

    #[test]
    fn test_queue_config_defaults() {
        let queue = QueueConfig::default();
        assert_eq!(queue.backend, "postgres");
        assert_eq!(queue.workers, 4);
        assert_eq!(queue.max_retries, 3);
    }

    #[test]
    fn test_analytics_config_defaults() {
        let analytics = AnalyticsConfig::default();
        assert_eq!(analytics.backend, "postgres");
        assert_eq!(analytics.clickhouse_port, 8123);
    }

    #[test]
    fn test_rate_limit_config_defaults() {
        let rate_limit = RateLimitConfig::default();
        assert!(rate_limit.enabled);
        assert_eq!(rate_limit.requests_per_second, 100);
        assert_eq!(rate_limit.burst_size, 200);
        assert_eq!(rate_limit.max_tracked_clients, 10_000);
        assert_eq!(rate_limit.client_ttl_secs, 300);
    }

    #[test]
    fn test_security_headers_defaults() {
        let headers = SecurityHeadersConfig::default();
        assert!(headers.enable_hsts);
        assert_eq!(headers.hsts_max_age, 31536000);
        assert!(headers.enable_x_content_type_options);
        assert!(headers.enable_x_frame_options);
        assert_eq!(headers.x_frame_options, "DENY");
    }

    #[test]
    fn test_admin_origin_defaults() {
        let admin_origin = AdminOriginConfig::default();
        assert!(admin_origin.enforce);
        assert!(admin_origin.allow_localhost);
        assert!(admin_origin
            .allowed_origins
            .contains(&"https://admin.cms.com".to_string()));
    }

    #[test]
    fn test_load_from_toml_content() {
        let toml_data = r#"
            [server]
            port = 8080
            host = "127.0.0.1"

            [database]
            url = "postgres://custom:custom@localhost:5432/custom_db"
            max_pool_size = 50

            [storage]
            backend = "s3"
            s3_bucket = "my-bucket"
            s3_region = "us-west-2"

            [rate_limit]
            enabled = false
            requests_per_second = 500
        "#;

        let builder = ConfigLib::builder()
            .add_source(config::File::from_str(toml_data, config::FileFormat::Toml));
        let settings = builder.build().expect("should parse toml data");
        let config: Config = settings
            .try_deserialize()
            .expect("should deserialize Config");

        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(
            config.database.url,
            "postgres://custom:custom@localhost:5432/custom_db"
        );
        assert_eq!(config.database.max_pool_size, 50);
        assert_eq!(config.storage.backend, "s3");
        assert_eq!(config.storage.s3_bucket.as_deref(), Some("my-bucket"));
        assert_eq!(config.storage.s3_region.as_deref(), Some("us-west-2"));
        assert!(!config.rate_limit.enabled);
        assert_eq!(config.rate_limit.requests_per_second, 500);
        // Untouched sections retain default values
        assert_eq!(config.queue.backend, "postgres");
        assert_eq!(config.search.backend, "pgvector");
    }

    #[test]
    fn test_load_from_path() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let deploy_path = format!("{}/../../config/deploy.toml", manifest_dir);
        let config = Config::load_from_path(&deploy_path);
        assert!(
            config.is_ok(),
            "loading config/deploy.toml should succeed: {:?}",
            config.err()
        );
        let config = config.unwrap();
        assert_eq!(config.database.max_pool_size, 30);
        assert_eq!(config.queue.workers, 8);
        assert_eq!(config.queue.max_retries, 5);
        assert!(!config.admin_origin.allow_localhost);
    }

    #[test]
    fn test_env_var_override() {
        let _guard = ENV_MUTEX.lock().unwrap();
        // Set an env var
        env::set_var("CMS_SERVER__PORT", "9999");
        env::set_var("CMS_DATABASE__MAX_POOL_SIZE", "77");

        let config = Config::load().expect("Config::load should succeed");
        assert_eq!(config.server.port, 9999);
        assert_eq!(config.database.max_pool_size, 77);

        // Cleanup
        env::remove_var("CMS_SERVER__PORT");
        env::remove_var("CMS_DATABASE__MAX_POOL_SIZE");
    }

    #[test]
    fn test_load_cascading() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let config = Config::load().expect("Config::load should succeed");
        assert_eq!(config.server.port, 3000);
        assert_eq!(
            config.database.url,
            "postgres://postgres:postgres@localhost:5432/cms"
        );
        assert_eq!(config.storage.backend, "local");
    }
}
