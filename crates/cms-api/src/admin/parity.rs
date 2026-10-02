//! cms admin-console compatibility surface, backed by cms-rs records.
//! Sensitive content, credentials, raw provider errors, and session tokens are
//! intentionally not returned to operators through these summary endpoints.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, Postgres, QueryBuilder};

use crate::auth::AuthExtractor;

async fn require_admin(state: &AppState, user_id: &str) -> Result<(), AppError> {
    state.biz_context.authz.require_system_admin(user_id).await
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct OverviewRow {
    users: i64,
    admins: i64,
    sites: i64,
    deployments: i64,
    published_deployments: i64,
    recent_users: i64,
    verified_users: i64,
    suspended_users: i64,
    failed_deployments_24h: i64,
    active_deployments: i64,
    domains: i64,
    healthy_domains: i64,
    domain_issues: i64,
    taken_down_sites: i64,
    expired_owner_invites: i64,
    failed_exports_7d: i64,
    git_issues: i64,
}

pub async fn overview_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<Value>, AppError> {
    require_admin(&state, &auth.user.id).await?;
    let stats = sqlx::query_as::<_, OverviewRow>(
        r#"SELECT
            (SELECT COUNT(*) FROM "User") AS users,
            (SELECT COUNT(*) FROM "User" WHERE role = 'admin') AS admins,
            (SELECT COUNT(*) FROM "Project") AS sites,
            (SELECT COUNT(*) FROM "Deployment") AS deployments,
            (SELECT COUNT(*) FROM "Deployment" WHERE status = 'ACTIVE') AS published_deployments,
            (SELECT COUNT(*) FROM "User" WHERE created_at >= NOW() - INTERVAL '7 days') AS recent_users,
            (SELECT COUNT(*) FROM "User" WHERE email_verified) AS verified_users,
            (SELECT COUNT(*) FROM "User" WHERE suspended_at IS NOT NULL) AS suspended_users,
            (SELECT COUNT(*) FROM "Deployment"
             WHERE status = 'FAILED' AND created_at >= NOW() - INTERVAL '24 hours') AS failed_deployments_24h,
            (SELECT COUNT(*) FROM "Deployment"
             WHERE status IN ('PENDING', 'BUILDING', 'DEPLOYING')) AS active_deployments,
            (SELECT COUNT(*) FROM "Domain") AS domains,
            (SELECT COUNT(*) FROM "Domain"
             WHERE verified_at IS NOT NULL AND ssl_status = 'ACTIVE'
               AND (ssl_certificate_expires_at > NOW()
                    OR (ssl_certificate_expires_at IS NULL
                        AND ssl_checked_at >= NOW() - INTERVAL '7 days'))) AS healthy_domains,
            (SELECT COUNT(*) FROM "Domain"
             WHERE ssl_status IN ('ERROR', 'EXPIRED')) AS domain_issues,
            (SELECT COUNT(*) FROM "Project" WHERE takedown_at IS NOT NULL) AS taken_down_sites,
            (SELECT COUNT(*) FROM "Invitation"
             WHERE role = 'OWNER' AND expires_at < NOW()) AS expired_owner_invites,
            (SELECT COUNT(*) FROM "ExportJob"
             WHERE status = 'FAILED' AND created_at >= NOW() - INTERVAL '7 days') AS failed_exports_7d,
            (SELECT COUNT(*) FROM "GitSyncOperation"
             WHERE status IN ('FAILED', 'CONFLICT')) AS git_issues"#,
    )
    .fetch_one(&state.biz_context.pool)
    .await?;
    Ok(Json(json!({ "data": stats })))
}

#[derive(Debug, FromRow)]
struct FunnelRow {
    signups: i64,
    edited: i64,
    published: i64,
    ready: i64,
    ready_within_24_hours: i64,
    median_hours_to_ready: Option<f64>,
}

#[derive(Deserialize)]
pub struct FunnelQuery {
    days: Option<i64>,
}

pub async fn funnel_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<FunnelQuery>,
) -> Result<Json<Value>, AppError> {
    require_admin(&state, &auth.user.id).await?;
    let days = query.days.unwrap_or(30).clamp(1, 365);
    let row = sqlx::query_as::<_, FunnelRow>(
        r#"WITH events AS (
               SELECT event_type, user_id, created_at, metadata
               FROM "PlatformEvent"
               WHERE created_at >= NOW() - ($1::INTEGER * INTERVAL '1 day')
           ), signups AS (
               SELECT user_id, MIN(created_at) AS signup_at
               FROM events WHERE event_type = 'signup_completed' AND user_id IS NOT NULL
               GROUP BY user_id
           ), ready AS (
               SELECT user_id, MIN(created_at) AS ready_at
               FROM events
               WHERE event_type = 'publish_ready' AND user_id IS NOT NULL
                 AND COALESCE(metadata->>'auto', 'false') = 'false'
               GROUP BY user_id
           )
           SELECT
             COUNT(DISTINCT user_id) FILTER (WHERE event_type = 'signup_completed') AS signups,
             COUNT(DISTINCT user_id) FILTER (WHERE event_type = 'page_edited') AS edited,
             COUNT(DISTINCT user_id) FILTER (
                 WHERE event_type = 'publish_clicked'
                   AND COALESCE(metadata->>'auto', 'false') = 'false'
             ) AS published,
             COUNT(DISTINCT user_id) FILTER (
                 WHERE event_type = 'publish_ready'
                   AND COALESCE(metadata->>'auto', 'false') = 'false'
             ) AS ready,
             (SELECT COUNT(*) FROM signups s JOIN ready r USING (user_id)
              WHERE r.ready_at >= s.signup_at
                AND r.ready_at <= s.signup_at + INTERVAL '24 hours') AS ready_within_24_hours,
             (SELECT percentile_cont(0.5) WITHIN GROUP (
                 ORDER BY EXTRACT(EPOCH FROM (r.ready_at - s.signup_at)) / 3600.0
              ) FROM signups s JOIN ready r USING (user_id)
              WHERE r.ready_at >= s.signup_at) AS median_hours_to_ready
           FROM events"#,
    )
    .bind(days as i32)
    .fetch_one(&state.biz_context.pool)
    .await?;
    Ok(Json(json!({
        "data": {
            "days": days,
            "signups": row.signups,
            "edited": row.edited,
            "published": row.published,
            "ready": row.ready,
            "readyWithin24Hours": row.ready_within_24_hours,
            "medianHoursToReady": row.median_hours_to_ready,
        }
    })))
}

