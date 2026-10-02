//! CMS Business Logic Layer
//!
//! This crate contains the business logic (use cases) for CMS, organized by domain.
//! Following AppFlowy's pattern, biz functions take already-resolved dependencies
//! (pool, identity, access-control) as parameters, making them unit-testable
//! without the web framework.
//!
//! Each domain module contains functions that orchestrate multi-step operations,
//! calling several database functions in sequence. This is not a behavior change
//! from the TypeScript actions, only a crate-boundary change.

pub mod analytics;
pub mod asset;
pub mod auth;
pub mod branch;
pub mod comment;
pub mod deployment;
pub mod domain;
pub mod email;
pub mod entitlement;
pub mod export;
pub mod git;
pub mod integration;
pub mod language;
pub mod mcp;
pub mod notification;
pub mod openapi;
pub mod org;
pub mod page;
pub mod platform_event;
pub mod project;
pub mod queue;
pub mod reader_access;
pub mod search;
pub mod theme;
pub mod usage;

// Re-export commonly used types
pub use cms_db::PgPool;
pub use cms_entity::*;
pub use cms_error::AppError;

/// Business context passed to most biz functions
///
/// This struct contains the common dependencies needed by business logic functions.
#[derive(Clone)]
pub struct BizContext {
    pub pool: PgPool,
    pub authz: std::sync::Arc<dyn cms_authz::Authz>,
}

impl BizContext {
    /// Create a new BizContext
    pub fn new(
        pool: PgPool,
        authz: std::sync::Arc<dyn cms_authz::Authz>,
    ) -> Self {
        Self {
            pool,
            authz,
        }
    }
}
