//! CMS API Layer
//!
//! This crate contains Axum routers and handlers for the CMS REST API.
//! Handlers are thin and delegate to biz functions.
//!
//! The API is organized by domain, matching the structure of `apps/server/src/modules/app`
//! in the original TypeScript monorepo.

pub mod admin;
pub mod analytics;
pub mod asset;
pub mod auth;
pub mod branch;
pub mod comment;
pub mod deployment;
pub mod domain;
pub mod export;
pub mod extractors;
pub mod git;
pub mod integration;
pub mod language;
pub mod mcp;
pub mod middleware;
pub mod notification;
pub mod openapi;
pub mod org;
pub mod page;
pub mod platform_event;
pub mod project;
pub mod public;
pub mod reader_access;
pub mod search;
pub mod theme;
pub mod usage;
pub mod validation;
pub mod workspace;
use std::sync::Arc;

use axum::Router;
use cms_error::AppError;
pub use openapi::docs as openapi_docs;

/// AppState type - this will be provided by the binary crate
pub type AppState = cms_middleware::AppState;

/// Create the main API router
pub fn create_api_router(state: Arc<AppState>) -> Router {
    use axum::routing::{get, post};
    use openapi::docs::{serve_openapi_spec, serve_openapi_yaml, serve_redoc, serve_swagger_ui};

    let mut router = Router::new();

    // Mount all domain routers
    router = router.nest("/auth", auth::router(state.clone()));
    router = router.nest("/orgs", org::router(state.clone()));
    router = router.nest("/projects", project::router(state.clone()));
    router = router.nest("/pages", page::router(state.clone()));
    router = router.nest("/branches", branch::router(state.clone()));
    router = router.nest("/languages", language::router(state.clone()));
    router = router.nest("/git", git::router(state.clone()));
    router = router.nest("/integrations", integration::router(state.clone()));
    router = router.nest("/deployments", deployment::router(state.clone()));
    router = router.nest("/domains", domain::router(state.clone()));
    router = router.nest("/reader-access", reader_access::router(state.clone()));
    router = router.nest("/comments", comment::router(state.clone()));
    router = router.nest("/search", search::router(state.clone()));
    router = router.nest("/export", export::router(state.clone()));
    router = router.nest("/openapi", openapi::router(state.clone()));
    router = router.nest("/usage", usage::router(state.clone()));
    router = router.nest("/notifications", notification::router(state.clone()));
    router = router.nest("/assets", asset::router(state.clone()));
    router = router.nest("/analytics", analytics::router(state.clone()));
    router = router.nest("/themes", theme::router(state.clone()));
    router = router.nest("/platform-events", platform_event::router(state.clone()));
    router = router.nest("/mcp", mcp::router(state.clone()));

    // App subrouter (mirrors original TypeScript /api/app/* routes)
    let app_router = Router::new()
        .nest("/projects", project::router(state.clone()))
        .nest("/pages", page::router(state.clone()))
        .nest("/branches", branch::router(state.clone()))
        .nest("/languages", language::router(state.clone()))
        .nest("/git", git::router(state.clone()))
        .nest("/integrations", integration::router(state.clone()))
        .nest("/deployments", deployment::router(state.clone()))
        .nest("/domains", domain::router(state.clone()))
        .nest("/reader-access", reader_access::router(state.clone()))
        .nest("/comments", comment::router(state.clone()))
        .nest("/search", search::router(state.clone()))
        .nest("/export", export::router(state.clone()))
        .nest("/openapi", openapi::router(state.clone()))
        .nest("/usage", usage::router(state.clone()))
        .nest("/notifications", notification::router(state.clone()))
        .nest("/assets", asset::router(state.clone()))
        .nest("/analytics", analytics::router(state.clone()))
        .nest("/themes", theme::router(state.clone()))
        .nest("/platform-events", platform_event::router(state.clone()))
        .nest("/mcp", mcp::router(state.clone()))
        .nest("/members", workspace::members_router(state.clone()))
        .nest("/workspace", workspace::workspace_router(state.clone()));
    router = router.nest("/app", app_router);

    // Admin routes (platform-admin-only)
    router = router.nest("/admin", admin::router(state.clone()));

    // Public routes (unauthenticated reader-facing JSON API)
    router = router.nest("/public", public::router(state.clone()));

    // OpenAPI documentation endpoints
    router = router.route("/api-docs/openapi.json", get(serve_openapi_spec));
    router = router.route("/api-docs/openapi.yaml", get(serve_openapi_yaml));
    router = router.route("/api-docs", get(serve_swagger_ui));
    router = router.route("/api-docs/redoc", get(serve_redoc));

    // Health check endpoint
    router = router.route("/health", get(get_health).with_state(state));

    router
}

/// Alias for create_api_router (for compatibility with main.rs)
pub fn api_router(state: Arc<AppState>) -> Router {
    middleware::create_api_router_with_middleware(state)
}

/// Health check handler
async fn get_health(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let health =
        cms_biz::platform_event::PlatformEventService::get_system_health(&state.biz_context)
            .await?;
    Ok(axum::Json(health))
}