#[derive(Debug, FromRow)]
struct AdminUserRow {
    id: String,
    name: Option<String>,
    email: String,
    role: String,
    email_verified: bool,
    suspended_at: Option<DateTime<Utc>>,
    workspaces: i64,
    providers: Vec<String>,
    active_sessions: i64,
    last_session_created_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

async fn load_admin_users(
    state: &AppState,
    user_id: Option<&str>,
    limit: i64,
) -> Result<Vec<AdminUserRow>, AppError> {
    let mut query: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"SELECT u.id, u.name, u.email, u.role, u.email_verified, u.suspended_at,
                  u.created_at, u.updated_at,
                  (SELECT COUNT(*) FROM "Member" m WHERE m.user_id = u.id) AS workspaces,
                  COALESCE((SELECT ARRAY_AGG(DISTINCT a.provider ORDER BY a.provider)
                            FROM "Account" a WHERE a.user_id = u.id), ARRAY[]::TEXT[]) AS providers,
                  (SELECT COUNT(*) FROM "Session" s
                   WHERE s.user_id = u.id AND s.expires_at > NOW()) AS active_sessions,
                  (SELECT MAX(s.created_at) FROM "Session" s WHERE s.user_id = u.id) AS last_session_created_at
           FROM "User" u"#,
    );
    if let Some(user_id) = user_id {
        query.push(" WHERE u.id = ").push_bind(user_id);
    }
    query
        .push(" ORDER BY u.created_at DESC LIMIT ")
        .push_bind(limit.clamp(1, 500));
    Ok(query
        .build_query_as::<AdminUserRow>()
        .fetch_all(&state.biz_context.pool)
        .await?)
}

fn admin_user_json(user: AdminUserRow) -> Value {
    json!({
        "id": user.id,
        "name": user.name,
        "email": user.email,
        "role": user.role,
        "emailVerified": user.email_verified,
        "suspendedAt": user.suspended_at,
        "workspaces": user.workspaces,
        "providers": user.providers,
        "activeSessions": user.active_sessions,
        "lastSessionCreatedAt": user.last_session_created_at,
        "createdAt": user.created_at,
        "updatedAt": user.updated_at,
    })
}

pub async fn users_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<Value>, AppError> {
    require_admin(&state, &auth.user.id).await?;
    let users = load_admin_users(&state, None, 500).await?;
    Ok(Json(
        json!({ "data": users.into_iter().map(admin_user_json).collect::<Vec<_>>() }),
    ))
}

