//! CMS Server - Composition Root
//!
//! This is the main entry point for the cms-server binary.
//! It composes all the crates together and starts the Axum HTTP server.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{http::StatusCode, Router};
use cms_api::api_router;
use cms_config::Config;
use cms_error::AppError;
use cms_middleware::{app_state::AppState, observability::init_tracing};
use cms_sites::sites_router;
use cms_worker::app_state::WorkerState;
use tokio::net::TcpListener;
use tracing::info;

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
    tracing::info!("Received graceful shutdown signal. Starting shutdown...");
}

/// Build a CORS layer from the configured admin/app origin allow-list.
///
/// Requests from the SPA itself are same-origin and need no CORS. This layer
/// enables credentialed cross-origin calls (e.g. an admin console hosted on a
/// different origin) only for origins listed in `[admin_origin]`. Localhost
/// origins are also permitted in development when `allow_localhost` is set.
fn build_cors(config: &cms_config::Config) -> tower_http::cors::CorsLayer {
    use axum::http::{header, HeaderName, Method};
    use tower_http::cors::{AllowOrigin, CorsLayer};

    let allow_localhost = config.admin_origin.allow_localhost;
    let allowed = config.admin_origin.allowed_origins.clone();

    let allow_origin = AllowOrigin::predicate(move |origin: &axum::http::HeaderValue, _| {
        let Some(origin) = origin.to_str().ok() else {
            return false;
        };
        if allowed.iter().any(|a| a == origin) {
            return true;
        }
        if allow_localhost {
            let is_localhost = origin.starts_with("http://localhost:")
                || origin.starts_with("http://127.0.0.1:")
                || origin.starts_with("http://[::1]:");
            if is_localhost {
                return true;
            }
        }
        false
    });

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
            Method::HEAD,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::ORIGIN,
            header::COOKIE,
            header::SET_COOKIE,
            header::CONTENT_LENGTH,
            HeaderName::from_static("x-requested-with"),
            HeaderName::from_static("x-api-key"),
            HeaderName::from_static("x-locale"),
        ])
        .max_age(std::time::Duration::from_secs(86400))
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Initialize observability (logging, tracing, metrics)
    init_tracing();

    info!("Starting CMS Server...");

    // Load configuration
    let config = Config::load()?;
    info!("Configuration loaded");

    // Build application state
    let state = Arc::new(AppState::from_config(&config).await?);
    info!("Application state initialized");

    // Run real job processors over the same durable queue used by API handlers.
    // The external cms-worker binary can be scaled horizontally against the same
    // PostgreSQL queue; FOR UPDATE SKIP LOCKED prevents duplicate claims.
    let worker_state = Arc::new(WorkerState::from_app_state(&state).await?);
    let (worker_shutdown_tx, worker_shutdown_rx) = tokio::sync::watch::channel(false);
    cms_worker::start_consumers_with_shutdown(
        state.job_queue.clone(),
        worker_state,
        worker_shutdown_rx,
    )
    .await?;

    // Build the full router with all middleware
    let api_router = api_router(state.clone());
    let sites_router = sites_router(state.clone());

    // CORS configured from the `[admin_origin]` allow-list rather than the wider
    // accepted-by-any-origin default. The SPA is served same-origin, so browser
    // requests do not need CORS, but an admin/app console on another origin must
    // still be allowed with credentials. Localhost origins are permitted in dev.
    let cors = build_cors(&config);

    let app = Router::new()
        .nest("/api", api_router)
        .merge(sites_router)
        .layer(axum::Extension(state.clone()))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(tower_http::compression::CompressionLayer::new())
        .layer(cors)
        .layer(tower_http::catch_panic::CatchPanicLayer::new())
        .layer(tower_http::timeout::TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ));

    // Bind to socket
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            let _ = worker_shutdown_tx.send(true);
        })
        .await
        .map_err(|e| AppError::Internal(e.into()))
}
