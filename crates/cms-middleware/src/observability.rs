//! Observability middleware using the metrics crate
//!
//! This module provides observability (logging, tracing, metrics) middleware
//! using the lightweight `metrics` crate for metrics collection.
//!
//! # Architecture
//!
//! - **Metrics**: Uses `metrics` crate (lightweight, maintained by tracing authors)
//! - **Tracing**: Uses `tracing` crate for spans and logging
//! - **Request ID**: Generated and propagated through headers
//! - **Timing**: Configurable response time headers
//!
//! # Why metrics crate?
//!
//! The `metrics` crate provides:
//! - Zero-cost abstractions for common metrics patterns
//! - Standard interface that works with various backends (prometheus, etc.)
//! - Very small dependency footprint (~5KB compiled)
//! - Maintained by the same team as `tracing`
//! - Simple, stable API
//!
//! This is preferred over custom metrics implementation for:
//! - Standard interface
//! - Future extensibility (can add Prometheus backend without code changes)
//! - Battle-tested implementation

use std::time::{Duration, Instant};

use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, HeaderValue, Request, StatusCode},
    response::Response,
};
use metrics::{counter, histogram};
#[cfg(feature = "prometheus")]
use metrics_exporter_prometheus::PrometheusBuilder;
use tower::Service;
use tracing::{debug, error, info, warn, Instrument, Span};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Request ID header name
pub const REQUEST_ID_HEADER: &str = "X-Request-ID";

/// Response time header name
pub const RESPONSE_TIME_HEADER: &str = "X-Response-Time";

/// Initialize observability subsystem
///
/// This should be called once at application startup.
/// It sets up tracing with appropriate filters and formatters.
pub fn init_observability() {
    // Only initialize once
    static INITIALIZED: std::sync::Once = std::sync::Once::new();

    INITIALIZED.call_once(|| {
        tracing_subscriber::registry()
            .with(EnvFilter::from_default_env())
            .with(
                fmt::Layer::default()
                    .with_target(true)
                    .with_line_number(true)
                    .with_thread_ids(true)
                    .with_thread_names(true),
            )
            .try_init()
            .ok();

        info!("Observability initialized");

        // Register metrics if using metrics crate
        register_metrics();
    });
}

/// Register metrics with the metrics crate
fn register_metrics() {
    // Pre-register metrics with descriptions using the metrics crate macros
    // These are lazily initialized on first use
    counter!("http_requests_total").increment(0);
    counter!("http_requests_allowed_total").increment(0);
    counter!("http_requests_rejected_total").increment(0);
    counter!("http_responses_total").increment(0);
    histogram!("http_request_duration_seconds").record(0.0);
}

/// Observability configuration
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    /// Whether to add X-Response-Time header to responses
    pub enable_response_time_header: bool,
    /// Whether to generate and track request IDs
    pub enable_request_id: bool,
    /// Whether to log requests
    pub enable_request_logging: bool,
    /// Whether to log responses
    pub enable_response_logging: bool,
    /// Whether to collect metrics
    pub enable_metrics: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            enable_response_time_header: true,
            enable_request_id: true,
            enable_request_logging: true,
            enable_response_logging: true,
            enable_metrics: true,
        }
    }
}

/// Generate a unique request ID
pub fn generate_request_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}

/// Extract request ID from headers or generate a new one
pub fn get_or_generate_request_id(headers: &HeaderMap) -> String {
    if let Some(request_id) = headers.get(REQUEST_ID_HEADER) {
        if let Ok(id) = request_id.to_str() {
            return id.to_string();
        }
    }
    generate_request_id()
}

/// Add request ID to headers if not present, returns the request ID
pub fn ensure_request_id(headers: &mut HeaderMap) -> String {
    let request_id = get_or_generate_request_id(headers);

    if headers.get(REQUEST_ID_HEADER).is_none() {
        if let Ok(header_value) = HeaderValue::from_str(&request_id) {
            headers.insert(REQUEST_ID_HEADER, header_value);
        }
    }

    request_id
}

/// Record a request metric
pub fn record_request(method: &str) {
    counter!("http_requests_total", "method" => method.to_string()).increment(1);
}

/// Record a response metric
pub fn record_response(status: StatusCode) {
    counter!("http_responses_total", "status" => status.as_u16().to_string()).increment(1);
}

/// Record request duration
pub fn record_duration(method: &str, duration: Duration) {
    histogram!("http_request_duration_seconds", "method" => method.to_string())
        .record(duration.as_secs_f64());
}