pub async fn user_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(user_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&state, &auth.user.id).await?;
    let user = load_admin_users(&state, Some(&user_id), 1)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    let memberships = sqlx::query(
        r#"SELECT m.id AS membership_id, m.role::TEXT AS role, m.created_at,
                  o.id AS organization_id, o.name AS organization_name,
                  active_plan.plan,
                  COALESCE(projects.project_count, 0)::BIGINT AS project_count,
                  COALESCE(projects.items, '[]'::JSONB) AS projects
           FROM "Member" m
           JOIN "Organization" o ON o.id = m.organization_id
           LEFT JOIN LATERAL (
               SELECT jsonb_build_object(
                   'name', plan.name,
                   'billingPeriod', plan.billing_period,
                   'startsAt', assigned.starts_at,
                   'endsAt', assigned.ends_at
               ) AS plan
               FROM "OrganizationUsagePlan" assigned
               JOIN "UsagePlan" plan ON plan.id = assigned.usage_plan_id
               WHERE assigned.organization_id = o.id
                 AND assigned.status = 'ACTIVE'
                 AND assigned.starts_at <= NOW()
                 AND (assigned.ends_at IS NULL OR assigned.ends_at > NOW())
               ORDER BY assigned.starts_at DESC LIMIT 1
           ) active_plan ON TRUE
           LEFT JOIN LATERAL (
               SELECT COUNT(*) AS project_count,
                      jsonb_agg(jsonb_build_object(
                          'id', project.id, 'name', project.name, 'slug', project.slug,
                          'takedownAt', project.takedown_at
                      ) ORDER BY project.created_at) AS items
               FROM "Project" project
               WHERE project.organization_id = o.id
           ) projects ON TRUE
           WHERE m.user_id = $1
           ORDER BY m.created_at DESC LIMIT 500"#,
    )
    .bind(&user_id)
    .fetch_all(&state.biz_context.pool)
    .await?
    .into_iter()
    .map(|row| {
        json!({
            "membershipId": row.get::<String, _>("membership_id"),
            "organizationId": row.get::<String, _>("organization_id"),
            "organizationName": row.get::<String, _>("organization_name"),
            "role": row.get::<String, _>("role"),
            "joinedAt": row.get::<DateTime<Utc>, _>("created_at"),
            "plan": row.get::<Option<Value>, _>("plan"),
            "projectCount": row.get::<i64, _>("project_count"),
            "projects": row.get::<Value, _>("projects"),
        })
    })
    .collect::<Vec<_>>();
    let activity = sqlx::query(
        r#"SELECT id, event_type, organization_id, created_at
           FROM "PlatformEvent" WHERE user_id = $1
           ORDER BY created_at DESC LIMIT 30"#,
    )
    .bind(&user_id)
    .fetch_all(&state.biz_context.pool)
    .await?
    .into_iter()
    .map(|row| {
        json!({
            "id": row.get::<String, _>("id"),
            "type": row.get::<String, _>("event_type"),
            "organizationId": row.try_get::<Option<String>, _>("organization_id").ok().flatten(),
            "createdAt": row.get::<DateTime<Utc>, _>("created_at"),
        })
    })
    .collect::<Vec<_>>();

    let mut data = admin_user_json(user);
    data["workspaces"] = json!(memberships);
    data["activity"] = json!(activity);
    Ok(Json(json!({ "data": data })))
}

#[derive(Deserialize)]
pub struct SetRoleBody {
    role: String,
}

