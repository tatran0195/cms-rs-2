//! Analytics database queries

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use cms_entity::analytics::{AnalyticsDashboardResponse, AnalyticsEvent, TimeSeriesAnalytics};
use cms_error::AppError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

// Note: AnalyticsEvent is already defined in usage.rs
// This module provides additional analytics-specific queries

/// Analytics queries
pub struct AnalyticsQueries;

impl AnalyticsQueries {
    /// Get page view count for a project
    pub async fn get_page_view_count(
        pool: &PgPool,
        project_id: &str,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<i64, AppError> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) as count FROM \"AnalyticsEvent\" WHERE project_id = ",
        );
        query_builder.push_bind(project_id);
        query_builder.push(" AND event_type = 'page_view'");

        if let Some(start) = start_date {
            query_builder.push(" AND created_at >= ");
            query_builder.push_bind(start);
        }

        if let Some(end) = end_date {
            query_builder.push(" AND created_at <= ");
            query_builder.push_bind(end);
        }

        let count: i64 = query_builder
            .build()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?
            .get::<i64, _>("count");

        Ok(count)
    }

    /// Get search count for a project
    pub async fn get_search_count(
        pool: &PgPool,
        project_id: &str,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<i64, AppError> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) as count FROM \"AnalyticsEvent\" WHERE project_id = ",
        );
        query_builder.push_bind(project_id);
        query_builder.push(" AND event_type = 'search'");

        if let Some(start) = start_date {
            query_builder.push(" AND created_at >= ");
            query_builder.push_bind(start);
        }

        if let Some(end) = end_date {
            query_builder.push(" AND created_at <= ");
            query_builder.push_bind(end);
        }

        let count: i64 = query_builder
            .build()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?
            .get::<i64, _>("count");

        Ok(count)
    }

    /// Get unique user count for a project
    pub async fn get_unique_user_count(
        pool: &PgPool,
        project_id: &str,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<i64, AppError> {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT COUNT(DISTINCT user_id) as count FROM \"AnalyticsEvent\" WHERE project_id = ",
        );
        query_builder.push_bind(project_id);

        if let Some(start) = start_date {
            query_builder.push(" AND created_at >= ");
            query_builder.push_bind(start);
        }

        if let Some(end) = end_date {
            query_builder.push(" AND created_at <= ");
            query_builder.push_bind(end);
        }

        let count: i64 = query_builder
            .build()
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?
            .get::<i64, _>("count");

        Ok(count)
    }

    /// Get most viewed pages for a project
    pub async fn get_most_viewed_pages(
        pool: &PgPool,
        project_id: &str,
        limit: i64,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<Vec<(String, i64)>, AppError> {
        let mut qb = QueryBuilder::<Postgres>::new(
            "SELECT COALESCE(metadata->>'page_id', metadata->>'path', 'unknown') AS page_key, \
             COUNT(*) AS view_count FROM \"AnalyticsEvent\" WHERE project_id = ",
        );
        qb.push_bind(project_id);
        qb.push(" AND event_type = 'page_view'");

        if let Some(start) = start_date {
            qb.push(" AND created_at >= ");
            qb.push_bind(start);
        }
        if let Some(end) = end_date {
            qb.push(" AND created_at <= ");
            qb.push_bind(end);
        }

        qb.push(" GROUP BY page_key ORDER BY view_count DESC LIMIT ");
        qb.push_bind(limit);

        let rows = qb
            .build()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        let results = rows
            .into_iter()
            .map(|r| {
                (
                    r.get::<String, _>("page_key"),
                    r.get::<i64, _>("view_count"),
                )
            })
            .collect();

        Ok(results)
    }

    /// Get organization-level analytics summary
    pub async fn get_summary(
        pool: &PgPool,
        org_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<serde_json::Value, AppError> {
        let total_events: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM \"AnalyticsEvent\" WHERE organization_id = $1 AND created_at >= \
             $2 AND created_at <= $3",
        )
        .bind(org_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        let unique_users: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT user_id) FROM \"AnalyticsEvent\" WHERE organization_id = $1 \
             AND created_at >= $2 AND created_at <= $3 AND user_id IS NOT NULL",
        )
        .bind(org_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        let page_views: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM \"AnalyticsEvent\" WHERE organization_id = $1 AND event_type = \
             'page_view' AND created_at >= $2 AND created_at <= $3",
        )
        .bind(org_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        let searches: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM \"AnalyticsEvent\" WHERE organization_id = $1 AND event_type = \
             'search' AND created_at >= $2 AND created_at <= $3",
        )
        .bind(org_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(serde_json::json!({
            "total_events": total_events,
            "unique_users": unique_users,
            "page_views": page_views,
            "searches": searches,
        }))
    }

    /// Get project analytics dashboard
    pub async fn get_dashboard(
        pool: &PgPool,
        project_id: &str,
    ) -> Result<AnalyticsDashboardResponse, AppError> {
        let total_events: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM \"AnalyticsEvent\" WHERE project_id = $1")
                .bind(project_id)
                .fetch_one(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        let event_type_rows = sqlx::query(
            "SELECT event_type, COUNT(*) as count FROM \"AnalyticsEvent\" WHERE project_id = $1 \
             GROUP BY event_type",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        let mut events_by_type = HashMap::new();
        for row in event_type_rows {
            let event_type: String = row.get("event_type");
            let count: i64 = row.get("count");
            events_by_type.insert(event_type, count);
        }

        let time_series_rows = sqlx::query(
            "SELECT TO_CHAR(created_at, 'YYYY-MM-DD') AS dt, COUNT(*) AS cnt FROM \
             \"AnalyticsEvent\" WHERE project_id = $1 AND created_at >= NOW() - INTERVAL '30 \
             days' GROUP BY dt ORDER BY dt ASC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        let time_series = time_series_rows
            .into_iter()
            .map(|r| TimeSeriesAnalytics {
                date: r.get("dt"),
                count: r.get("cnt"),
            })
            .collect();

        Ok(AnalyticsDashboardResponse {
            total_events,
            events_by_type,
            time_series,
        })
    }

    /// Get view count and unique visitor count for a page
    pub async fn get_page_views(
        pool: &PgPool,
        page_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        let views: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM \"AnalyticsEvent\" WHERE event_type = 'page_view' AND \
             (metadata->>'page_id' = $1 OR metadata->>'id' = $1)",
        )
        .bind(page_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        let unique_visitors: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT COALESCE(user_id, ip_address)) FROM \"AnalyticsEvent\" WHERE \
             event_type = 'page_view' AND (metadata->>'page_id' = $1 OR metadata->>'id' = $1)",
        )
        .bind(page_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(serde_json::json!({
            "views": views,
            "unique_visitors": unique_visitors,
        }))
    }

    /// Get aggregate statistics for an organization
    pub async fn get_organization_stats(
        pool: &PgPool,
        org_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        let projects: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM \"Project\" WHERE organization_id = $1")
                .bind(org_id)
                .fetch_one(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        let members: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM \"Member\" WHERE organization_id = $1")
                .bind(org_id)
                .fetch_one(pool)
                .await
                .map_err(|e| AppError::Database(e.into()))?;

        let events: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM \"AnalyticsEvent\" WHERE organization_id = $1",
        )
        .bind(org_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(serde_json::json!({
            "projects": projects,
            "members": members,
            "events": events,
        }))
    }

    /// Get system-wide statistics
    pub async fn get_system_stats(pool: &PgPool) -> Result<serde_json::Value, AppError> {
        let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM \"User\"")
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        let organizations: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM \"Organization\"")
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        let projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM \"Project\"")
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;

        Ok(serde_json::json!({
            "users": users,
            "organizations": organizations,
            "projects": projects,
        }))
    }
}

