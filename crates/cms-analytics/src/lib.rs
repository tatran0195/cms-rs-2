//! CMS Analytics
//!
//! This crate provides analytics storage with:
//! - Postgres as default backend (delegates to cms_db::analytics)
//! - ClickHouse as optional backend

use std::sync::Arc;

use async_trait::async_trait;
use cms_config::AnalyticsConfig;
use cms_db::{analytics::AnalyticsEventQueries, PgPool};
use cms_error::AppError;

/// Analytics store trait
#[allow(clippy::too_many_arguments)]
#[async_trait]
pub trait AnalyticsStore: Send + Sync {
    /// Record an analytics event
    async fn record_event(
        &self,
        org_id: Option<&str>,
        project_id: Option<&str>,
        user_id: Option<&str>,
        event_type: &str,
        metadata: serde_json::Value,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(), AppError>;

    /// Query analytics events
    async fn query_events(
        &self,
        org_id: Option<&str>,
        project_id: Option<&str>,
        user_id: Option<&str>,
        event_type: Option<&str>,
        start_date: Option<chrono::DateTime<chrono::Utc>>,
        end_date: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<cms_entity::analytics::AnalyticsEvent>, AppError>;

    /// Get summary statistics
    async fn get_summary(
        &self,
        org_id: &str,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<serde_json::Value, AppError>;
}

/// Create an AnalyticsStore implementation based on configuration.
///
/// The `pool` argument is required for the Postgres backend. Pass the
/// application's PgPool here.
pub async fn create_analytics_store(
    config: &AnalyticsConfig,
    pool: PgPool,
) -> Result<Arc<dyn AnalyticsStore>, AppError> {
    match config.backend.as_str() {
        "postgres" => {
            let store = PostgresAnalyticsStore::new(pool);
            Ok(Arc::new(store))
        }
        "clickhouse" => {
            #[cfg(feature = "clickhouse")]
            {
                let store = ClickHouseAnalyticsStore::new(
                    config.clickhouse_host.clone().unwrap_or_default(),
                    config.clickhouse_port,
                    config.clickhouse_database.clone().unwrap_or_default(),
                    config.clickhouse_username.clone(),
                    config.clickhouse_password.clone(),
                )
                .await?;
                Ok(Arc::new(store))
            }
            #[cfg(not(feature = "clickhouse"))]
            {
                Err(AppError::Storage(
                    "ClickHouse backend requires the 'clickhouse' feature".to_string(),
                ))
            }
        }
        _ => Err(AppError::Storage(format!(
            "Unknown analytics backend: {}",
            config.backend
        ))),
    }
}

// ---------------------------------------------------------------------------
// Postgres backend
// ---------------------------------------------------------------------------

/// Postgres analytics store — delegates to `cms_db::analytics::AnalyticsEventQueries`.
pub struct PostgresAnalyticsStore {
    pool: PgPool,
}

impl PostgresAnalyticsStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AnalyticsStore for PostgresAnalyticsStore {
    async fn record_event(
        &self,
        org_id: Option<&str>,
        project_id: Option<&str>,
        user_id: Option<&str>,
        event_type: &str,
        metadata: serde_json::Value,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(), AppError> {
        AnalyticsEventQueries::create(
            &self.pool, org_id, project_id, user_id, event_type, metadata, ip_address, user_agent,
        )
        .await?;
        Ok(())
    }

    async fn query_events(
        &self,
        _org_id: Option<&str>,
        project_id: Option<&str>,
        user_id: Option<&str>,
        event_type: Option<&str>,
        start_date: Option<chrono::DateTime<chrono::Utc>>,
        end_date: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<cms_entity::analytics::AnalyticsEvent>, AppError> {
        AnalyticsEventQueries::query(
            &self.pool,
            project_id,
            user_id,
            event_type,
            start_date,
            end_date,
            limit.unwrap_or(100),
            offset.unwrap_or(0),
        )
        .await
    }

    async fn get_summary(
        &self,
        org_id: &str,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<serde_json::Value, AppError> {
        use cms_db::analytics::AnalyticsQueries;

        let total_events = AnalyticsEventQueries::query(
            &self.pool,
            None,
            None,
            None,
            Some(start_date),
            Some(end_date),
            1000,
            0,
        )
        .await
        .map(|v| v.len() as i64)
        .unwrap_or(0);

        let page_views = AnalyticsQueries::get_page_view_count(
            &self.pool,
            org_id,
            Some(start_date),
            Some(end_date),
        )
        .await
        .unwrap_or(0);

        let unique_users = AnalyticsQueries::get_unique_user_count(
            &self.pool,
            org_id,
            Some(start_date),
            Some(end_date),
        )
        .await
        .unwrap_or(0);

        let searches = AnalyticsQueries::get_search_count(
            &self.pool,
            org_id,
            Some(start_date),
            Some(end_date),
        )
        .await
        .unwrap_or(0);

        Ok(serde_json::json!({
            "total_events": total_events,
            "unique_users": unique_users,
            "page_views": page_views,
            "searches": searches,
        }))
    }
}

// ---------------------------------------------------------------------------
// ClickHouse backend (feature-gated)
// ---------------------------------------------------------------------------

/// ClickHouse analytics store (optional — enable with the `clickhouse` feature)
#[cfg(feature = "clickhouse")]
pub struct ClickHouseAnalyticsStore;

#[cfg(feature = "clickhouse")]
impl ClickHouseAnalyticsStore {
    pub async fn new(
        _host: String,
        _port: u16,
        _database: String,
        _username: Option<String>,
        _password: Option<String>,
    ) -> Result<Self, AppError> {
        // ClickHouse integration is not yet implemented.
        // Tracked as future work — requires the `clickhouse` crate.
        Err(AppError::Storage(
            "ClickHouse backend is not yet implemented".to_string(),
        ))
    }
}

#[cfg(feature = "clickhouse")]
#[async_trait]
impl AnalyticsStore for ClickHouseAnalyticsStore {
    async fn record_event(
        &self,
        _org_id: Option<&str>,
        _project_id: Option<&str>,
        _user_id: Option<&str>,
        _event_type: &str,
        _metadata: serde_json::Value,
        _ip_address: Option<&str>,
        _user_agent: Option<&str>,
    ) -> Result<(), AppError> {
        Err(AppError::Storage(
            "ClickHouse backend is not yet implemented".to_string(),
        ))
    }

    async fn query_events(
        &self,
        _org_id: Option<&str>,
        _project_id: Option<&str>,
        _user_id: Option<&str>,
        _event_type: Option<&str>,
        _start_date: Option<chrono::DateTime<chrono::Utc>>,
        _end_date: Option<chrono::DateTime<chrono::Utc>>,
        _limit: Option<i64>,
        _offset: Option<i64>,
    ) -> Result<Vec<cms_entity::analytics::AnalyticsEvent>, AppError> {
        Err(AppError::Storage(
            "ClickHouse backend is not yet implemented".to_string(),
        ))
    }

    async fn get_summary(
        &self,
        _org_id: &str,
        _start_date: chrono::DateTime<chrono::Utc>,
        _end_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::Storage(
            "ClickHouse backend is not yet implemented".to_string(),
        ))
    }
}