pub async fn set_user_role_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(user_id): Path<String>,
    Json(body): Json<SetRoleBody>,
) -> Result<Json<Value>, AppError> {
    require_admin(&state, &auth.user.id).await?;
    let requested = body.role.trim().to_ascii_lowercase();
    if !matches!(requested.as_str(), "user" | "admin") {
        return Err(AppError::InvalidInput(
            "role must be 'user' or 'admin'".to_string(),
        ));
    }
    let mut tx = state.biz_context.pool.begin().await?;
    // Serialize administrator-roster updates so concurrent demotions cannot
    // both observe themselves as non-final administrators.
    sqlx::query("SELECT pg_advisory_xact_lock(78098457211712)")
        .execute(&mut *tx)
        .await?;
    let target = sqlx::query_as::<_, (String, String)>(
        r#"SELECT email, role FROM "User" WHERE id = $1 FOR UPDATE"#,
    )
    .bind(&user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    if requested == "user"
        && state
            .config
            .auth
            .system_admin_emails
            .iter()
            .any(|email| email.trim().eq_ignore_ascii_case(&target.0))
    {
        return Err(AppError::Conflict(
            "Configured system administrators cannot be demoted".to_string(),
        ));
    }
    if requested == "user" && target.1 == "admin" {
        let admins: i64 = sqlx::query_scalar(r#"SELECT COUNT(*) FROM "User" WHERE role = 'admin'"#)
            .fetch_one(&mut *tx)
            .await?;
        if admins <= 1 {
            return Err(AppError::Conflict(
                "Cannot remove the last system administrator".to_string(),
            ));
        }
    }
    sqlx::query(r#"UPDATE "User" SET role = $2, updated_at = NOW() WHERE id = $1"#)
        .bind(&user_id)
        .bind(&requested)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({ "data": { "ok": true } })))
}

pub async fn suspend_user_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(user_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&state, &auth.user.id).await?;
    let mut tx = state.biz_context.pool.begin().await?;
    let target_role: Option<String> =
        sqlx::query_scalar(r#"SELECT role FROM "User" WHERE id = $1 FOR UPDATE"#)
            .bind(&user_id)
            .fetch_optional(&mut *tx)
            .await?;
    let target_role =
        target_role.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    if target_role == "admin" {
        return Err(AppError::Conflict(
            "System administrators cannot be suspended".to_string(),
        ));
    }
    sqlx::query(r#"UPDATE "User" SET suspended_at = COALESCE(suspended_at, NOW()), updated_at = NOW() WHERE id = $1"#)
        .bind(&user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(r#"DELETE FROM "Session" WHERE user_id = $1"#)
        .bind(&user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({ "data": { "ok": true } })))
}

pub async fn unsuspend_user_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(user_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&state, &auth.user.id).await?;
    let result =
        sqlx::query(r#"UPDATE "User" SET suspended_at = NULL, updated_at = NOW() WHERE id = $1"#)
            .bind(&user_id)
            .execute(&state.biz_context.pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("User not found".to_string()));
    }
    Ok(Json(json!({ "data": { "ok": true } })))
}

#[derive(Debug, FromRow)]
struct AdminSiteRow {
    id: String,
    name: String,
    slug: String,
    description: Option<String>,
    organization_id: String,
    organization_name: String,
    owner_email: Option<String>,
    owner_suspended_at: Option<DateTime<Utc>>,
    owner_invitation_id: Option<String>,
    owner_invitation_email: Option<String>,
    plan: Option<String>,
    pages: i64,
    deployments: i64,
    languages: i64,
    members: i64,
    domains: i64,
    domain_issues: i64,
    is_public: bool,
    takedown_at: Option<DateTime<Utc>>,
    takedown_reason: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    latest_deployment: Option<Value>,
}

async fn load_admin_sites(
    state: &AppState,
    project_id: Option<&str>,
) -> Result<Vec<AdminSiteRow>, AppError> {
    let mut query: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"SELECT p.id, p.name, p.slug, p.description, p.organization_id,
                  o.name AS organization_name, p.is_public, p.takedown_at,
                  p.takedown_reason, p.created_at, p.updated_at,
                  owner_member.email AS owner_email,
                  owner_member.suspended_at AS owner_suspended_at,
                  (SELECT i.id FROM "Invitation" i
                   WHERE i.organization_id = p.organization_id AND i.role = 'OWNER'
                     AND i.expires_at > NOW() ORDER BY i.created_at LIMIT 1) AS owner_invitation_id,
                  (SELECT i.email FROM "Invitation" i
                   WHERE i.organization_id = p.organization_id AND i.role = 'OWNER'
                     AND i.expires_at > NOW() ORDER BY i.created_at LIMIT 1) AS owner_invitation_email,
                  (SELECT plan.name FROM "OrganizationUsagePlan" assigned
                   JOIN "UsagePlan" plan ON plan.id = assigned.usage_plan_id
                   WHERE assigned.organization_id = p.organization_id AND assigned.status = 'ACTIVE'
                     AND assigned.starts_at <= NOW()
                     AND (assigned.ends_at IS NULL OR assigned.ends_at > NOW())
                   ORDER BY assigned.starts_at DESC LIMIT 1) AS plan,
                  (SELECT COUNT(*) FROM "Page" pg
                   WHERE pg.project_id = p.id AND UPPER(pg.kind) = 'PAGE') AS pages,
                  (SELECT COUNT(*) FROM "Deployment" d WHERE d.project_id = p.id) AS deployments,
                  (SELECT COUNT(*) FROM "Language" l WHERE l.project_id = p.id) AS languages,
                  (SELECT COUNT(*) FROM "Member" m WHERE m.organization_id = p.organization_id) AS members,
                  (SELECT COUNT(*) FROM "Domain" dm JOIN "Deployment" d ON d.id = dm.deployment_id
                   WHERE d.project_id = p.id) AS domains,
                  (SELECT COUNT(*) FROM "Domain" dm JOIN "Deployment" d ON d.id = dm.deployment_id
                   WHERE d.project_id = p.id AND dm.ssl_status IN ('ERROR', 'EXPIRED')) AS domain_issues,
                  (SELECT jsonb_build_object(
                       'id', d.id,
                       'version', snap.version,
                       'status', d.status::TEXT,
                       'at', COALESCE(d.deployed_at, d.created_at)
                   )
                   FROM "Deployment" d
                   LEFT JOIN "DeploymentSnapshot" snap ON snap.deployment_id = d.id
                   WHERE d.project_id = p.id
                   ORDER BY d.created_at DESC LIMIT 1) AS latest_deployment
           FROM "Project" p
           JOIN "Organization" o ON o.id = p.organization_id
           LEFT JOIN LATERAL (
               SELECT u.email, u.suspended_at
               FROM "Member" m JOIN "User" u ON u.id = m.user_id
               WHERE m.organization_id = p.organization_id AND m.role = 'OWNER'
               ORDER BY (u.suspended_at IS NOT NULL), m.created_at LIMIT 1
           ) owner_member ON TRUE"#,
    );
    if let Some(project_id) = project_id {
        query.push(" WHERE p.id = ").push_bind(project_id);
    }
    query.push(" ORDER BY p.created_at DESC LIMIT 500");
    Ok(query
        .build_query_as::<AdminSiteRow>()
        .fetch_all(&state.biz_context.pool)
        .await?)
}

fn admin_site_summary(site: &AdminSiteRow) -> Value {
    let owner_status = if site.owner_email.is_some() && site.owner_suspended_at.is_some() {
        "suspended"
    } else if site.owner_email.is_some() {
        "active"
    } else if site.owner_invitation_id.is_some() {
        "invited"
    } else {
        "missing"
    };
    let latest = site.latest_deployment.clone().map(|mut latest| {
        if let Some(status) = latest.get("status").and_then(Value::as_str) {
            latest["status"] = json!(match status {
                "ACTIVE" => "READY",
                "PENDING" => "PENDING",
                "BUILDING" | "DEPLOYING" => "BUILDING",
                "FAILED" | "DELETED" => "FAILED",
                _ => status,
            });
        }
        latest
    });
    json!({
        "id": site.id,
        "name": site.name,
        "slug": site.slug,
        "org": site.organization_name,
        "organizationId": site.organization_id,
        "plan": site.plan,
        "owner": site.owner_email.as_deref().or(site.owner_invitation_email.as_deref()),
        "ownerStatus": owner_status,
        "ownerInvitationId": site.owner_invitation_id,
        "pages": site.pages,
        "deployments": site.deployments,
        "languages": site.languages,
        "members": site.members,
        "domains": site.domains,
        "domainIssues": site.domain_issues,
        "publicVisibility": site.is_public,
        "takedownAt": site.takedown_at,
        "takedownReason": site.takedown_reason,
        "createdAt": site.created_at,
        "updatedAt": site.updated_at,
        "latestDeployment": latest,
    })
}

pub async fn sites_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<Value>, AppError> {
    require_admin(&state, &auth.user.id).await?;
    let sites = load_admin_sites(&state, None).await?;
    Ok(Json(
        json!({ "data": sites.iter().map(admin_site_summary).collect::<Vec<_>>() }),
    ))
}

pub async fn site_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&state, &auth.user.id).await?;
    let site = load_admin_sites(&state, Some(&project_id))
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::NotFound("Site not found".to_string()))?;
    let languages: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(jsonb_build_object(
               'code', code, 'label', name, 'direction', CASE WHEN is_rtl THEN 'RTL' ELSE 'LTR' END,
               'enabled', enabled, 'isDefault', is_default
           ) ORDER BY position), '[]'::JSONB)
           FROM "Language" WHERE project_id = $1"#,
    )
    .bind(&project_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    let members: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(jsonb_build_object(
               'id', m.id, 'role', m.role::TEXT, 'joinedAt', m.created_at,
               'user', jsonb_build_object('id', u.id, 'name', u.name, 'email', u.email,
                   'role', u.role, 'emailVerified', u.email_verified, 'suspendedAt', u.suspended_at)
           ) ORDER BY m.created_at), '[]'::JSONB)
           FROM "Member" m JOIN "User" u ON u.id = m.user_id
           WHERE m.organization_id = $1"#,
    )
    .bind(&site.organization_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    let invitations: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(jsonb_build_object(
               'id', id, 'email', email, 'role', role::TEXT, 'expiresAt', expires_at,
               'expired', expires_at < NOW()
           ) ORDER BY expires_at), '[]'::JSONB)
           FROM "Invitation" WHERE organization_id = $1"#,
    )
    .bind(&site.organization_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    let deployments = deployment_rows(&state, Some(&project_id), 30).await?;
    let domains = domain_rows(&state, Some(&project_id), 100).await?;
    let exports = export_rows(&state, Some(&project_id), 50).await?;
    let git = git_rows(&state, Some(&project_id), 10).await?;
    let (readers, audiences, jwt_provider) = tokio::try_join!(
        cms_db::reader_access::ReaderQueries::get_by_project(&state.biz_context.pool, &project_id),
        cms_db::reader_access::AudienceQueries::get_by_project(
            &state.biz_context.pool,
            &project_id
        ),
        cms_db::reader_access::JwtAccessProviderQueries::get_by_issuer_and_audience(
            &state.biz_context.pool,
            &project_id,
            &project_id,
        ),
    )?;
    let access_mode = if jwt_provider.is_some() || !readers.is_empty() || !audiences.is_empty() {
        "READERS"
    } else if site.is_public {
        "PUBLIC"
    } else {
        "PRIVATE"
    };
    let activity: Value = sqlx::query_scalar(
        r#"SELECT COALESCE(jsonb_agg(jsonb_build_object(
               'id', e.id, 'type', e.event_type, 'actorUserId', e.user_id,
               'actorName', u.name, 'createdAt', e.created_at
           ) ORDER BY e.created_at DESC), '[]'::JSONB)
           FROM (
               SELECT * FROM "PlatformEvent" WHERE organization_id = $1
               ORDER BY created_at DESC LIMIT 30
           ) e LEFT JOIN "User" u ON u.id = e.user_id"#,
    )
    .bind(&site.organization_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    let mut summary = admin_site_summary(&site);
    summary["description"] = json!(site.description);
    summary["workspace"] = json!({
        "id": site.organization_id,
        "name": site.organization_name,
        "plan": site.plan,
    });
    summary["usage"] = json!({
        "pages": site.pages,
        "deployments": site.deployments,
        "languages": site.languages,
        "members": site.members,
        "domains": site.domains,
    });
    summary["access"] = json!({
        "mode": access_mode,
        "readers": readers.len(),
        "audiences": audiences.len(),
        "jwtEnabled": jwt_provider.is_some(),
    });
    summary["languages"] = languages;
    summary["members"] = members;
    summary["invitations"] = invitations;
    summary["deployments"] = json!(deployments);
    summary["domains"] = json!(domains);
    summary["exports"] = json!({ "items": exports });
    summary["git"] = json!(git.first());
    summary["activity"] = activity;
    Ok(Json(json!({ "data": summary })))
}

