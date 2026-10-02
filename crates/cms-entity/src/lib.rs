//! CMS Entity Types
//!
//! This crate contains wire-shape DTOs (request/response types) with zero
//! database dependency, following database-entity pattern.
//!
//! CMS uses a single crate since the Prisma-generated types already serve
//! as both storage and API shapes with no friction.
//!
//! This crate exists so that cms-api and any future codegen tooling
//! can depend on the same types without pulling in the database crate.

pub mod analytics;
pub mod asset;
pub mod auth;
pub mod branch;
pub mod comment;
pub mod common;
pub mod deployment;
pub mod domain;
pub mod email;
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
pub mod reader_access;
pub mod search;
pub mod theme;
pub mod usage;

// Re-export commonly used types
pub use analytics::*;
pub use asset::*;
pub use auth::*;
pub use branch::*;
pub use comment::*;
pub use common::*;
pub use deployment::*;
pub use domain::*;
pub use email::*;
pub use export::*;
pub use git::*;
pub use integration::*;
pub use language::*;
pub use mcp::*;
pub use notification::*;
pub use openapi::*;
pub use org::*;
pub use page::*;
pub use platform_event::*;
pub use project::*;
pub use reader_access::*;
pub use search::*;
pub use theme::*;
pub use usage::*;