/// Analytics event queries (alias for AnalyticsQueries for API compatibility)
///
/// Provides CRUD operations for analytics events in the AnalyticsEvent table.
pub struct AnalyticsEventQueries;

impl AnalyticsEventQueries {
    /// Create an analytics event
    pub async fn create(
        pool: &PgPool,
        org_id: Option<&str>,
        project_id: Option<&str>,
        user_id: Option<&str>,
        event_type: &str,
        metadata: serde_json::Value,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<cms_entity::analytics::AnalyticsEvent, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let row = sqlx::query_as::<_, AnalyticsEventRow>(
            r#"
            INSERT INTO "AnalyticsEvent" (id, organization_id, project_id, user_id, event_type, metadata, ip_address, user_agent, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(org_id)
        .bind(project_id)
        .bind(user_id)
        .bind(event_type)
        .bind(metadata)
        .bind(ip_address)
        .bind(user_agent)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(row.into())
    }

    /// List analytics events for a project
    pub async fn list_by_project(
        pool: &PgPool,
        project_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<cms_entity::analytics::AnalyticsEvent>, AppError> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let rows = sqlx::query_as::<_, AnalyticsEventRow>(
            r#"SELECT * FROM "AnalyticsEvent" WHERE project_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"#
        )
        .bind(project_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(e.into()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Query analytics events with flexible filters
    pub async fn query(
        pool: &PgPool,
        project_id: Option<&str>,
        user_id: Option<&str>,
        event_type: Option<&str>,
        start_date: Option<chrono::DateTime<chrono::Utc>>,
        end_date: Option<chrono::DateTime<chrono::Utc>>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<cms_entity::analytics::AnalyticsEvent>, AppError> {
        let mut builder =
            sqlx::QueryBuilder::<sqlx::Postgres>::new(r#"SELECT * FROM "AnalyticsEvent""#);
        let mut has_where = false;

        if let Some(v) = project_id {
            builder.push(if !has_where { " WHERE " } else { " AND " });
            builder.push("project_id = ");
            builder.push_bind(v);
            has_where = true;
        }
        if let Some(v) = user_id {
            builder.push(if !has_where { " WHERE " } else { " AND " });
            builder.push("user_id = ");
            builder.push_bind(v);
            has_where = true;
        }
        if let Some(v) = event_type {
            builder.push(if !has_where { " WHERE " } else { " AND " });
            builder.push("event_type = ");
            builder.push_bind(v);
            has_where = true;
        }
        if let Some(v) = start_date {
            builder.push(if !has_where { " WHERE " } else { " AND " });
            builder.push("created_at >= ");
            builder.push_bind(v);
            has_where = true;
        }
        if let Some(v) = end_date {
            builder.push(if !has_where { " WHERE " } else { " AND " });
            builder.push("created_at <= ");
            builder.push_bind(v);
            has_where = true;
        }

        builder.push(" ORDER BY created_at DESC LIMIT ");
        builder.push_bind(limit);
        builder.push(" OFFSET ");
        builder.push_bind(offset);

        let rows = builder
            .build_query_as::<AnalyticsEventRow>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(e.into()))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

/// Analytics event row from the database
#[derive(Debug, sqlx::FromRow)]
struct AnalyticsEventRow {
    id: String,
    organization_id: Option<String>,
    project_id: Option<String>,
    user_id: Option<String>,
    event_type: String,
    metadata: serde_json::Value,
    ip_address: Option<String>,
    user_agent: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<AnalyticsEventRow> for cms_entity::analytics::AnalyticsEvent {
    fn from(row: AnalyticsEventRow) -> Self {
        Self {
            id: row.id,
            organization_id: row.organization_id,
            project_id: row.project_id,
            user_id: row.user_id,
            event_type: row.event_type,
            metadata: row.metadata,
            ip_address: row.ip_address,
            user_agent: row.user_agent,
            created_at: row.created_at,
        }
    }
}