#[derive(Debug, FromRow)]
struct DeploymentOpRow {
    id: String,
    project_id: String,
    project_name: String,
    version: Option<i64>,
    status: String,
    pages: i64,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

async fn deployment_rows(
    state: &AppState,
    project_id: Option<&str>,
    limit: i64,
) -> Result<Vec<Value>, AppError> {
    let mut query: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"SELECT d.id, d.project_id, p.name AS project_name, snap.version,
                  d.status::TEXT AS status,
                  (SELECT COUNT(*) FROM "DeploymentSnapshotPageIndex" idx
                   WHERE idx.deployment_id = d.id AND UPPER(idx.kind) = 'PAGE') AS pages,
                  d.error_message, d.created_at, d.deployed_at AS completed_at
           FROM "Deployment" d JOIN "Project" p ON p.id = d.project_id
           LEFT JOIN "DeploymentSnapshot" snap ON snap.deployment_id = d.id"#,
    );
    if let Some(project_id) = project_id {
        query.push(" WHERE d.project_id = ").push_bind(project_id);
    }
    query
        .push(" ORDER BY d.created_at DESC LIMIT ")
        .push_bind(limit.clamp(1, 100));
    Ok(query
        .build_query_as::<DeploymentOpRow>()
        .fetch_all(&state.biz_context.pool)
        .await?
        .into_iter()
        .map(|row| {
            json!({
                "id": row.id,
                "projectId": row.project_id,
                "projectName": row.project_name,
                "version": row.version,
                "status": match row.status.as_str() {
                    "ACTIVE" => "READY",
                    "PENDING" => "PENDING",
                    "BUILDING" | "DEPLOYING" => "BUILDING",
                    "FAILED" | "DELETED" => "FAILED",
                    other => other,
                },
                "pages": row.pages,
                "hasError": row.error_message.is_some(),
                "createdAt": row.created_at,
                "completedAt": row.completed_at,
            })
        })
        .collect())
}

