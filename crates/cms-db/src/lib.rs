#![allow(clippy::useless_conversion)]
#![allow(clippy::unnecessary_unwrap)]
#![allow(clippy::too_many_arguments)]

//! CMS Database Layer
//!
//! This crate provides hand-written SQLx queries organized by domain
//!
//! Each domain has its own module with query functions that return
//! either row structs from cms-entity or Result<T, AppError>.

pub mod analytics;
pub mod asset;
pub mod auth;
pub mod branch;
pub mod comment;
pub mod deployment;
pub mod domain;
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
pub mod search_index;
pub mod theme;
pub mod usage;

use cms_error::AppError;
pub use sqlx;
use sqlx::postgres::PgPoolOptions;

/// Database connection pool type
pub type PgPool = sqlx::PgPool;

/// Create a new database connection pool
pub async fn create_pool(database_url: &str) -> Result<PgPool, AppError> {
    let pool = PgPoolOptions::new()
        .max_connections(50)
        .min_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(15))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .test_before_acquire(true)
        .connect(database_url)
        .await
        .map_err(|e| {
            tracing::error!("Database connection failed: {}", e);
            AppError::DatabaseConnectionFailed
        })?;

    Ok(pool)
}

/// Run database migrations
///
/// This uses SQLx's migration runner. During the transition from Prisma,
/// we can use the existing Prisma migrations (which are plain SQL files)
/// or create new SQLx migrations.
pub async fn run_migrations(pool: &PgPool) -> Result<(), AppError> {
    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .map_err(|e| {
            tracing::error!("Migration failed: {}", e);
            AppError::Database(e.into())
        })?;

    Ok(())
}

/// Test database connection
pub async fn test_connection(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query("SELECT 1")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_migrations() {
        let url = std::env::var("CMS_DATABASE__URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/cms".to_string());
        if let Ok(pool) = create_pool(&url).await {
            let res = run_migrations(&pool).await;
            assert!(res.is_ok(), "Migration should succeed: {:?}", res.err());
        }
    }

    #[tokio::test]
    async fn test_query_count() {
        let url = std::env::var("CMS_DATABASE__URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/cms".to_string());
        if let Ok(pool) = create_pool(&url).await {
            let res = crate::notification::NotificationQueries::count_unread_by_user(&pool, "nonexistent-user").await;
            assert!(res.is_ok(), "Notification count should succeed: {:?}", res.err());

            let mem_res = crate::org::MemberQueries::get_by_user(&pool, "nonexistent-user").await;
            assert!(mem_res.is_ok(), "Member get should succeed: {:?}", mem_res.err());
        }
    }
}

