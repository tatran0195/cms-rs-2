//! PostgreSQL-backed product flow covering project creation, language-scoped
//! document creation, public resolution, and a real deployment artifact.
//!
//! Run against a disposable database with `cargo xtask e2e`.

use std::{path::PathBuf, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Extension, Router,
};
use chrono::{Duration, Timelike, Utc};
use cms_config::Config;
use cms_middleware::app_state::AppState;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

struct Seed {
    user_id: String,
    organization_id: String,
    session_token: String,
    platform_admin_id: String,
    platform_admin_session_token: String,
    storage_root: PathBuf,
}

fn session_cookie(token: &str) -> String {
    format!("better-auth.session_token={token}")
}

async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> anyhow::Result<(StatusCode, Value)> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    let request_body = match body {
        Some(value) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(serde_json::to_vec(&value)?)
        }
        None => Body::empty(),
    };
    let response = app.clone().oneshot(builder.body(request_body)?).await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await?;
    let payload = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)?
    };
    Ok((status, payload))
}

async fn site_request(app: &Router, host: &str, uri: &str) -> anyhow::Result<(StatusCode, String)> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .header(header::HOST, host)
                .body(Body::empty())?,
        )
        .await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await?;
    Ok((status, String::from_utf8(bytes.to_vec())?))
}

async fn site_request_through_tls_proxy(
    app: &Router,
    host: &str,
    uri: &str,
    proxy_secret: Option<&str>,
) -> anyhow::Result<(StatusCode, String)> {
    let mut request = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::HOST, host)
        .header("x-forwarded-proto", "https");
    if let Some(secret) = proxy_secret {
        request = request.header("x-cms-proxy-token", secret);
    }
    let response = app.clone().oneshot(request.body(Body::empty())?).await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await?;
    Ok((status, String::from_utf8(bytes.to_vec())?))
}

async fn wait_for_deployment_ready(
    state: &AppState,
    deployment_id: &str,
) -> anyhow::Result<(i64, i64)> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        let deployment = cms_db::deployment::DeploymentQueries::get_by_id(
            &state.biz_context.pool,
            deployment_id,
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("deployment {deployment_id} disappeared"))?;
        match &deployment.status {
            cms_entity::deployment::DeploymentStatus::Active => {
                let snapshot = cms_db::deployment::DeploymentQueries::get_snapshot_by_deployment(
                    &state.biz_context.pool,
                    deployment_id,
                )
                .await?
                .ok_or_else(|| anyhow::anyhow!("ready deployment has no immutable snapshot"))?;
                let page_count = cms_db::deployment::DeploymentQueries::get_snapshot_page_count(
                    &state.biz_context.pool,
                    deployment_id,
                )
                .await?;
                return Ok((snapshot.version, page_count));
            }
            cms_entity::deployment::DeploymentStatus::Failed => {
                anyhow::bail!(
                    "deployment {deployment_id} failed: {}",
                    deployment
                        .error_message
                        .as_deref()
                        .unwrap_or("no error detail")
                );
            }
            _ if tokio::time::Instant::now() >= deadline => {
                anyhow::bail!("timed out waiting for deployment {deployment_id}");
            }
            _ => tokio::time::sleep(std::time::Duration::from_millis(25)).await,
        }
    }
}

fn expect_status(
    result: (StatusCode, Value),
    expected: StatusCode,
    operation: &str,
) -> anyhow::Result<Value> {
    let (status, payload) = result;
    anyhow::ensure!(
        status == expected,
        "{operation}: expected HTTP {}, got HTTP {} with body {}",
        expected,
        status,
        payload
    );
    Ok(payload)
}

fn required_string<'a>(value: &'a Value, key: &str, context: &str) -> anyhow::Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("{context}: missing string field {key:?} in {value}"))
}

async fn seed(state: &Arc<AppState>) -> anyhow::Result<Seed> {
    let registration_app = Router::new()
        .nest("/api", cms_api::create_api_router(state.clone()))
        .layer(Extension(state.clone()));
    let organization_id = Uuid::new_v4().to_string();
    let member_id = Uuid::new_v4().to_string();
    let session_id = Uuid::new_v4().to_string();
    let session_token = format!("e2e_{}", Uuid::new_v4().simple());
    let platform_admin_id = Uuid::new_v4().to_string();
    let platform_admin_session_id = Uuid::new_v4().to_string();
    let platform_admin_session_token = format!("e2e_admin_{}", Uuid::new_v4().simple());
    let email = format!("e2e-{}@example.invalid", Uuid::new_v4());
    let platform_admin_email = format!("e2e-admin-{}@example.invalid", Uuid::new_v4());
    let now = Utc::now();

    let registered_user = expect_status(
        request(
            &registration_app,
            Method::POST,
            "/api/auth/register",
            None,
            Some(json!({
                "email": email,
                "password": "E2E-Password-2026",
                "name": "Product E2E",
            })),
        )
        .await?,
        StatusCode::OK,
        "register the E2E user through the product auth route",
    )?;
    let user_id = required_string(&registered_user, "id", "registered E2E user")?.to_string();
    let signup_events: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM "PlatformEvent"
           WHERE user_id = $1 AND event_type = 'signup_completed'"#,
    )
    .bind(&user_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    anyhow::ensure!(
        signup_events == 1,
        "registration should emit one trusted signup event"
    );

    sqlx::query(
        r#"INSERT INTO "User" (id, email, name, email_verified, role, created_at, updated_at)
           VALUES ($1, $2, 'Platform Admin E2E', TRUE, 'admin', $3, $3)"#,
    )
    .bind(&platform_admin_id)
    .bind(platform_admin_email)
    .bind(now)
    .execute(&state.biz_context.pool)
    .await?;

    sqlx::query(
        r#"INSERT INTO "Organization" (id, name, slug, created_at, updated_at)
           VALUES ($1, 'Product E2E', $2, $3, $3)"#,
    )
    .bind(&organization_id)
    .bind(format!("e2e-{}", Uuid::new_v4().simple()))
    .bind(now)
    .execute(&state.biz_context.pool)
    .await?;

    sqlx::query(
        r#"INSERT INTO "Member" (id, user_id, organization_id, role, created_at, updated_at)
           VALUES ($1, $2, $3, 'OWNER', $4, $4)"#,
    )
    .bind(member_id)
    .bind(&user_id)
    .bind(&organization_id)
    .bind(now)
    .execute(&state.biz_context.pool)
    .await?;

    sqlx::query(
        r#"INSERT INTO "Session" (id, user_id, session_token, expires_at, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $5)"#,
    )
    .bind(session_id)
    .bind(&user_id)
    .bind(&session_token)
    .bind(now + Duration::hours(2))
    .bind(now)
    .execute(&state.biz_context.pool)
    .await?;

    sqlx::query(
        r#"INSERT INTO "Session" (id, user_id, session_token, expires_at, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $5)"#,
    )
    .bind(platform_admin_session_id)
    .bind(&platform_admin_id)
    .bind(&platform_admin_session_token)
    .bind(now + Duration::hours(2))
    .bind(now)
    .execute(&state.biz_context.pool)
    .await?;

    Ok(Seed {
        user_id,
        organization_id,
        session_token,
        platform_admin_id,
        platform_admin_session_token,
        storage_root: state
            .config
            .storage
            .local_root
            .as_deref()
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("E2E storage root was not configured"))?,
    })
}