#[derive(Debug, FromRow)]
struct DomainOpRow {
    id: String,
    project_id: String,
    project_name: String,
    hostname: String,
    verified_at: Option<DateTime<Utc>>,
    ssl_status: String,
    ssl_last_error: Option<String>,
    is_primary: bool,
    ssl_checked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

async fn domain_rows(
    state: &AppState,
    project_id: Option<&str>,
    limit: i64,
) -> Result<Vec<Value>, AppError> {
    let mut query: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"SELECT dm.id, p.id AS project_id, p.name AS project_name, dm.hostname,
                  dm.verified_at, dm.ssl_status, dm.ssl_last_error, dm.is_primary,
                  dm.ssl_checked_at, dm.created_at
           FROM "Domain" dm
           JOIN "Deployment" d ON d.id = dm.deployment_id
           JOIN "Project" p ON p.id = d.project_id"#,
    );
    if let Some(project_id) = project_id {
        query.push(" WHERE p.id = ").push_bind(project_id);
    }
    query
        .push(" ORDER BY dm.created_at DESC LIMIT ")
        .push_bind(limit.clamp(1, 100));
    Ok(query
        .build_query_as::<DomainOpRow>()
        .fetch_all(&state.biz_context.pool)
        .await?
        .into_iter()
        .map(|row| {
            json!({
                "id": row.id,
                "projectId": row.project_id,
                "projectName": row.project_name,
                "domain": row.hostname,
                "verified": row.verified_at.is_some(),
                "dnsStatus": if row.verified_at.is_some() { "VERIFIED" } else { "PENDING" },
                "sslStatus": row.ssl_status,
                "isPrimary": row.is_primary,
                "hasError": row.ssl_last_error.is_some(),
                "lastCheckedAt": row.ssl_checked_at,
                "createdAt": row.created_at,
            })
        })
        .collect())
}

#[derive(Debug, FromRow)]
struct ExportOpRow {
    id: String,
    project_id: String,
    project_name: String,
    status: String,
    format: String,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

async fn export_rows(
    state: &AppState,
    project_id: Option<&str>,
    limit: i64,
) -> Result<Vec<Value>, AppError> {
    let mut query: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"SELECT job.id, project.id AS project_id, project.name AS project_name,
                  job.status::TEXT AS status, job.format::TEXT AS format,
                  job.error_message, job.created_at, job.completed_at
           FROM "ExportJob" job
           JOIN "ExportSnapshot" snapshot ON snapshot.id = job.snapshot_id
           JOIN "Project" project ON project.id = snapshot.project_id"#,
    );
    if let Some(project_id) = project_id {
        query.push(" WHERE project.id = ").push_bind(project_id);
    }
    query
        .push(" ORDER BY job.created_at DESC LIMIT ")
        .push_bind(limit.clamp(1, 100));
    Ok(query
        .build_query_as::<ExportOpRow>()
        .fetch_all(&state.biz_context.pool)
        .await?
        .into_iter()
        .map(|row| {
            json!({
                "id": row.id,
                "projectId": row.project_id,
                "projectName": row.project_name,
                "status": row.status,
                "format": row.format,
                "hasError": row.error_message.is_some(),
                "createdAt": row.created_at,
                "completedAt": row.completed_at,
            })
        })
        .collect())
}

