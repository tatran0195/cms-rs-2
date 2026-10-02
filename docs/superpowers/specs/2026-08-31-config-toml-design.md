# Design Specification: TOML Configuration System & Multi-tiered Loader

- **Date**: 2026-08-31
- **Topic**: TOML Configuration System (`config.toml`, `config/dev.toml`, `config/deploy.toml`, `cms-config`)
- **Status**: Draft / Under Review

---

## 1. Overview & Goals

The CMS platform requires a robust, readable, and environment-friendly configuration system. Currently, `cms-config` loads only `.env` files or environment variables. This design introduces full TOML configuration support, structured sample/default files for both local development and AWS production deployment, and a cascading loading hierarchy that maintains backwards compatibility with `.env` and environment variables.

---

## 2. Configuration Schema & Section Mapping

The TOML configuration maps directly to the strongly typed Rust structs in `crates/cms-config/src/lib.rs`:

### 2.1 `[server]` (`ServerConfig`)
- `host` (string, default: `"0.0.0.0"`): IP/host to bind the Axum web server to.
- `port` (u16, default: `3000`): TCP port to listen on.
- `https` (bool, default: `false`): Enable direct TLS termination.
- `tls_cert_path` (optional string): Path to TLS public certificate file.
- `tls_key_path` (optional string): Path to TLS private key file.
- `trusted_proxy_hops` (usize, default: `1`): Number of upstream proxy hops for client IP extraction.

### 2.2 `[database]` (`DatabaseConfig`)
- `url` (string, default: `"postgres://postgres:postgres@localhost:5432/cms"`): PostgreSQL connection string.
- `max_pool_size` (u32, default: `20`): Maximum SQLx pool connection limit.
- `connection_timeout` (u64, default: `30`): Pool connection timeout in seconds.
- `ssl` (bool, default: `false`): Require TLS/SSL connection to PostgreSQL.

### 2.3 `[storage]` (`StorageConfig`)
- `backend` (string, default: `"local"`): Storage provider (`"local"` or `"s3"`).
- `local_root` (optional string, default: `"./uploads"`): Root folder for filesystem storage backend.
- `s3_endpoint` (optional string): Custom S3/MinIO endpoint URL.
- `s3_region` (optional string): AWS region (e.g. `"ap-northeast-1"`).
- `s3_bucket` (optional string): S3 bucket name.
- `s3_access_key` (optional string): S3 access key ID.
- `s3_secret_key` (optional string): S3 secret access key.
- `s3_path_style` (bool, default: `false`): Use path-style addressing (`true` for MinIO/LocalStack).

### 2.4 `[search]` (`SearchConfig`)
- `backend` (string, default: `"pgvector"`): Vector search engine (`"pgvector"` or `"qdrant"`).
- `pgvector_url` (optional string): Dedicated PostgreSQL connection string if separate from primary DB.
- `qdrant_host` (optional string, default: `"localhost"`): Qdrant service host.
- `qdrant_port` (u16, default: `6333`): Qdrant REST/gRPC port.
- `qdrant_api_key` (optional string): API key for authenticated Qdrant instances.
- `lindera_dict_path` (optional string): Path to Lindera dictionary for Japanese morphological tokenization.
- `max_results` (usize, default: `50`): Default maximum search result items returned.

### 2.5 `[queue]` (`QueueConfig`)
- `backend` (string, default: `"memory"`): Job queue engine (`"memory"` or `"redis"`).
- `redis_url` (optional string): Redis connection URI (e.g. `"redis://127.0.0.1:6379"`).
- `workers` (usize, default: `4`): Worker concurrency count.
- `max_retries` (usize, default: `3`): Maximum automatic retry count for failed tasks.

### 2.6 `[analytics]` (`AnalyticsConfig`)
- `backend` (string, default: `"postgres"`): Storage backend for analytics events (`"postgres"` or `"clickhouse"`).
- `clickhouse_host` (optional string): ClickHouse host.
- `clickhouse_port` (u16, default: `8123`): ClickHouse HTTP port.
- `clickhouse_database` (optional string): Analytics database name.
- `clickhouse_username` (optional string): Username for ClickHouse.
- `clickhouse_password` (optional string): Password for ClickHouse.

### 2.7 `[auth]` & `[auth.oauth.*]` (`AuthConfig`, `OAuthConfig`)
- `session_secret` (string): Secret key for signing session cookies.
- `session_expiration_hours` (i64, default: `168`): Session validity duration in hours (7 days).
- `jwt_secret` (string): Secret key for signing reader/API JWTs.
- `jwt_expiration_hours` (i64, default: `720`): JWT token expiration in hours (30 days).
- `api_key_prefix` (string, default: `"cms_api_key"`): Prefix used for hashing and validating API keys.
- `[auth.oauth.github]`: `client_id`, `client_secret`, `redirect_uri` (optional).
- `[auth.oauth.google]`: `client_id`, `client_secret`, `redirect_uri` (optional).