async fn cleanup(state: &AppState, seed: &Seed) -> anyhow::Result<()> {
    // Organization/project cascades remove all seeded product rows. User/session
    // cleanup is separate because organizations are not owned by the user FK.
    sqlx::query(r#"DELETE FROM "Organization" WHERE id = $1"#)
        .bind(&seed.organization_id)
        .execute(&state.biz_context.pool)
        .await?;
    sqlx::query(r#"DELETE FROM "User" WHERE id = $1"#)
        .bind(&seed.user_id)
        .execute(&state.biz_context.pool)
        .await?;
    sqlx::query(r#"DELETE FROM "User" WHERE id = $1"#)
        .bind(&seed.platform_admin_id)
        .execute(&state.biz_context.pool)
        .await?;
    if seed.storage_root.exists() {
        tokio::fs::remove_dir_all(&seed.storage_root).await?;
    }
    Ok(())
}

async fn create_project(
    app: &Router,
    cookie: &str,
    name: String,
    organization_id: &str,
    is_public: bool,
) -> anyhow::Result<Value> {
    let value = expect_status(
        request(
            app,
            Method::POST,
            "/api/app/projects",
            Some(cookie),
            Some(json!({
                "name": name,
                "organizationId": organization_id,
                "isPublic": is_public,
            })),
        )
        .await?,
        StatusCode::OK,
        "create project",
    )?;
    Ok(value["data"].clone())
}

async fn create_page(
    app: &Router,
    cookie: &str,
    project_id: &str,
    body: Value,
) -> anyhow::Result<Value> {
    let value = expect_status(
        request(
            app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/pages"),
            Some(cookie),
            Some(body),
        )
        .await?,
        StatusCode::OK,
        "create page",
    )?;
    Ok(value["data"].clone())
}

async fn run_flow(state: Arc<AppState>, seed: &Seed) -> anyhow::Result<()> {
    let app = Router::new()
        .nest("/api", cms_api::create_api_router(state.clone()))
        .merge(cms_sites::sites_router(state.clone()))
        .layer(Extension(state.clone()));
    let cookie = session_cookie(&seed.session_token);
    let admin_cookie = session_cookie(&seed.platform_admin_session_token);
    expect_status(
        request(&app, Method::GET, "/api/admin/sites", Some(&cookie), None).await?,
        StatusCode::FORBIDDEN,
        "organization owners are not platform administrators",
    )?;

    // The real app route creates a project and atomically seeds its default
    // branch and English language.
    let project = create_project(
        &app,
        &cookie,
        format!("Product E2E {}", Uuid::new_v4().simple()),
        &seed.organization_id,
        true,
    )
    .await?;
    let project_id = required_string(&project, "id", "created project")?.to_string();
    let project_slug = required_string(&project, "slug", "created project")?.to_string();
    let organization_slug: String =
        sqlx::query_scalar(r#"SELECT slug FROM "Organization" WHERE id = $1"#)
            .bind(&seed.organization_id)
            .fetch_one(&state.biz_context.pool)
            .await?;
    let private_project = create_project(
        &app,
        &cookie,
        format!("Private E2E {}", Uuid::new_v4().simple()),
        &seed.organization_id,
        false,
    )
    .await?;
    let private_project_id = required_string(&private_project, "id", "private project")?;
    let private_project_slug = required_string(&private_project, "slug", "private project")?;

    let signup_at = Utc::now() - Duration::hours(2);
    for (event_type, created_at, metadata) in [
        ("signup_completed", signup_at, json!({})),
        ("page_edited", signup_at + Duration::minutes(20), json!({})),
        (
            "publish_clicked",
            signup_at + Duration::minutes(30),
            json!({ "auto": false }),
        ),
        (
            "publish_ready",
            signup_at - Duration::minutes(1),
            json!({ "auto": false }),
        ),
    ] {
        sqlx::query(
            r#"INSERT INTO "PlatformEvent" (id, organization_id, user_id, event_type, metadata, created_at)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&seed.organization_id)
        .bind(&seed.platform_admin_id)
        .bind(event_type)
        .bind(metadata)
        .bind(created_at)
        .execute(&state.biz_context.pool)
        .await?;
    }

    let overview = expect_status(
        request(
            &app,
            Method::GET,
            "/api/admin/overview",
            Some(&admin_cookie),
            None,
        )
        .await?,
        StatusCode::OK,
        "load platform-admin overview from persisted records",
    )?;
    anyhow::ensure!(overview["data"]["admins"]
        .as_i64()
        .is_some_and(|count| count >= 1));
    anyhow::ensure!(overview["data"]["sites"]
        .as_i64()
        .is_some_and(|count| count >= 2));
    let admin_user = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/admin/users/{}", seed.user_id),
            Some(&admin_cookie),
            None,
        )
        .await?,
        StatusCode::OK,
        "load user detail with actual memberships",
    )?;
    let workspaces = admin_user["data"]["workspaces"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("admin user workspaces is not an array"))?;
    let workspace = workspaces
        .iter()
        .find(|workspace| workspace["organizationId"] == seed.organization_id)
        .ok_or_else(|| anyhow::anyhow!("admin user detail omitted seeded organization"))?;
    anyhow::ensure!(workspace["projectCount"] == 2);
    let projects = workspace["projects"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("admin membership projects is not an array"))?;
    anyhow::ensure!(projects.iter().any(|project| project["id"] == project_id));
    anyhow::ensure!(projects
        .iter()
        .any(|project| project["id"] == private_project_id));
    expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/admin/users/{}/suspend", seed.user_id),
            Some(&cookie),
            None,
        )
        .await?,
        StatusCode::FORBIDDEN,
        "organization owner cannot invoke platform user operations",
    )?;
    let invite = expect_status(
        request(
            &app,
            Method::POST,
            "/api/admin/organizations/invite",
            Some(&admin_cookie),
            Some(json!({
                "organizationName": format!("Invited E2E {}", Uuid::new_v4().simple()),
                "siteName": "Invited docs",
                "siteSlug": format!("invited-{}", Uuid::new_v4().simple()),
                "ownerEmail": format!("owner-{}@example.invalid", Uuid::new_v4().simple()),
                "delivery": "link",
            })),
        )
        .await?,
        StatusCode::OK,
        "platform admin creates a real owner invitation and starter site",
    )?;
    let invited_org_id = required_string(&invite["data"], "organizationId", "admin invite")?;
    let invited_project_id = required_string(&invite["data"], "projectId", "admin invite")?;
    let invitation_id = required_string(&invite["data"], "invitationId", "admin invite")?;
    let public_invitation = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/invitations/{invitation_id}"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "resolve an invitation by its opaque invitation id",
    )?;
    anyhow::ensure!(public_invitation["data"]["id"] == invitation_id);
    anyhow::ensure!(public_invitation["data"]["email"] == invite["data"]["ownerEmail"]);
    let starter_records: (i64, i64, i64) = sqlx::query_as(
        r#"SELECT
             (SELECT COUNT(*) FROM "Project" WHERE id = $1 AND organization_id = $2),
             (SELECT COUNT(*) FROM "Branch" WHERE project_id = $1 AND is_default),
             (SELECT COUNT(*) FROM "Language" WHERE project_id = $1 AND is_default AND code = 'en')"#,
    )
    .bind(invited_project_id)
    .bind(invited_org_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    anyhow::ensure!(starter_records == (1, 1, 1));
    sqlx::query(r#"DELETE FROM "Organization" WHERE id = $1"#)
        .bind(invited_org_id)
        .execute(&state.biz_context.pool)
        .await?;
    expect_status(
        request(
            &app,
            Method::POST,
            "/api/platform-events",
            None,
            Some(json!({ "event_type": "custom_test" })),
        )
        .await?,
        StatusCode::UNAUTHORIZED,
        "reject unauthenticated platform-event injection",
    )?;
    expect_status(
        request(
            &app,
            Method::POST,
            "/api/platform-events",
            Some(&cookie),
            Some(json!({
                "event_type": "custom_test",
                "organization_id": seed.organization_id,
                "user_id": seed.platform_admin_id,
                "metadata": { "source": "e2e" },
            })),
        )
        .await?,
        StatusCode::OK,
        "record an event using authenticated identity instead of body user ID",
    )?;
    let recorded_event_user: String = sqlx::query_scalar(
        r#"SELECT user_id FROM "PlatformEvent" WHERE event_type = 'custom_test' ORDER BY created_at DESC LIMIT 1"#,
    )
    .fetch_one(&state.biz_context.pool)
    .await?;
    anyhow::ensure!(recorded_event_user == seed.user_id);
    expect_status(
        request(
            &app,
            Method::POST,
            "/api/platform-events",
            Some(&cookie),
            Some(json!({
                "event_type": "publish_ready",
                "organization_id": seed.organization_id,
                "metadata": { "auto": false },
            })),
        )
        .await?,
        StatusCode::FORBIDDEN,
        "prevent clients from spoofing trusted funnel milestones",
    )?;

    let public_host = format!("{project_slug}.cms.app");
    let (unreleased_status, _) = site_request(&app, &public_host, "/").await?;
    anyhow::ensure!(
        unreleased_status == StatusCode::NOT_FOUND,
        "hosted site should stay hidden before its first release, got {unreleased_status}"
    );
    let private_host = format!("{private_project_slug}.cms.app");
    let (private_status, _) = site_request(&app, &private_host, "/").await?;
    anyhow::ensure!(
        private_status == StatusCode::NOT_FOUND,
        "private project must not be rendered by the hosted SSR router, got {private_status}"
    );

    // Public sites expose only committed releases, never unsnapshotted editor
    // state. The first publish below makes the site public.
    expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/sites/{project_id}"),
            None,
            None,
        )
        .await?,
        StatusCode::NOT_FOUND,
        "hide project before its first published release",
    )?;
    expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/sites/{private_project_id}"),
            None,
            None,
        )
        .await?,
        StatusCode::NOT_FOUND,
        "hide private project from public API",
    )?;
    expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/projects/{organization_slug}/{private_project_slug}"),
            None,
            None,
        )
        .await?,
        StatusCode::NOT_FOUND,
        "hide private project from legacy public route",
    )?;

    // The public shell intentionally emits only language codes; get the actual
    // default-language ID from the authenticated project language list.
    let language_list = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/app/projects/{project_id}/languages"),
            Some(&cookie),
            None,
        )
        .await?,
        StatusCode::OK,
        "list project languages",
    )?;
    let languages = language_list["data"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("language list did not return an array: {language_list}"))?;
    let english = languages
        .iter()
        .find(|language| language["isDefault"] == true)
        .ok_or_else(|| anyhow::anyhow!("project create did not seed a default language"))?;
    let english_language_id =
        required_string(english, "id", "default English language")?.to_string();
    anyhow::ensure!(english["code"] == "en");

    let arabic_language_response = expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/languages"),
            Some(&cookie),
            Some(json!({
                "code": "ar-eg",
                "name": "Arabic (Egypt)",
                "direction": "RTL",
                "config": { "reader": { "greeting": "مرحبا" } },
            })),
        )
        .await?,
        StatusCode::OK,
        "create Arabic language",
    )?;
    let arabic_language = arabic_language_response["data"].clone();
    let arabic_language_id =
        required_string(&arabic_language, "id", "Arabic language")?.to_string();
    anyhow::ensure!(arabic_language["code"] == "ar-EG");
    anyhow::ensure!(arabic_language["direction"] == "RTL");

    // BCP-47 canonicalization and duplicate codes are observable API failures.
    expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/languages"),
            Some(&cookie),
            Some(json!({ "code": "bad..tag", "name": "Broken" })),
        )
        .await?,
        StatusCode::BAD_REQUEST,
        "reject malformed language code",
    )?;
    expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/languages"),
            Some(&cookie),
            Some(json!({ "code": "AR-eg", "name": "Duplicate" })),
        )
        .await?,
        StatusCode::CONFLICT,
        "reject duplicate language code",
    )?;

    let english_group = create_page(
        &app,
        &cookie,
        &project_id,
        json!({
            "title": "Guide",
            "slug": "guide",
            "kind": "GROUP",
            "languageId": english_language_id,
            "isPublished": true,
        }),
    )
    .await?;
    let english_group_id =
        required_string(&english_group, "id", "English guide group")?.to_string();

    let arabic_group = create_page(
        &app,
        &cookie,
        &project_id,
        json!({
            "title": "دليل",
            "slug": "guide",
            "kind": "GROUP",
            "languageId": arabic_language_id,
            "isPublished": true,
        }),
    )
    .await?;
    let arabic_group_id = required_string(&arabic_group, "id", "Arabic guide group")?.to_string();
    anyhow::ensure!(arabic_group["languageId"] == arabic_language_id);

    let english_page = create_page(
        &app,
        &cookie,
        &project_id,
        json!({
            "title": "Getting started",
            "slug": "start",
            "kind": "PAGE",
            "parentId": english_group_id,
            "languageId": english_language_id,
            "translationKey": "getting-started",
            "isPublished": true,
            "content": "# Getting started\n\nEnglish release content.",
        }),
    )
    .await?;
    let english_page_id = required_string(&english_page, "id", "English page")?.to_string();
    anyhow::ensure!(english_page["path"] == "/guide/start");
    let page_created_events: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM "PlatformEvent"
           WHERE user_id = $1 AND event_type = 'page_edited'
             AND metadata->>'page_id' = $2 AND metadata->>'operation' = 'create'"#,
    )
    .bind(&seed.user_id)
    .bind(&english_page_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    anyhow::ensure!(
        page_created_events == 1,
        "creating a document should emit one trusted edit event"
    );

    let arabic_page = create_page(
        &app,
        &cookie,
        &project_id,
        json!({
            "title": "البدء",
            "slug": "start",
            "kind": "PAGE",
            "parentId": arabic_group_id,
            "languageId": arabic_language_id,
            "translationKey": "getting-started",
            "isPublished": true,
            "content": "# البدء\n\nمحتوى الإصدار العربي.",
        }),
    )
    .await?;
    let arabic_page_id = required_string(&arabic_page, "id", "Arabic page")?.to_string();
    anyhow::ensure!(arabic_page["languageId"] == arabic_language_id);
    anyhow::ensure!(arabic_page["path"] == "/guide/start");

    // A parent from a different language and a language from a different
    // project must not leak records across the scope boundary.
    expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/pages"),
            Some(&cookie),
            Some(json!({
                "title": "Wrong scope",
                "slug": "wrong-scope",
                "parentId": english_group_id,
                "languageId": arabic_language_id,
            })),
        )
        .await?,
        StatusCode::CONFLICT,
        "reject parent in another language",
    )?;
    let private_default_language =
        cms_db::language::LanguageQueries::get_default(&state.biz_context.pool, private_project_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("private project has no default language"))?;
    expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/pages"),
            Some(&cookie),
            Some(json!({
                "title": "Wrong project language",
                "slug": "wrong-project-language",
                "languageId": private_default_language.id,
            })),
        )
        .await?,
        StatusCode::CONFLICT,
        "reject language from another project",
    )?;

    let draft_page = create_page(
        &app,
        &cookie,
        &project_id,
        json!({
            "title": "Draft",
            "slug": "draft",
            "parentId": arabic_group_id,
            "languageId": arabic_language_id,
            "isPublished": false,
            "content": "DO NOT SHIP",
        }),
    )
    .await?;
    let _draft_page_id = required_string(&draft_page, "id", "draft page")?;

    let reorder = expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/pages/reorder"),
            Some(&cookie),
            Some(json!({
                "items": [
                    { "id": arabic_group_id, "parentId": null, "position": 0 },
                    { "id": arabic_page_id, "parentId": arabic_group_id, "position": 1 },
                    { "id": _draft_page_id, "parentId": arabic_group_id, "position": 2 },
                ]
            })),
        )
        .await?,
        StatusCode::OK,
        "reorder Arabic page tree",
    )?;
    anyhow::ensure!(reorder["data"]["success"] == true);
    expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/pages/reorder"),
            Some(&cookie),
            Some(json!({
                "items": [
                    { "id": arabic_group_id, "parentId": arabic_page_id, "position": 0 },
                    { "id": arabic_page_id, "parentId": arabic_group_id, "position": 1 },
                    { "id": _draft_page_id, "parentId": arabic_group_id, "position": 2 },
                ]
            })),
        )
        .await?,
        StatusCode::CONFLICT,
        "reject page-tree cycle atomically",
    )?;

    // Real operations test for docs side-tree handling all possible cases:
    // 1. Un-nest draft page from group to root level
    let reorder_unnest = expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/pages/reorder"),
            Some(&cookie),
            Some(json!({
                "items": [
                    { "id": arabic_group_id, "parentId": null, "position": 0 },
                    { "id": arabic_page_id, "parentId": arabic_group_id, "position": 0 },
                    { "id": _draft_page_id, "parentId": null, "position": 1 },
                ]
            })),
        )
        .await?,
        StatusCode::OK,
        "un-nest draft page to root",
    )?;
    anyhow::ensure!(reorder_unnest["data"]["success"] == true);

    let draft_row: (Option<String>, String) = sqlx::query_as(
        r#"SELECT parent_id, path FROM "Page" WHERE id = $1"#,
    )
    .bind(&_draft_page_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    anyhow::ensure!(draft_row.0.is_none(), "draft page parent_id must be null after un-nesting");
    anyhow::ensure!(draft_row.1 == "/draft", "draft page path must be /draft at root");

    // 2. Re-nest draft page back under arabic group with new position
    let reorder_renest = expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/pages/reorder"),
            Some(&cookie),
            Some(json!({
                "items": [
                    { "id": arabic_group_id, "parentId": null, "position": 0 },
                    { "id": _draft_page_id, "parentId": arabic_group_id, "position": 0 },
                    { "id": arabic_page_id, "parentId": arabic_group_id, "position": 1 },
                ]
            })),
        )
        .await?,
        StatusCode::OK,
        "re-nest draft page back under arabic group",
    )?;
    anyhow::ensure!(reorder_renest["data"]["success"] == true);

    let draft_nested_row: (Option<String>, String) = sqlx::query_as(
        r#"SELECT parent_id, path FROM "Page" WHERE id = $1"#,
    )
    .bind(&_draft_page_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    anyhow::ensure!(
        draft_nested_row.0.as_deref() == Some(&arabic_group_id),
        "draft page parent_id must be arabic_group_id"
    );
    anyhow::ensure!(
        draft_nested_row.1.ends_with("/draft"),
        "draft page path must end with /draft"
    );

    // 3. Reject duplicate IDs in reorder request
    expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/pages/reorder"),
            Some(&cookie),
            Some(json!({
                "items": [
                    { "id": arabic_group_id, "parentId": null, "position": 0 },
                    { "id": arabic_group_id, "parentId": null, "position": 1 },
                ]
            })),
        )
        .await?,
        StatusCode::BAD_REQUEST,
        "reject duplicate page id in reorder request",
    )?;

    // 4. Reject self-parenting
    expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/pages/reorder"),
            Some(&cookie),
            Some(json!({
                "items": [
                    { "id": arabic_group_id, "parentId": arabic_group_id, "position": 0 },
                ]
            })),
        )
        .await?,
        StatusCode::CONFLICT,
        "reject self-parenting",
    )?;

    // 5. Reject non-existent parent
    expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/pages/reorder"),
            Some(&cookie),
            Some(json!({
                "items": [
                    { "id": arabic_page_id, "parentId": "non-existent-page-id", "position": 0 },
                ]
            })),
        )
        .await?,
        StatusCode::CONFLICT,
        "reject non-existent parent",
    )?;

    // 6. Reset tree to clean state before publishing
    let reorder_final = expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/pages/reorder"),
            Some(&cookie),
            Some(json!({
                "items": [
                    { "id": arabic_group_id, "parentId": null, "position": 0 },
                    { "id": arabic_page_id, "parentId": arabic_group_id, "position": 1 },
                    { "id": _draft_page_id, "parentId": arabic_group_id, "position": 2 },
                ]
            })),
        )
        .await?,
        StatusCode::OK,
        "final reorder Arabic page tree",
    )?;
    anyhow::ensure!(reorder_final["data"]["success"] == true);

    // Publishing runs the real renderer and writes branch/language-scoped
    // artifacts into the configured storage backend.
    let deployment = expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/deployments"),
            Some(&cookie),
            Some(json!({ "message": "E2E release" })),
        )
        .await?,
        StatusCode::OK,
        "publish project",
    )?;
    anyhow::ensure!(deployment["data"]["status"] == "PENDING");
    let deployment_id = required_string(&deployment["data"], "id", "deployment")?;
    let publish_click_events: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM "PlatformEvent"
           WHERE user_id = $1 AND event_type = 'publish_clicked'
             AND metadata->>'deployment_id' = $2"#,
    )
    .bind(&seed.user_id)
    .bind(deployment_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    anyhow::ensure!(
        publish_click_events == 1,
        "publish request should emit one trusted click event"
    );
    let (first_version, first_pages) = wait_for_deployment_ready(&state, deployment_id).await?;
    let publish_ready_events: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM "PlatformEvent"
           WHERE user_id = $1 AND event_type = 'publish_ready'
             AND metadata->>'deployment_id' = $2"#,
    )
    .bind(&seed.user_id)
    .bind(deployment_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    anyhow::ensure!(
        publish_ready_events == 1,
        "successful publish should emit one trusted ready event"
    );
    let funnel_after_product_flow = expect_status(
        request(
            &app,
            Method::GET,
            "/api/admin/funnel?days=30",
            Some(&admin_cookie),
            None,
        )
        .await?,
        StatusCode::OK,
        "include server-emitted product events in the trusted funnel",
    )?;
    for stage in ["signups", "edited", "published", "ready"] {
        anyhow::ensure!(
            funnel_after_product_flow["data"][stage]
                .as_i64()
                .is_some_and(|count| count >= 2),
            "funnel stage {stage} should count both product events and aggregation fixture"
        );
    }
    anyhow::ensure!(
        funnel_after_product_flow["data"]["readyWithin24Hours"].as_i64() == Some(1),
        "a ready event that predates signup must not count toward 24-hour activation"
    );
    anyhow::ensure!(funnel_after_product_flow["data"]["medianHoursToReady"]
        .as_f64()
        .is_some());
    anyhow::ensure!(first_version == 1);
    anyhow::ensure!(first_pages == 2);
    let english_page_listing = cms_db::deployment::DeploymentQueries::get_snapshot_page_listing(
        &state.biz_context.pool,
        deployment_id,
        &english_language_id,
    )
    .await?;
    anyhow::ensure!(english_page_listing.len() == 1);
    anyhow::ensure!(english_page_listing[0].page_id == english_page_id);
    anyhow::ensure!(english_page_listing[0].path.trim_matches('/') == "guide/start");
    anyhow::ensure!(english_page_listing[0].title == "Getting started");
    let expected_admin_page_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM "Page" WHERE project_id = $1 AND UPPER(kind) = 'PAGE'"#,
    )
    .bind(&project_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    let admin_site = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/admin/sites/{project_id}"),
            Some(&admin_cookie),
            None,
        )
        .await?,
        StatusCode::OK,
        "load platform site detail with real deployment, language, and page data",
    )?;
    anyhow::ensure!(admin_site["data"]["pages"] == expected_admin_page_count);
    anyhow::ensure!(admin_site["data"]["usage"]["deployments"] == 1);
    anyhow::ensure!(admin_site["data"]["languages"]
        .as_array()
        .is_some_and(|items| items.len() == 2));
    anyhow::ensure!(admin_site["data"]["latestDeployment"]["id"] == deployment_id);
    anyhow::ensure!(admin_site["data"]["latestDeployment"]["status"] == "READY");
    anyhow::ensure!(admin_site["data"]["access"]["readers"] == 0);
    anyhow::ensure!(admin_site["data"]["access"]["audiences"] == 0);
    anyhow::ensure!(admin_site["data"]["access"]["jwtEnabled"] == false);
    let english_artifact = state
        .storage
        .get(&format!(
            "sites/{project_id}/{deployment_id}/en/guide/start.html"
        ))
        .await?;
    let arabic_artifact = state
        .storage
        .get(&format!(
            "sites/{project_id}/{deployment_id}/ar-EG/guide/start.html"
        ))
        .await?;
    let english_html = String::from_utf8(english_artifact.to_vec())?;
    let arabic_html = String::from_utf8(arabic_artifact.to_vec())?;
    anyhow::ensure!(english_html.contains("lang=\"en\""));
    anyhow::ensure!(english_html.contains("English release content"));
    anyhow::ensure!(arabic_html.contains("lang=\"ar-EG\""));
    anyhow::ensure!(arabic_html.contains("محتوى الإصدار العربي"));
    anyhow::ensure!(
        !state
            .storage
            .exists(&format!(
                "sites/{project_id}/{deployment_id}/ar-EG/quickstart/draft.html"
            ))
            .await?
    );

    let (ssr_status, ssr_html) = site_request(&app, &public_host, "/guide/start").await?;
    anyhow::ensure!(
        ssr_status == StatusCode::OK,
        "published host SSR should serve the release page, got {ssr_status}: {ssr_html}"
    );
    anyhow::ensure!(ssr_html.contains("<html lang=\"en\" dir=\"ltr\">"));
    anyhow::ensure!(ssr_html.contains("English release content"));
    anyhow::ensure!(ssr_html.contains(&format!(
        "<link rel=\"canonical\" href=\"https://{public_host}/guide/start\">"
    )));
    anyhow::ensure!(!ssr_html.contains("DO NOT SHIP"));
    let (root_listing_status, root_listing_html) = site_request(&app, &public_host, "/").await?;
    anyhow::ensure!(root_listing_status == StatusCode::OK);
    anyhow::ensure!(root_listing_html.contains("Getting started"));
    anyhow::ensure!(
        !root_listing_html.contains("English release content"),
        "root listing must render page metadata without loading Markdown bodies"
    );
    let (sitemap_status, sitemap_body) = site_request(&app, &public_host, "/sitemap.xml").await?;
    anyhow::ensure!(
        sitemap_status == StatusCode::OK,
        "published sitemap endpoint failed: {sitemap_body}"
    );
    anyhow::ensure!(sitemap_body.contains("/guide/start"));
    anyhow::ensure!(!sitemap_body.contains("/ar-EG/"));
    let (robots_status, robots_body) = site_request(&app, &public_host, "/robots.txt").await?;
    anyhow::ensure!(robots_status == StatusCode::OK);
    anyhow::ensure!(robots_body.contains("https://"));
    let (manifest_status, manifest_body) =
        site_request(&app, &public_host, "/site.webmanifest").await?;
    anyhow::ensure!(manifest_status == StatusCode::OK);
    anyhow::ensure!(serde_json::from_str::<Value>(&manifest_body).is_ok());
    let (draft_ssr_status, draft_ssr_body) =
        site_request(&app, &public_host, "/guide/draft").await?;
    anyhow::ensure!(
        draft_ssr_status == StatusCode::NOT_FOUND,
        "hosted SSR must not expose an unpublished page: {draft_ssr_body}"
    );

    // Change the editor record after v1. Both the API and the public host SSR
    // must continue rendering the immutable release until another deploy passes.
    expect_status(
        request(
            &app,
            Method::PATCH,
            &format!("/api/app/projects/{project_id}/pages/{english_page_id}"),
            Some(&cookie),
            Some(json!({
                "content": "# Getting started\n\nEditor-only change after release one.",
            })),
        )
        .await?,
        StatusCode::OK,
        "edit English page after v1 without changing published content",
    )?;
    let page_updated_events: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM "PlatformEvent"
           WHERE user_id = $1 AND event_type = 'page_edited'
             AND metadata->>'page_id' = $2 AND metadata->>'operation' = 'update'"#,
    )
    .bind(&seed.user_id)
    .bind(&english_page_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    anyhow::ensure!(
        page_updated_events == 1,
        "updating a document should emit one trusted edit event"
    );
    let (ssr_after_edit_status, ssr_after_edit_html) =
        site_request(&app, &public_host, "/guide/start").await?;
    anyhow::ensure!(
        ssr_after_edit_status == StatusCode::OK,
        "v1 should remain available after an editor edit, got {ssr_after_edit_status}"
    );
    anyhow::ensure!(ssr_after_edit_html.contains("English release content"));
    anyhow::ensure!(!ssr_after_edit_html.contains("Editor-only change after release one"));

    let page_by_id = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/pages/{arabic_page_id}"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "read an immutable release page by ID",
    )?;
    anyhow::ensure!(page_by_id["data"]["page"]["path"] == "guide/start");
    anyhow::ensure!(page_by_id["data"]["version"] == 1);

    expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/public/sites/{project_id}/events"),
            None,
            Some(json!({
                "type": "page_view",
                "userId": &seed.user_id,
                "ip": "203.0.113.99",
                "metadata": { "path": "guide/start" },
            })),
        )
        .await?,
        StatusCode::OK,
        "record a public analytics event",
    )?;
    let stored_event = sqlx::query_as::<_, (Option<String>, Option<String>)>(
        r#"SELECT user_id, ip_address FROM "AnalyticsEvent" WHERE project_id = $1 ORDER BY created_at DESC LIMIT 1"#,
    )
    .bind(&project_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    anyhow::ensure!(stored_event.0.is_none());
    anyhow::ensure!(stored_event.1.is_none());
    expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/public/sites/{private_project_id}/events"),
            None,
            Some(json!({ "type": "page_view" })),
        )
        .await?,
        StatusCode::NOT_FOUND,
        "do not accept public analytics for a private project",
    )?;
    expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/public/sites/{project_id}/events"),
            None,
            Some(json!({ "type": "bad event type!" })),
        )
        .await?,
        StatusCode::BAD_REQUEST,
        "reject malformed analytics event type",
    )?;

    let english_search = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/sites/{project_id}/search?q=English&lang=en&version=main"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "search the published English snapshot",
    )?;
    anyhow::ensure!(english_search["data"]["hits"]
        .as_array()
        .is_some_and(|hits| {
            hits.iter().any(|hit| hit["id"] == english_page_id)
                && hits.iter().all(|hit| hit["id"] != arabic_page_id)
        }));
    let arabic_search = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/sites/{project_id}/search?q=English&lang=ar-EG&version=main"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "keep public search scoped to the selected language",
    )?;
    anyhow::ensure!(arabic_search["data"]["hits"]
        .as_array()
        .is_some_and(Vec::is_empty));
    let grounded_answer = expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/public/sites/{project_id}/answer"),
            None,
            Some(json!({
                "question": "البدء",
                "lang": "ar-EG",
                "version": "main",
            })),
        )
        .await?,
        StatusCode::OK,
        "answer from the selected published language",
    )?;
    anyhow::ensure!(grounded_answer["data"]["mode"] == "extractive");
    anyhow::ensure!(grounded_answer["data"]["sources"]
        .as_array()
        .is_some_and(|sources| { sources.iter().any(|source| source["id"] == arabic_page_id) }));

    let legacy_project = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/projects/{organization_slug}/{project_slug}"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "read snapshot-backed legacy project route",
    )?;
    anyhow::ensure!(legacy_project["id"] == project_id);
    let legacy_page = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/pages/{organization_slug}/{project_slug}/guide/start"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "read snapshot-backed legacy page route",
    )?;
    anyhow::ensure!(legacy_page["id"] == english_page_id);
    let legacy_pages = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/pages/{organization_slug}/{project_slug}"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "list snapshot-backed legacy pages",
    )?;
    anyhow::ensure!(legacy_pages.as_array().is_some_and(|pages| {
        pages.iter().any(|page| page["id"] == english_page_id)
            && pages.iter().all(|page| page["id"] != arabic_page_id)
    }));
    let legacy_search = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/search/{organization_slug}/{project_slug}?q=English&lang=en"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "search from the snapshot-backed legacy route",
    )?;
    anyhow::ensure!(legacy_search
        .as_array()
        .is_some_and(|pages| { pages.iter().any(|page| page["id"] == english_page_id) }));
    let legacy_sitemap = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/sitemap/{organization_slug}/{project_slug}"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "build legacy sitemap from published snapshot",
    )?;
    anyhow::ensure!(legacy_sitemap["urls"].as_array().is_some_and(|urls| {
        urls.iter().any(|url| {
            url["loc"]
                .as_str()
                .is_some_and(|loc| loc.ends_with("/guide/start"))
        })
    }));

    let after_create_shell = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/sites/{project_id}?lang=ar-EG"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "resolve Arabic site shell",
    )?;
    anyhow::ensure!(after_create_shell["data"]["activeLanguage"] == "ar-EG");
    let arabic_nav = after_create_shell["data"]["nav"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("public nav is not an array"))?;
    anyhow::ensure!(arabic_nav.iter().any(|node| node["title"] == "دليل"));

    let arabic_public_page = expect_status(
        request(
            &app,
            Method::GET,
            &format!(
                "/api/public/sites/{project_id}/page?path=guide%2Fstart&lang=ar-EG&version=main"
            ),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "read Arabic public page",
    )?;
    anyhow::ensure!(arabic_public_page["data"]["activeLanguage"] == "ar-EG");
    anyhow::ensure!(arabic_public_page["data"]["page"]["title"] == "البدء");
    anyhow::ensure!(arabic_public_page["data"]["page"]["content"]
        .as_str()
        .unwrap()
        .contains("محتوى الإصدار العربي"));
    anyhow::ensure!(arabic_public_page["data"]["languages"]
        .as_array()
        .is_some_and(|items| {
            items
                .iter()
                .any(|item| item["code"] == "en" && item["path"] == "guide/start")
        }));

    let english_public_page = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/sites/{project_id}/page?path=guide%2Fstart&lang=en&version=main"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "read English public page",
    )?;
    anyhow::ensure!(english_public_page["data"]["page"]["title"] == "Getting started");
    anyhow::ensure!(english_public_page["data"]["page"]["content"]
        .as_str()
        .unwrap()
        .contains("English release content"));

    let fallback_shell = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/sites/{project_id}?lang=not-a-language"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "fallback unknown public language",
    )?;
    anyhow::ensure!(fallback_shell["data"]["activeLanguage"] == "en");

    // Coverage deliberately includes unpublished PAGE records, excludes GROUP
    // records, and matches same paths even though translation keys are metadata.
    let refreshed_languages = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/app/projects/{project_id}/languages"),
            Some(&cookie),
            None,
        )
        .await?,
        StatusCode::OK,
        "read page translation coverage",
    )?;
    let arabic_coverage = refreshed_languages["data"]
        .as_array()
        .and_then(|languages| {
            languages
                .iter()
                .find(|language| language["id"] == arabic_language_id)
        })
        .and_then(|language| language.get("coverage"))
        .ok_or_else(|| anyhow::anyhow!("Arabic coverage missing from {refreshed_languages}"))?;
    anyhow::ensure!(arabic_coverage["sourcePageCount"] == 1);
    anyhow::ensure!(arabic_coverage["pageCount"] == 2);
    anyhow::ensure!(arabic_coverage["matchedPages"] == 1);
    anyhow::ensure!(arabic_coverage["extraPages"] == 1);

    // Rename one language's parent and verify descendant paths recompute without
    // changing the corresponding tree in English.
    expect_status(
        request(
            &app,
            Method::PATCH,
            &format!("/api/app/projects/{project_id}/pages/{arabic_group_id}"),
            Some(&cookie),
            Some(json!({ "slug": "quickstart" })),
        )
        .await?,
        StatusCode::OK,
        "rename Arabic group",
    )?;
    let moved_arabic_page = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/app/projects/{project_id}/pages/{arabic_page_id}"),
            Some(&cookie),
            None,
        )
        .await?,
        StatusCode::OK,
        "read moved Arabic page",
    )?;
    anyhow::ensure!(moved_arabic_page["data"]["path"] == "/quickstart/start");
    let unchanged_english_page = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/app/projects/{project_id}/pages/{english_page_id}"),
            Some(&cookie),
            None,
        )
        .await?,
        StatusCode::OK,
        "read unchanged English page",
    )?;
    anyhow::ensure!(unchanged_english_page["data"]["path"] == "/guide/start");
    let (unreleased_translation_path_status, unreleased_translation_path_body) =
        site_request(&app, &public_host, "/quickstart/start").await?;
    anyhow::ensure!(
        unreleased_translation_path_status == StatusCode::NOT_FOUND,
        "SSR must not resolve a translated editor path before deploy: {unreleased_translation_path_body}"
    );

    let old_release_page = expect_status(
        request(
            &app,
            Method::GET,
            &format!(
                "/api/public/sites/{project_id}/page?path=guide%2Fstart&lang=ar-EG&version=main"
            ),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "keep published v1 paths immutable after editor rename",
    )?;
    anyhow::ensure!(old_release_page["data"]["page"]["id"] == arabic_page_id);
    anyhow::ensure!(old_release_page["data"]["activeVersion"] == "main");
    anyhow::ensure!(old_release_page["data"]["version"] == 1);
    expect_status(
        request(
            &app,
            Method::GET,
            &format!(
                "/api/public/sites/{project_id}/page?path=quickstart%2Fstart&lang=ar-EG&version=main"
            ),
            None,
            None,
        )
        .await?,
        StatusCode::NOT_FOUND,
        "do not leak unpublished editor path into v1",
    )?;
    let old_release_search = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/sites/{project_id}/search?q=quickstart&lang=ar-EG&version=main"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "search must not expose paths changed after v1",
    )?;
    anyhow::ensure!(old_release_search["data"]["hits"]
        .as_array()
        .is_some_and(Vec::is_empty));

    // A draft is neither publicly readable nor included in deployment output.
    expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/sites/{project_id}/page?path=quickstart%2Fdraft&lang=ar-EG"),
            None,
            None,
        )
        .await?,
        StatusCode::NOT_FOUND,
        "hide unpublished page",
    )?;

    let default_delete = request(
        &app,
        Method::DELETE,
        &format!("/api/app/projects/{project_id}/languages/{english_language_id}"),
        Some(&cookie),
        None,
    )
    .await?;
    expect_status(
        default_delete,
        StatusCode::CONFLICT,
        "protect default language",
    )?;

    // A second release captures the editor changes and gets the next immutable
    // branch-local version. Public reads switch only after this commit.
    let second_deployment = expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/deployments"),
            Some(&cookie),
            Some(json!({ "message": "E2E release after rename" })),
        )
        .await?,
        StatusCode::OK,
        "publish updated project release",
    )?;
    anyhow::ensure!(second_deployment["data"]["status"] == "PENDING");
    let second_deployment_id =
        required_string(&second_deployment["data"], "id", "second deployment")?;
    let (second_version, second_pages) =
        wait_for_deployment_ready(&state, second_deployment_id).await?;
    anyhow::ensure!(second_version == 2);
    anyhow::ensure!(second_pages == 2);
    let second_english_artifact = state
        .storage
        .get(&format!(
            "sites/{project_id}/{second_deployment_id}/en/guide/start.html"
        ))
        .await?;
    anyhow::ensure!(String::from_utf8(second_english_artifact.to_vec())?
        .contains("Editor-only change after release one"));
    let (ssr_after_v2_status, ssr_after_v2_html) =
        site_request(&app, &public_host, "/guide/start").await?;
    anyhow::ensure!(ssr_after_v2_status == StatusCode::OK);
    anyhow::ensure!(ssr_after_v2_html.contains("Editor-only change after release one"));

    let custom_hostname = format!("docs-{}.example.invalid", Uuid::new_v4().simple());
    let custom_domain = cms_db::domain::DomainQueries::create(
        &state.biz_context.pool,
        second_deployment_id,
        &custom_hostname,
        true,
    )
    .await?;
    let (unverified_host_status, unverified_host_body) =
        site_request(&app, &custom_hostname, "/guide/start").await?;
    anyhow::ensure!(unverified_host_status == StatusCode::OK);
    anyhow::ensure!(!unverified_host_body.contains("Editor-only change after release one"));
    let unverified_domain_resolution = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/domains/resolve?host={custom_hostname}"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "do not resolve an unverified public hostname",
    )?;
    anyhow::ensure!(unverified_domain_resolution["data"].is_null());
    expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/domains/tls-authorize?domain={custom_hostname}"),
            None,
            None,
        )
        .await?,
        StatusCode::NOT_FOUND,
        "do not issue TLS for an unverified hostname",
    )?;

    let second_arabic_artifact = state
        .storage
        .get(&format!(
            "sites/{project_id}/{second_deployment_id}/ar-EG/quickstart/start.html"
        ))
        .await?;
    anyhow::ensure!(
        String::from_utf8(second_arabic_artifact.to_vec())?.contains("محتوى الإصدار العربي")
    );
    let second_public_page = expect_status(
        request(
            &app,
            Method::GET,
            &format!(
                "/api/public/sites/{project_id}/page?path=quickstart%2Fstart&lang=ar-EG&version=main"
            ),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "switch public reads to newly committed release",
    )?;
    anyhow::ensure!(second_public_page["data"]["version"] == 2);
    anyhow::ensure!(second_public_page["data"]["page"]["path"] == "quickstart/start");
    expect_status(
        request(
            &app,
            Method::GET,
            &format!(
                "/api/public/sites/{project_id}/page?path=guide%2Fstart&lang=ar-EG&version=main"
            ),
            None,
            None,
        )
        .await?,
        StatusCode::NOT_FOUND,
        "remove stale page path from latest release",
    )?;
    let new_release_search = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/sites/{project_id}/search?q=quickstart&lang=ar-EG&version=main"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "search the updated release in the selected language",
    )?;
    anyhow::ensure!(new_release_search["data"]["hits"]
        .as_array()
        .is_some_and(|hits| { hits.iter().any(|hit| hit["id"] == arabic_page_id) }));
    let page_by_id_after_v2 = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/pages/{arabic_page_id}"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "page-ID reads follow the latest immutable release",
    )?;
    anyhow::ensure!(page_by_id_after_v2["data"]["page"]["path"] == "quickstart/start");
    let changelog = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/sites/{project_id}/changelog"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "read snapshot-backed changelog",
    )?;
    anyhow::ensure!(changelog["data"]
        .as_array()
        .is_some_and(|items| items.len() == 2));
    anyhow::ensure!(changelog["data"][0]["version"] == 2);

    let rollback = expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/deployments/{deployment_id}/rollback"),
            Some(&cookie),
            Some(json!({})),
        )
        .await?,
        StatusCode::OK,
        "rollback by publishing a copy of the target snapshot",
    )?;
    anyhow::ensure!(rollback["data"]["status"] == "PENDING");
    let rollback_id = required_string(&rollback["data"], "id", "rollback deployment")?;
    let (rollback_version, rollback_pages) = wait_for_deployment_ready(&state, rollback_id).await?;
    anyhow::ensure!(rollback_version == 3);
    anyhow::ensure!(rollback_pages == 2);
    anyhow::ensure!(
        state
            .storage
            .exists(&format!(
                "sites/{project_id}/{rollback_id}/ar-EG/guide/start.html"
            ))
            .await?
    );
    let public_after_rollback = expect_status(
        request(
            &app,
            Method::GET,
            &format!(
                "/api/public/sites/{project_id}/page?path=guide%2Fstart&lang=ar-EG&version=main"
            ),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "serve rollback snapshot as latest release",
    )?;
    anyhow::ensure!(public_after_rollback["data"]["version"] == 3);
    let english_after_rollback = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/sites/{project_id}/page?path=guide%2Fstart&lang=en&version=main"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "restore the first release's English page on rollback",
    )?;
    anyhow::ensure!(english_after_rollback["data"]["page"]["content"]
        .as_str()
        .is_some_and(|content| content.contains("English release content")));
    anyhow::ensure!(!english_after_rollback["data"]["page"]["content"]
        .as_str()
        .unwrap()
        .contains("Editor-only change after release one"));
    let (ssr_after_rollback_status, ssr_after_rollback_html) =
        site_request(&app, &public_host, "/guide/start").await?;
    anyhow::ensure!(ssr_after_rollback_status == StatusCode::OK);
    anyhow::ensure!(ssr_after_rollback_html.contains("English release content"));
    anyhow::ensure!(!ssr_after_rollback_html.contains("Editor-only change after release one"));

    // The custom hostname stays bound to deployment 2 even after the default
    // branch is rolled back to v1 as release 3.
    cms_db::domain::DomainQueries::verify(
        &state.biz_context.pool,
        &custom_domain.id,
        &custom_domain.verification_token,
    )
    .await?;
    // This fixture changes the database directly instead of going through the
    // product verification route, so explicitly invalidate the router cache.
    state.invalidate_host_resolution_cache();
    expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/domains/tls-authorize?domain={custom_hostname}"),
            None,
            None,
        )
        .await?,
        StatusCode::NO_CONTENT,
        "authorize TLS for a verified hostname with a published release",
    )?;
    let (custom_host_status, custom_host_html) =
        site_request(&app, &custom_hostname, "/guide/start").await?;
    anyhow::ensure!(custom_host_status == StatusCode::OK);
    anyhow::ensure!(custom_host_html.contains("Editor-only change after release one"));
    anyhow::ensure!(custom_host_html.contains(&format!(
        "<link rel=\"canonical\" href=\"https://{custom_hostname}/guide/start\">"
    )));
    let (custom_sitemap_status, custom_sitemap) =
        site_request(&app, &custom_hostname, "/sitemap.xml").await?;
    anyhow::ensure!(custom_sitemap_status == StatusCode::OK);
    anyhow::ensure!(custom_sitemap.contains(&format!("https://{custom_hostname}/guide/start")));
    let verified_domain_resolution = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/domains/resolve?host={custom_hostname}"),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "resolve a verified custom hostname",
    )?;
    anyhow::ensure!(verified_domain_resolution["data"]["projectId"] == project_id);
    anyhow::ensure!(verified_domain_resolution["data"]["verified"] == true);

    let (untrusted_https_status, _) =
        site_request_through_tls_proxy(&app, &custom_hostname, "/", None).await?;
    anyhow::ensure!(untrusted_https_status == StatusCode::OK);
    let pending_tls_status: String =
        sqlx::query_scalar(r#"SELECT ssl_status FROM "Domain" WHERE id = $1"#)
            .bind(&custom_domain.id)
            .fetch_one(&state.biz_context.pool)
            .await?;
    anyhow::ensure!(
        pending_tls_status == "PENDING",
        "client-supplied HTTPS headers must not mark TLS active"
    );

    let (trusted_https_status, _) = site_request_through_tls_proxy(
        &app,
        &custom_hostname,
        "/",
        Some("e2e-trusted-proxy-secret"),
    )
    .await?;
    anyhow::ensure!(trusted_https_status == StatusCode::OK);
    let (active_tls_status, tls_checked_at): (String, Option<chrono::DateTime<Utc>>) =
        sqlx::query_as(r#"SELECT ssl_status, ssl_checked_at FROM "Domain" WHERE id = $1"#)
            .bind(&custom_domain.id)
            .fetch_one(&state.biz_context.pool)
            .await?;
    anyhow::ensure!(active_tls_status == "ACTIVE" && tls_checked_at.is_some());

    // Unrelated partial updates retain hostname-bound state. Changing the
    // hostname rotates the DNS token and atomically clears all TLS observations.
    let previous_expiry = (Utc::now() + Duration::days(30))
        .with_nanosecond(0)
        .expect("a whole-second timestamp is valid");
    sqlx::query(
        r#"UPDATE "Domain"
           SET ssl_certificate = 'e2e-public-cert', ssl_certificate_expires_at = $2,
               ssl_last_error = 'stale test detail', acme_order_url = 'https://acme.invalid/order'
           WHERE id = $1"#,
    )
    .bind(&custom_domain.id)
    .bind(previous_expiry)
    .execute(&state.biz_context.pool)
    .await?;
    expect_status(
        request(
            &app,
            Method::PUT,
            &format!("/api/domains/{}", custom_domain.id),
            Some(&cookie),
            Some(json!({ "is_primary": false })),
        )
        .await?,
        StatusCode::OK,
        "apply an unrelated domain update through the authenticated API",
    )?;
    let partial_update =
        cms_db::domain::DomainQueries::get_by_id(&state.biz_context.pool, &custom_domain.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("domain disappeared after partial update"))?;
    anyhow::ensure!(partial_update.verified_at.is_some());
    anyhow::ensure!(partial_update.verification_token == custom_domain.verification_token);
    anyhow::ensure!(partial_update.ssl_certificate.as_deref() == Some("e2e-public-cert"));
    anyhow::ensure!(partial_update.ssl_certificate_expires_at == Some(previous_expiry));
    anyhow::ensure!(partial_update.ssl_status == "ACTIVE");
    anyhow::ensure!(partial_update.ssl_checked_at.is_some());
    anyhow::ensure!(partial_update.ssl_last_error.as_deref() == Some("stale test detail"));

    let replacement_hostname = format!("renamed-{}.example.invalid", Uuid::new_v4().simple());
    let hostname_update = expect_status(
        request(
            &app,
            Method::PUT,
            &format!("/api/domains/{}", custom_domain.id),
            Some(&cookie),
            Some(json!({ "hostname": replacement_hostname.clone() })),
        )
        .await?,
        StatusCode::OK,
        "change a custom hostname through the authenticated API",
    )?;
    anyhow::ensure!(hostname_update["hostname"] == replacement_hostname);
    let changed_domain =
        cms_db::domain::DomainQueries::get_by_id(&state.biz_context.pool, &custom_domain.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("domain disappeared after hostname change"))?;
    anyhow::ensure!(changed_domain.hostname == replacement_hostname);
    anyhow::ensure!(changed_domain.verification_token != custom_domain.verification_token);
    anyhow::ensure!(changed_domain.verified_at.is_none());
    anyhow::ensure!(changed_domain.ssl_certificate.is_none());
    anyhow::ensure!(changed_domain.ssl_certificate_expires_at.is_none());
    anyhow::ensure!(changed_domain.ssl_status == "PENDING");
    anyhow::ensure!(changed_domain.ssl_checked_at.is_none());
    anyhow::ensure!(changed_domain.ssl_last_error.is_none());
    let acme_order: Option<String> =
        sqlx::query_scalar(r#"SELECT acme_order_url FROM "Domain" WHERE id = $1"#)
            .bind(&custom_domain.id)
            .fetch_one(&state.biz_context.pool)
            .await?;
    anyhow::ensure!(acme_order.is_none());
    for hostname in [&custom_hostname, &replacement_hostname] {
        expect_status(
            request(
                &app,
                Method::GET,
                &format!("/api/public/domains/tls-authorize?domain={hostname}"),
                None,
                None,
            )
            .await?,
            StatusCode::NOT_FOUND,
            "revoke TLS authorization after a hostname change",
        )?;
    }
    let (old_host_after_rename_status, old_host_after_rename_html) =
        site_request(&app, &custom_hostname, "/guide/start").await?;
    anyhow::ensure!(old_host_after_rename_status == StatusCode::OK);
    anyhow::ensure!(
        !old_host_after_rename_html.contains("Editor-only change after release one"),
        "a cached old-host mapping must be invalidated immediately after rename"
    );
    let (new_host_before_verification_status, new_host_before_verification_html) =
        site_request(&app, &replacement_hostname, "/guide/start").await?;
    anyhow::ensure!(new_host_before_verification_status == StatusCode::OK);
    anyhow::ensure!(
        !new_host_before_verification_html.contains("Editor-only change after release one")
    );

    expect_status(
        request(
            &app,
            Method::GET,
            &format!(
                "/api/public/sites/{project_id}/page?path=quickstart%2Fstart&lang=ar-EG&version=main"
            ),
            None,
            None,
        )
        .await?,
        StatusCode::NOT_FOUND,
        "rollback removes newer path from public release",
    )?;

    // Deleting a navigation group must not strand its children with stale
    // materialized paths. They become root documents while the finished release
    // artifact remains untouched.
    let delete_group = expect_status(
        request(
            &app,
            Method::DELETE,
            &format!("/api/app/projects/{project_id}/pages/{arabic_group_id}"),
            Some(&cookie),
            None,
        )
        .await?,
        StatusCode::OK,
        "delete Arabic navigation group",
    )?;
    anyhow::ensure!(delete_group["data"]["success"] == true);
    let reparented_page = expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/app/projects/{project_id}/pages/{arabic_page_id}"),
            Some(&cookie),
            None,
        )
        .await?,
        StatusCode::OK,
        "read child reparented after group deletion",
    )?;
    anyhow::ensure!(reparented_page["data"]["path"] == "/start");
    anyhow::ensure!(reparented_page["data"]["parentId"].is_null());
    anyhow::ensure!(
        state
            .storage
            .exists(&format!(
                "sites/{project_id}/{deployment_id}/ar-EG/guide/start.html"
            ))
            .await?
    );
    anyhow::ensure!(
        state
            .storage
            .exists(&format!(
                "sites/{project_id}/{second_deployment_id}/ar-EG/quickstart/start.html"
            ))
            .await?
    );
    anyhow::ensure!(
        state
            .storage
            .exists(&format!(
                "sites/{project_id}/{rollback_id}/ar-EG/guide/start.html"
            ))
            .await?
    );
    let public_after_delete = expect_status(
        request(
            &app,
            Method::GET,
            &format!(
                "/api/public/sites/{project_id}/page?path=guide%2Fstart&lang=ar-EG&version=main"
            ),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "keep rollback snapshot immutable after navigation deletion",
    )?;
    anyhow::ensure!(public_after_delete["data"]["version"] == 3);

    // Publish a real non-default preview branch. Public readers can select it
    // explicitly, but the ordinary project host must remain on the default
    // branch rather than exposing the most recently published branch.
    let preview_branch_response = expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/branches"),
            Some(&cookie),
            Some(json!({
                "project_id": project_id,
                "name": "Preview",
                "is_protected": true,
            })),
        )
        .await?,
        StatusCode::OK,
        "create non-default preview branch",
    )?;
    let preview_branch = preview_branch_response["data"].clone();
    let preview_branch_id = required_string(&preview_branch, "id", "preview branch")?;
    anyhow::ensure!(preview_branch["is_default"] == false);
    let preview_page = create_page(
        &app,
        &cookie,
        &project_id,
        json!({
            "title": "Preview only",
            "slug": "preview-only",
            "kind": "PAGE",
            "branchId": preview_branch_id,
            "languageId": english_language_id,
            "isPublished": true,
            "content": "This page belongs only to the preview branch.",
        }),
    )
    .await?;
    let preview_page_id = required_string(&preview_page, "id", "preview-only page")?;
    let preview_deployment = expect_status(
        request(
            &app,
            Method::POST,
            &format!("/api/app/projects/{project_id}/branches/{preview_branch_id}/merge"),
            Some(&cookie),
            Some(json!({})),
        )
        .await?,
        StatusCode::OK,
        "build and publish the preview branch through the product route",
    )?;
    anyhow::ensure!(preview_deployment["data"]["status"] == "PENDING");
    let preview_deployment_id =
        required_string(&preview_deployment["data"], "id", "preview deployment")?;
    let (preview_version, preview_pages) =
        wait_for_deployment_ready(&state, preview_deployment_id).await?;
    anyhow::ensure!(preview_version == 1);
    anyhow::ensure!(preview_pages == 1);

    let selected_preview_page = expect_status(
        request(
            &app,
            Method::GET,
            &format!(
                "/api/public/sites/{project_id}/page?path=preview-only&lang=en&version=preview"
            ),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "read a page from its explicitly selected preview branch",
    )?;
    anyhow::ensure!(selected_preview_page["data"]["page"]["id"] == preview_page_id);
    let default_branch_page = request(
        &app,
        Method::GET,
        &format!("/api/public/sites/{project_id}/page?path=preview-only&lang=en&version=main"),
        None,
        None,
    )
    .await?;
    expect_status(
        default_branch_page,
        StatusCode::NOT_FOUND,
        "keep branch content isolated",
    )?;
    let (preview_path_status, preview_path_body) =
        site_request(&app, &public_host, "/preview-only").await?;
    anyhow::ensure!(
        preview_path_status == StatusCode::NOT_FOUND,
        "default public host must not expose a preview-branch page: {preview_path_body}"
    );

    // Edge case: if the default branch has no successful active release, a
    // preview release must not silently become the default public site.
    let default_branch_id: String = sqlx::query_scalar(
        r#"SELECT id FROM "Branch" WHERE project_id = $1 AND is_default = TRUE"#,
    )
    .bind(&project_id)
    .fetch_one(&state.biz_context.pool)
    .await?;
    sqlx::query(
        r#"UPDATE "Deployment" SET status = 'FAILED', updated_at = NOW()
           WHERE project_id = $1 AND branch_id = $2 AND status = 'ACTIVE'"#,
    )
    .bind(&project_id)
    .bind(default_branch_id)
    .execute(&state.biz_context.pool)
    .await?;
    let (no_default_release_status, no_default_release_body) =
        site_request(&app, &public_host, "/").await?;
    anyhow::ensure!(
        no_default_release_status == StatusCode::NOT_FOUND,
        "do not fall back to a non-default branch when main has no active release: {no_default_release_body}"
    );
    expect_status(
        request(
            &app,
            Method::GET,
            &format!("/api/public/sites/{project_id}"),
            None,
            None,
        )
        .await?,
        StatusCode::NOT_FOUND,
        "do not default public API reads to a preview branch",
    )?;
    let still_explicitly_selectable_preview = expect_status(
        request(
            &app,
            Method::GET,
            &format!(
                "/api/public/sites/{project_id}/page?path=preview-only&lang=en&version=preview"
            ),
            None,
            None,
        )
        .await?,
        StatusCode::OK,
        "allow explicit preview-branch reads when main has no release",
    )?;
    anyhow::ensure!(still_explicitly_selectable_preview["data"]["page"]["id"] == preview_page_id);

    Ok(())
}

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL database; run `cargo xtask e2e`"]
async fn product_flow_creates_and_publishes_language_scoped_docs() -> anyhow::Result<()> {
    let database_url = std::env::var("CMS_E2E_DATABASE_URL").map_err(|_| {
        anyhow::anyhow!("set CMS_E2E_DATABASE_URL to a disposable PostgreSQL database")
    })?;
    let mut config = Config::default();
    config.database.url = database_url;
    config.site.self_host = Some("cms.app".to_string());
    config.domain_tls.proxy_secret = Some("e2e-trusted-proxy-secret".to_string());
    let storage_root = std::env::temp_dir().join(format!("cms-product-e2e-{}", Uuid::new_v4()));
    config.storage.local_root = Some(storage_root.to_string_lossy().into_owned());

    let state = Arc::new(AppState::from_config(&config).await?);
    let worker_state = Arc::new(cms_worker::app_state::WorkerState::from_app_state(&state).await?);
    let (worker_shutdown_tx, worker_shutdown_rx) = tokio::sync::watch::channel(false);
    cms_worker::start_consumers_with_shutdown(
        state.job_queue.clone(),
        worker_state,
        worker_shutdown_rx,
    )
    .await?;

    let seed_data = seed(&state).await?;
    let result = run_flow(state.clone(), &seed_data).await;
    let _ = worker_shutdown_tx.send(true);
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    let cleanup_result = cleanup(&state, &seed_data).await;
    state.biz_context.pool.close().await;
    cleanup_result?;
    result
}