#[derive(Debug, FromRow)]
struct GitOpRow {
    id: String,
    project_id: String,
    project_name: String,
    kind: String,
    status: String,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

async fn git_rows(
    state: &AppState,
    project_id: Option<&str>,
    limit: i64,
) -> Result<Vec<Value>, AppError> {
    let mut query: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"SELECT operation.id, project.id AS project_id, project.name AS project_name,
                  operation.operation_type::TEXT AS kind, operation.status::TEXT AS status,
                  operation.error_message, operation.created_at, operation.completed_at
           FROM "GitSyncOperation" operation
           JOIN "GitConnection" connection ON connection.id = operation.connection_id
           JOIN "Project" project ON project.id = connection.project_id"#,
    );
    if let Some(project_id) = project_id {
        query.push(" WHERE project.id = ").push_bind(project_id);
    }
    query
        .push(" ORDER BY operation.created_at DESC LIMIT ")
        .push_bind(limit.clamp(1, 100));
    Ok(query
        .build_query_as::<GitOpRow>()
        .fetch_all(&state.biz_context.pool)
        .await?
        .into_iter()
        .map(|row| {
            json!({
                "id": row.id,
                "projectId": row.project_id,
                "projectName": row.project_name,
                "kind": row.kind,
                "status": row.status,
                "hasError": row.error_message.is_some(),
                "createdAt": row.created_at,
                "completedAt": row.completed_at,
            })
        })
        .collect())
}

pub async fn operations_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
) -> Result<Json<Value>, AppError> {
    require_admin(&state, &auth.user.id).await?;
    let (deployments, domains, exports, git) = tokio::try_join!(
        deployment_rows(&state, None, 100),
        domain_rows(&state, None, 100),
        export_rows(&state, None, 50),
        git_rows(&state, None, 50),
    )?;
    Ok(Json(json!({ "data": {
        "deployments": deployments,
        "domains": domains,
        "exports": exports,
        "git": git,
    } })))
}

#[derive(Deserialize)]
pub struct InviteOrganizationBody {
    #[serde(rename = "organizationName")]
    organization_name: String,
    #[serde(rename = "siteName")]
    site_name: String,
    #[serde(rename = "ownerEmail")]
    owner_email: String,
    #[serde(rename = "siteSlug")]
    site_slug: Option<String>,
    description: Option<String>,
    #[serde(default = "default_delivery")]
    delivery: String,
}

fn default_delivery() -> String {
    "email".to_string()
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut dash = false;
    for character in value.trim().chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            dash = false;
        } else if !slug.is_empty() && !dash {
            slug.push('-');
            dash = true;
        }
        if slug.len() >= 63 {
            break;
        }
    }
    slug.trim_matches('-').to_string()
}

fn valid_owner_email(email: &str) -> bool {
    if email.len() > 254 || email.chars().any(char::is_whitespace) {
        return false;
    }
    let mut parts = email.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    !local.is_empty()
        && local.len() <= 64
        && domain.split('.').count() >= 2
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        })
}