/// Request tracing middleware
///
/// Creates a span for each request and adds request ID to headers.
pub async fn trace_request<B>(request: Request<B>) -> Request<B> {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let mut headers = request.headers().clone();

    // Ensure request ID is present
    let request_id = ensure_request_id(&mut headers);

    // Create a span for this request
    let span = Span::current();
    span.record("http.method", method.to_string());
    span.record("http.uri", uri.to_string());
    span.record(REQUEST_ID_HEADER, request_id.clone());

    info!(
        method = %method,
        path = %uri,
        request_id = %request_id,
        "Request started"
    );

    // Reconstruct request with potentially updated headers
    let (mut parts, body) = request.into_parts();
    parts.headers = headers;

    Request::from_parts(parts, body)
}

/// Response logging middleware
///
/// Logs response status and records metrics.
pub async fn log_response<B>(response: Response<B>) -> Response<B> {
    let status = response.status();

    // Record metrics
    record_response(status);

    // Log based on status
    if status.is_server_error() {
        error!(status = %status, "Server error response");
    } else if status.is_client_error() {
        warn!(status = %status, "Client error response");
    } else {
        debug!(status = %status, "Successful response");
    }

    response
}

/// Request ID extractor
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl<S> FromRequestParts<S> for RequestId
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let request_id = get_or_generate_request_id(&parts.headers);
        Ok(RequestId(request_id))
    }
}

/// Observability layer for Tower
///
/// This layer provides:
/// - Request tracing with spans
/// - Request ID generation and propagation
/// - Response logging
/// - Request timing
/// - Metrics collection
#[derive(Debug, Clone)]
pub struct ObservabilityLayer {
    config: ObservabilityConfig,
}

impl Default for ObservabilityLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl ObservabilityLayer {
    pub fn new() -> Self {
        Self {
            config: ObservabilityConfig::default(),
        }
    }

    pub fn with_config(config: ObservabilityConfig) -> Self {
        Self { config }
    }

    /// Get the configuration
    pub fn config(&self) -> &ObservabilityConfig {
        &self.config
    }
}

impl<S> tower::Layer<S> for ObservabilityLayer {
    type Service = ObservabilityService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ObservabilityService {
            inner,
            config: self.config.clone(),
        }
    }
}

/// The actual service that wraps the inner service with observability
#[derive(Debug, Clone)]
pub struct ObservabilityService<S> {
    inner: S,
    config: ObservabilityConfig,
}

impl<S, B> Service<Request<B>> for ObservabilityService<S>
where
    S: Service<Request<B>, Response = Response<B>> + Clone + 'static + Send,
    S::Error: std::fmt::Debug + std::fmt::Display + Send,
    S::Future: Send,
    B: 'static + Send,
{
    type Response = Response<B>;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        let mut inner = self.inner.clone();
        let config = self.config.clone();

        // Extract request info before consuming the request
        let request_method = request.method().to_string();
        let request_path = request.uri().path().to_string();
        let request_id = get_or_generate_request_id(request.headers());

        // Create span for tracing
        let span = tracing::span!(
            tracing::Level::INFO,
            "request",
            http.method = %request_method,
            http.uri = %request_path,
            request_id = %request_id,
        );

        let future = async move {
            // Record request start
            if config.enable_request_logging {
                info!(
                    request_id = %request_id,
                    method = %request_method,
                    path = %request_path,
                    "Request started"
                );
            }

            // Record in metrics
            if config.enable_metrics {
                record_request(&request_method);
            }

            let start = Instant::now();

            // Process the request
            let result = inner.call(request).await;

            let duration = start.elapsed();

            match result {
                Ok(response) => {
                    let status = response.status();

                    // Record metrics
                    if config.enable_metrics {
                        record_response(status);
                        record_duration(&request_method, duration);
                    }

                    // Log response
                    if config.enable_response_logging {
                        if status.is_server_error() {
                            error!(
                                request_id = %request_id,
                                status = %status,
                                duration_ms = %duration.as_millis(),
                                "Server error response"
                            );
                        } else if status.is_client_error() {
                            warn!(
                                request_id = %request_id,
                                status = %status,
                                duration_ms = %duration.as_millis(),
                                "Client error response"
                            );
                        } else {
                            debug!(
                                request_id = %request_id,
                                status = %status,
                                duration_ms = %duration.as_millis(),
                                "Successful response"
                            );
                        }
                    }

                    // Add timing header if enabled
                    let mut response = response;
                    if config.enable_response_time_header {
                        let timing_value = format!("{}ms", duration.as_millis());
                        if let Ok(header_value) = HeaderValue::from_str(&timing_value) {
                            response
                                .headers_mut()
                                .insert(RESPONSE_TIME_HEADER, header_value);
                        }
                    }

                    // Ensure request ID is in response headers
                    if config.enable_request_id
                        && response.headers().get(REQUEST_ID_HEADER).is_none()
                    {
                        if let Ok(header_value) = HeaderValue::from_str(&request_id) {
                            response
                                .headers_mut()
                                .insert(REQUEST_ID_HEADER, header_value);
                        }
                    }

                    Ok(response)
                }
                Err(e) => {
                    error!(
                        request_id = %request_id,
                        error = %e,
                        duration_ms = %duration.as_millis(),
                        "Request failed"
                    );
                    Err(e)
                }
            }
        };

        Box::pin(future.instrument(span))
    }
}