### 2.8 `[site]` (`SiteConfig`)
- `marketing_host` (optional string): Primary domain for marketing site (e.g. `"cms.com"`).
- `self_host` (optional string): Custom self-hosted domain.
- `edge_secret` (optional string): Shared secret for Cloudflare edge worker verification.
- `seo_cache_max_age` (usize, default: `300`): Cache duration in seconds for rendered SEO HTML.

### 2.9 `[mcp]` (`McpConfig`)
- `enabled` (bool, default: `false`): Enable Model Context Protocol (MCP) server endpoints.
- `max_connections` (usize, default: `100`): Maximum concurrent MCP connections.
- `rate_limit` (usize, default: `60`): Maximum MCP requests per minute per connection.

### 2.10 `[mailer]` (`MailerConfig`)
- `smtp_host` (optional string): SMTP mail server hostname.
- `smtp_port` (u16, default: `587`): SMTP port.
- `smtp_username` (optional string): SMTP login username.
- `smtp_password` (optional string): SMTP password.
- `smtp_use_tls` (bool, default: `true`): Enable STARTTLS/TLS for SMTP.
- `from_email` (optional string): Sender address.
- `from_name` (optional string): Sender display name.

### 2.11 `[rate_limit]` (`RateLimitConfig`)
- `enabled` (bool, default: `true`): Enable global API rate limiting.
- `requests_per_second` (u32, default: `100`): Token bucket refill rate.
- `burst_size` (u32, default: `200`): Maximum burst token capacity.
- `max_tracked_clients` (usize, default: `10000`): Maximum tracked client IP states in memory.
- `client_ttl_secs` (u64, default: `300`): TTL before clearing idle client rate limits.

### 2.12 `[security_headers]` (`SecurityHeadersConfig`)
- `enable_hsts` (bool, default: `true`), `hsts_max_age` (u32, default: `31536000`), `hsts_include_subdomains` (bool, default: `true`).
- `enable_x_content_type_options` (bool, default: `true`).
- `enable_x_frame_options` (bool, default: `true`), `x_frame_options` (string, default: `"DENY"`).
- `enable_x_xss_protection` (bool, default: `true`).
- `enable_csp` (bool, default: `true`), `csp` (string, default: `"default-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self';"`).
- `enable_referrer_policy` (bool, default: `true`), `referrer_policy` (string, default: `"strict-origin-when-cross-origin"`).
- `enable_permissions_policy` (bool, default: `true`), `permissions_policy` (string, default: `"geolocation=(), microphone=(), camera=(), payment=()"`).

### 2.13 `[admin_origin]` (`AdminOriginConfig`)
- `allowed_origins` (list of strings): Allowed origins for CORS and admin API access.
- `enforce` (bool, default: `true`): Enforce origin check.
- `allow_localhost` (bool, default: `true`): Permit localhost origins for development.

---

## 3. Loading Hierarchy & Precedence

`Config::load()` will evaluate sources in the following cascading order (later sources override earlier sources):

```
┌────────────────────────────────────────────────────────┐
│ 1. Struct Default Values (Default trait)              │
└────────────────────────────────────────────────────────┘
                           ↓
┌────────────────────────────────────────────────────────┐
│ 2. Base Configuration: config.toml or config/config.toml │
└────────────────────────────────────────────────────────┘
                           ↓
┌────────────────────────────────────────────────────────┐
│ 3. Environment Profile: config/{CMS_ENV}.toml          │
│    (or fallback config/{CMS_ENV}.env)                   │
└────────────────────────────────────────────────────────┘
                           ↓
┌────────────────────────────────────────────────────────┐
│ 4. Explicit File Override: CMS_CONFIG_PATH / load_path │
└────────────────────────────────────────────────────────┘
                           ↓
┌────────────────────────────────────────────────────────┐
│ 5. Environment Variables: CMS_* (e.g. CMS_SERVER__PORT)│
└────────────────────────────────────────────────────────┘
```

---

## 4. File Structure

```
cms-rs/
├── Cargo.toml                    # update config dependency features = ["toml"]
├── config.toml                   # Local development configuration (with rich comments)
├── config.example.toml           # Clean template example
├── config/
│   ├── dev.toml                  # Dev environment profile overrides
│   └── deploy.toml               # Production deployment profile
└── crates/
    └── cms-config/
        ├── Cargo.toml
        └── src/
            └── lib.rs            # Updated loader logic & unit tests
```

---

## 5. Verification Plan

1. **Unit Tests in `cms-config`**:
   - `test_default_config`: Verify default configuration matches expected baseline.
   - `test_load_from_toml_string`: Test deserialization from TOML representation.
   - `test_load_with_env_override`: Verify `CMS_` env var overrides TOML settings.
   - `test_load_cascading`: Verify cascading profile loading.
2. **Build and Test Verification**:
   - Run `cargo test -p cms-config`.
   - Run `cargo check --workspace`.