pub async fn invite_organization_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(body): Json<InviteOrganizationBody>,
) -> Result<Json<Value>, AppError> {
    require_admin(&state, &auth.user.id).await?;
    let organization_name = body.organization_name.trim();
    let site_name = body.site_name.trim();
    let owner_email = body.owner_email.trim().to_ascii_lowercase();
    if organization_name.is_empty()
        || organization_name.len() > 100
        || site_name.is_empty()
        || site_name.len() > 100
        || !valid_owner_email(&owner_email)
        || body
            .description
            .as_deref()
            .is_some_and(|value| value.len() > 500)
        || !matches!(body.delivery.as_str(), "email" | "link")
    {
        return Err(AppError::InvalidInput(
            "Invalid organization invitation details".to_string(),
        ));
    }
    if body.delivery == "email"
        && state
            .config
            .mailer
            .as_ref()
            .and_then(|mailer| mailer.smtp_host.as_ref())
            .is_none()
    {
        return Err(AppError::ProviderError(
            "Email delivery is not configured".to_string(),
        ));
    }

    let org_slug_base = {
        let slug = slugify(organization_name);
        if slug.is_empty() {
            "workspace".to_string()
        } else {
            slug
        }
    };
    // The organization is new, so it cannot already own a project with this slug.
    let project_slug = slugify(body.site_slug.as_deref().unwrap_or(site_name));
    let org_id = uuid::Uuid::new_v4().to_string();
    let project_id = uuid::Uuid::new_v4().to_string();
    let branch_id = uuid::Uuid::new_v4().to_string();
    let language_id = uuid::Uuid::new_v4().to_string();
    let invitation_id = uuid::Uuid::new_v4().to_string();
    let invitation_token = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();
    let expires_at = now + chrono::Duration::days(7);
    let project_slug = if project_slug.is_empty() {
        "docs".to_string()
    } else {
        project_slug
    };

    // The operator is intentionally not inserted as a workspace member. The
    // invited user becomes the sole initial owner after accepting the invite.
    let mut tx = state.biz_context.pool.begin().await?;
    let description = body
        .description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut organization_created = false;
    for suffix in 0..100 {
        let candidate = if suffix == 0 {
            org_slug_base.clone()
        } else {
            format!("{}-{}", org_slug_base, suffix + 1)
        };
        let inserted = sqlx::query(
            r#"INSERT INTO "Organization" (id, name, slug, description, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $5)
               ON CONFLICT (slug) DO NOTHING"#,
        )
        .bind(&org_id)
        .bind(organization_name)
        .bind(&candidate)
        .bind(description)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        if inserted.rows_affected() == 1 {
            organization_created = true;
            break;
        }
    }
    if !organization_created {
        let fallback_slug = format!("{}-{}", org_slug_base, uuid::Uuid::new_v4().simple());
        let inserted = sqlx::query(
            r#"INSERT INTO "Organization" (id, name, slug, description, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $5)
               ON CONFLICT (slug) DO NOTHING"#,
        )
        .bind(&org_id)
        .bind(organization_name)
        .bind(fallback_slug)
        .bind(description)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        if inserted.rows_affected() == 0 {
            return Err(AppError::Conflict(
                "Could not reserve a unique organization slug".to_string(),
            ));
        }
    }
    sqlx::query(
        r#"INSERT INTO "Project" (id, organization_id, name, slug, description, is_public, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, TRUE, $6, $6)"#,
    )
    .bind(&project_id).bind(&org_id).bind(site_name).bind(&project_slug)
    .bind(body.description.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(now).execute(&mut *tx).await?;
    sqlx::query(
        r#"INSERT INTO "Branch" (id, project_id, name, slug, description, is_default, is_protected, created_at, updated_at)
           VALUES ($1, $2, 'main', 'main', 'Default branch', TRUE, TRUE, $3, $3)"#,
    )
    .bind(&branch_id).bind(&project_id).bind(now).execute(&mut *tx).await?;
    sqlx::query(
        r#"INSERT INTO "Language" (id, project_id, code, name, is_default, is_rtl, enabled, position, created_at, updated_at)
           VALUES ($1, $2, 'en', 'English', TRUE, FALSE, TRUE, 0, $3, $3)"#,
    )
    .bind(&language_id).bind(&project_id).bind(now).execute(&mut *tx).await?;
    sqlx::query(
        r#"INSERT INTO "Invitation" (id, organization_id, email, role, token, expires_at, created_at, updated_at)
           VALUES ($1, $2, $3, 'OWNER', $4, $5, $6, $6)"#,
    )
    .bind(&invitation_id).bind(&org_id).bind(&owner_email).bind(&invitation_token)
    .bind(expires_at).bind(now).execute(&mut *tx).await?;

    let base_url = state
        .config
        .admin_origin
        .allowed_origins
        .first()
        .map(|origin| origin.trim_end_matches('/').to_string())
        .unwrap_or_else(|| {
            format!(
                "https://{}",
                state.config.site.self_host.as_deref().unwrap_or("cms.app")
            )
        });
    let invitation_url = format!("{base_url}/accept-invite/{invitation_token}");
    if body.delivery == "email" {
        let email_payload = json!({
            "to": owner_email,
            "subject": format!("You're invited to {}", organization_name),
            "body": format!("You have been invited as the owner of {}. Accept the invitation: {}\n\nThis invitation expires in 7 days.", organization_name, invitation_url),
        });
        if state.config.queue.backend.eq_ignore_ascii_case("postgres") {
            sqlx::query(
                r#"INSERT INTO "CmsJob" (id, job_type, payload, status, retry_count, created_at, available_at)
                   VALUES ($1, 'email', $2, 'pending', 0, $3, $3)"#,
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(email_payload)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;

    if body.delivery == "email" && !state.config.queue.backend.eq_ignore_ascii_case("postgres") {
        let job = cms_queue::JobEnvelope::new(
            cms_queue::JobType::Email,
            json!({
                "to": owner_email,
                "subject": format!("You're invited to {}", organization_name),
                "body": format!("You have been invited as the owner of {}. Accept the invitation: {}\n\nThis invitation expires in 7 days.", organization_name, invitation_url),
            }),
        );
        if let Err(error) = state.job_queue.enqueue(job).await {
            let _ =
                cms_db::org::OrganizationQueries::delete(&state.biz_context.pool, &org_id).await;
            return Err(error);
        }
    }

    Ok(Json(json!({ "data": {
        "organizationId": org_id,
        "projectId": project_id,
        "invitationId": invitation_id,
        "ownerEmail": owner_email,
        "slug": project_slug,
        "invitationUrl": invitation_url,
        "delivery": body.delivery,
    } })))
}

#[derive(Deserialize)]
pub struct TakedownBody {
    reason: String,
}

pub async fn takedown_site_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
    Json(body): Json<TakedownBody>,
) -> Result<Json<Value>, AppError> {
    require_admin(&state, &auth.user.id).await?;
    let reason = body.reason.trim();
    if reason.is_empty() || reason.len() > 500 {
        return Err(AppError::InvalidInput(
            "reason must be between 1 and 500 characters".to_string(),
        ));
    }
    cms_db::project::ProjectQueries::set_takedown(&state.biz_context.pool, &project_id, reason)
        .await?;
    Ok(Json(json!({ "data": { "ok": true } })))
}

pub async fn restore_site_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&state, &auth.user.id).await?;
    cms_db::project::ProjectQueries::restore_takedown(&state.biz_context.pool, &project_id).await?;
    Ok(Json(json!({ "data": { "ok": true } })))
}

use sqlx::Row;