/// Initialize tracing with custom configuration
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter("cms=debug,tower_http=debug,axum=debug")
        .with_target(true)
        .with_line_number(true)
        .try_init()
        .ok();
}

/// Initialize tracing with JSON output
pub fn init_tracing_json() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter("cms=debug")
        .try_init()
        .ok();
}

/// Add request ID to error response
///
/// This should be used in error handlers to ensure request ID is included
/// in error responses for correlation.
pub fn add_request_id_to_error<B>(mut response: Response<B>, request_id: &str) -> Response<B> {
    if response.headers().get(REQUEST_ID_HEADER).is_none() {
        if let Ok(header_value) = HeaderValue::from_str(request_id) {
            response
                .headers_mut()
                .insert(REQUEST_ID_HEADER, header_value);
        }
    }
    response
}

/// Start a Prometheus metrics exporter (optional, requires prometheus feature)
///
/// This is useful for production deployments with Prometheus monitoring.
///
/// # Example
/// ```rust,ignore
/// #[cfg(feature = "prometheus")]
/// observability::start_prometheus_exporter("0.0.0.0:9090").await?;
/// ```
#[cfg(feature = "prometheus")]
pub async fn start_prometheus_exporter(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::net::SocketAddr;

    let addr: SocketAddr = addr.parse()?;
    let builder = PrometheusBuilder::new().with_http_listener(addr);

    // Install the recorder
    builder.install()?;

    info!("Prometheus metrics exporter started on {}", addr);
    Ok(())
}

#[cfg(not(feature = "prometheus"))]
pub async fn start_prometheus_exporter(_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    warn!(
        "Prometheus exporter requested but prometheus feature is not enabled. Add `prometheus` \
         feature to cms-middleware in Cargo.toml"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::body::Body;

    use super::*;

    #[test]
    fn test_generate_request_id() {
        let id1 = generate_request_id();
        let id2 = generate_request_id();

        assert!(!id1.is_empty());
        assert_ne!(id1, id2);
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
    fn test_ensure_request_id() {
        let mut headers = HeaderMap::new();

        // Should add request ID
        let id1 = ensure_request_id(&mut headers);
        assert!(!id1.is_empty());
        assert_eq!(
            headers.get(REQUEST_ID_HEADER).unwrap().to_str().unwrap(),
            id1
        );

        // With existing ID - should not change
        let existing_id = "existing-123";
        headers.insert(REQUEST_ID_HEADER, HeaderValue::from_static(existing_id));
        let id2 = ensure_request_id(&mut headers);
        assert_eq!(id2, existing_id);
        assert_eq!(
            headers.get(REQUEST_ID_HEADER).unwrap().to_str().unwrap(),
            existing_id
        );
    }

    #[test]
    fn test_add_request_id_to_error() {
        let response = Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::empty())
            .unwrap();

        let response = add_request_id_to_error(response, "test-id");

        assert_eq!(
            response
                .headers()
                .get(REQUEST_ID_HEADER)
                .unwrap()
                .to_str()
                .unwrap(),
            "test-id"
        );
    }

    #[test]
    fn test_observability_config_default() {
        let config = ObservabilityConfig::default();

        assert!(config.enable_response_time_header);
        assert!(config.enable_request_id);
        assert!(config.enable_request_logging);
        assert!(config.enable_response_logging);
        assert!(config.enable_metrics);
    }
}
