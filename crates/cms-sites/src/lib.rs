//! CMS Sites
//!
//! This crate handles serving published documentation sites and marketing pages.
//! It provides host resolution, Markdown to HTML rendering, SEO/machine files,
//! and security headers for published sites.
//!
//! This replaces the functionality in `apps/app/src/server.ts` from the original
//! TypeScript monorepo.

pub mod host_resolution;
pub mod markdown_renderer;
pub mod security;
pub mod seo;
pub mod static_files;

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Extension, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Router,
};
use cms_db::{deployment::DeploymentQueries, domain::DomainQueries, project::ProjectQueries};
use cms_entity::deployment::DeploymentSnapshotContent;
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use host_resolution::{HostResolutionResult, HostResolver};
use markdown_renderer::{HighlightTheme, MarkdownRenderer, MarkdownRendererConfig};
use security::SiteSecurityHeaders;
use seo::{SeoGenerator, SitemapGenerator, SitemapPageMetadata};
use static_files::StaticFileServer;

/// AppState for sites - this will be provided by the binary crate
pub type SitesAppState = AppState;

/// Create the sites router
pub fn create_sites_router(state: Arc<SitesAppState>) -> Router {
    create_router(state)
}

/// Alias for create_sites_router (for compatibility with main.rs)
pub fn sites_router(state: Arc<SitesAppState>) -> Router {
    create_sites_router(state)
}

/// Helper to get MIME type by file extension
fn get_mime_type(ext: &str) -> &'static str {
    match ext {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "json" | "webmanifest" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "map" => "application/json",
        _ => "application/octet-stream",
    }
}

/// Serve static file or SPA index.html fallback from dist/frontend
pub fn serve_spa_file(path: &str) -> Response {
    let sanitized = path.trim_start_matches('/').replace('\\', "/");
    if sanitized.starts_with("api/") || sanitized == "api" {
        let json = serde_json::json!({
            "error": {
                "code": "not_found",
                "message": "API endpoint not found"
            }
        });
        let mut res = Response::new(Body::from(serde_json::to_vec(&json).unwrap()));
        *res.status_mut() = StatusCode::NOT_FOUND;
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        return res;
    }
    let candidate_dirs = [
        "dist/frontend",
        "frontend/dist",
        "../dist/frontend",
        "../frontend/dist",
    ];

    // 1. If path is a specific static file, try to serve it
    if !sanitized.is_empty() {
        for dir in &candidate_dirs {
            let file_path = std::path::Path::new(dir).join(&sanitized);
            if file_path.is_file() {
                if let Ok(bytes) = std::fs::read(&file_path) {
                    let ext = file_path
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let mime = get_mime_type(&ext);
                    let mut res = Response::new(Body::from(bytes));
                    if let Ok(val) = header::HeaderValue::from_str(mime) {
                        res.headers_mut().insert(header::CONTENT_TYPE, val);
                    }
                    if ext != "html" {
                        res.headers_mut().insert(
                            header::CACHE_CONTROL,
                            header::HeaderValue::from_static("public, max-age=31536000, immutable"),
                        );
                    }
                    return res;
                }
            }
        }
    }

    // 2. SPA fallback: serve index.html for client-side routing
    for dir in &candidate_dirs {
        let index_path = std::path::Path::new(dir).join("index.html");
        if index_path.is_file() {
            if let Ok(bytes) = std::fs::read(&index_path) {
                let mut res = Response::new(Body::from(bytes));
                res.headers_mut().insert(
                    header::CONTENT_TYPE,
                    header::HeaderValue::from_static("text/html; charset=utf-8"),
                );
                res.headers_mut().insert(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("no-cache"),
                );
                return res;
            }
        }
    }

    // 3. Fallback HTML if build artifacts are missing
    Html(r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>CMS</title></head><body><div id="root"><h1>CMS App</h1><p>Frontend assets not found in dist/frontend.</p></div></body></html>"#.to_string()).into_response()
}

/// Create the full router with all site routes
fn create_router(state: Arc<SitesAppState>) -> Router {
    use axum::routing::get;

    let host_resolver = Arc::new(HostResolver::with_generation(
        state.biz_context.pool.clone(),
        site_host(&state),
        state.host_resolution_generation.clone(),
    ));

    Router::new()
        // Main site handler - resolves host and serves appropriate content
        .route("/", get(root_handler))
        .route("/{*path}", get(wildcard_handler))
        // SEO files
        .route("/robots.txt", get(robots_txt_handler))
        .route("/sitemap.xml", get(sitemap_xml_handler))
        .route("/.well-known/security.txt", get(security_txt_handler))
        .route("/.well-known/pgp-key.txt", get(pgp_key_handler))
        // Static assets for published sites / SPA
        .route("/assets/{*path}", get(asset_handler))
        .route("/css/{*path}", get(css_handler))
        .route("/js/{*path}", get(js_handler))
        .route("/fonts/{*path}", get(font_handler))
        .route("/images/{*path}", get(image_handler))
        // Favicon
        .route("/favicon.ico", get(favicon_handler))
        .route("/favicon-32x32.png", get(favicon_32_handler))
        .route("/favicon-16x16.png", get(favicon_16_handler))
        .route("/apple-touch-icon.png", get(apple_touch_icon_handler))
        // Manifest
        .route("/site.webmanifest", get(manifest_handler))
        .layer(Extension(host_resolver))
        .with_state(state)
}

/// Derive the site base host from config, falling back to `"cms.app"`.
///
/// Prefers the operator's `site.self_host`, then `site.marketing_host`, then the
/// historical default — so self-hosted operators and branded deployments don't
/// need to bake `cms.app` into the binary.
fn site_host(state: &Arc<SitesAppState>) -> String {
    let site = &state.config.site;
    site.self_host
        .clone()
        .or_else(|| site.marketing_host.clone())
        .map(|h| {
            h.trim_start_matches("https://")
                .trim_start_matches("http://")
                .to_string()
        })
        .unwrap_or_else(|| "cms.app".to_string())
}

/// Record a successful HTTPS request only when it carries the secret injected by
/// the configured TLS-terminating proxy. The proxy remains responsible for
/// ACME issuance and renewal; this observation is operational status only.
async fn record_trusted_tls_observation(
    state: &Arc<SitesAppState>,
    headers: &HeaderMap,
    resolution: &HostResolutionResult,
) {
    if !resolution.is_custom_domain
        || !headers
            .get("x-forwarded-proto")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https"))
    {
        return;
    }

    let Some(expected_secret) = state.config.domain_tls.proxy_secret.as_deref() else {
        return;
    };
    let trusted = headers
        .get("x-cms-proxy-token")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|provided| provided == expected_secret);
    if !trusted {
        return;
    }

    let Some(domain_id) = resolution.domain_id.as_deref() else {
        return;
    };
    if let Err(error) = DomainQueries::mark_tls_active(&state.biz_context.pool, domain_id).await {
        // TLS status reporting must never take down a successfully published site.
        tracing::warn!(%error, domain_id, "failed to record trusted TLS observation");
    }
}

/// The immutable public view of a successfully deployed documentation site.
/// Editor tables are deliberately not used to render published pages.
struct PublishedSiteSnapshot {
    project: cms_entity::project::Project,
    deployment_id: String,
    language_id: String,
    language_code: String,
    language_is_rtl: bool,
}

async fn load_published_site_snapshot(
    state: &Arc<SitesAppState>,
    project_id: &str,
    preferred_deployment_id: Option<&str>,
) -> Result<PublishedSiteSnapshot, AppError> {
    // Public visibility can be revoked immediately, even though old snapshots
    // remain retained for rollback and audit history.
    let current_project = ProjectQueries::get_by_id(&state.biz_context.pool, project_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;
    if !current_project.is_public
        || cms_db::project::ProjectQueries::is_taken_down(&state.biz_context.pool, project_id)
            .await?
            .unwrap_or(false)
    {
        return Err(AppError::NotFound("Published site not found".to_string()));
    }

    let snapshot = if let Some(preferred_deployment_id) = preferred_deployment_id {
        DeploymentQueries::get_snapshot_by_deployment(
            &state.biz_context.pool,
            preferred_deployment_id,
        )
        .await?
        .filter(|snapshot| snapshot.project_id == project_id)
        .ok_or_else(|| AppError::NotFound("Domain release is not published".to_string()))?
    } else {
        let releases =
            DeploymentQueries::get_latest_branch_releases(&state.biz_context.pool, project_id)
                .await?;
        let default_release = releases
            .iter()
            .find(|release| release.is_default)
            .ok_or_else(|| AppError::NotFound("No default-branch release found".to_string()))?;
        DeploymentQueries::get_snapshot_by_deployment(
            &state.biz_context.pool,
            &default_release.deployment_id,
        )
        .await?
        .filter(|snapshot| snapshot.project_id == project_id)
        .ok_or_else(|| AppError::NotFound("Published release not found".to_string()))?
    };
    let deployment_id = snapshot.deployment_id.clone();
    let content: DeploymentSnapshotContent = serde_json::from_value(snapshot.snapshot)?;
    if content.project.id != project_id
        || content.branch.id != snapshot.branch_id
        || content.branch.project_id != project_id
    {
        return Err(AppError::Conflict(
            "Published snapshot scope does not match the requested site".to_string(),
        ));
    }

    let default_language = content
        .languages
        .iter()
        .find(|language| language.is_default && language.enabled)
        .or_else(|| content.languages.iter().find(|language| language.enabled))
        .ok_or_else(|| AppError::NotFound("Published language not found".to_string()))?;
    Ok(PublishedSiteSnapshot {
        project: content.project,
        deployment_id,
        language_id: default_language.id.clone(),
        language_code: default_language.code.clone(),
        language_is_rtl: default_language.is_rtl,
    })
}

async fn load_published_site_page_listing(
    state: &Arc<SitesAppState>,
    site: &PublishedSiteSnapshot,
) -> Result<Vec<cms_db::deployment::DeploymentSnapshotPageListing>, AppError> {
    DeploymentQueries::get_snapshot_page_listing(
        &state.biz_context.pool,
        &site.deployment_id,
        &site.language_id,
    )
    .await
}

/// Get markdown renderer from app state
fn get_markdown_renderer(_state: &Arc<SitesAppState>) -> MarkdownRenderer {
    let config = MarkdownRendererConfig {
        enable_syntax_highlighting: true,
        enable_toc: true,
        enable_footnotes: true,
        enable_task_lists: true,
        enable_strikethrough: true,
        enable_emoji: true,
        highlight_theme: HighlightTheme::default(),
        enable_heading_ids: true,
        base_url: "/".to_string(),
    };
    MarkdownRenderer::new(config)
}

fn site_base_url(
    state: &Arc<SitesAppState>,
    resolution: &HostResolutionResult,
    project: &cms_entity::project::Project,
) -> String {
    if resolution.is_custom_domain {
        format!("https://{}", resolution.hostname)
    } else {
        format!("https://{}.{}", project.slug, site_host(state))
    }
}

fn get_seo_generator(base_url: &str) -> SeoGenerator {
    SeoGenerator::new(base_url.to_string())
}

/// Root handler - resolves host and serves appropriate content
async fn root_handler(
    State(state): State<Arc<SitesAppState>>,
    Extension(host_resolver): Extension<Arc<HostResolver>>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    // Resolve host to project using the router-scoped cache.
    let resolution = host_resolver.resolve(&headers).await?;
    if let Some(result) = resolution.as_ref() {
        record_trusted_tls_observation(&state, &headers, result).await;
    }

    match resolution {
        Some(result) => serve_project_index(&state, &result)
            .await
            .map(|html| html.into_response()),
        None => Ok(serve_spa_file("index.html")),
    }
}

/// Wildcard handler for all paths
async fn wildcard_handler(
    State(state): State<Arc<SitesAppState>>,
    Extension(host_resolver): Extension<Arc<HostResolver>>,
    headers: HeaderMap,
    Path(path): Path<String>,
) -> Result<Response, AppError> {
    // Resolve host to project using the router-scoped cache.
    let resolution = host_resolver.resolve(&headers).await?;
    if let Some(result) = resolution.as_ref() {
        record_trusted_tls_observation(&state, &headers, result).await;
    }

    match resolution {
        Some(result) => serve_page(&state, &result, &path)
            .await
            .map(|html| html.into_response()),
        None => Ok(serve_spa_file(&path)),
    }
}

/// Serve project index page
async fn serve_project_index(
    state: &Arc<SitesAppState>,
    resolution: &HostResolutionResult,
) -> Result<Html<String>, AppError> {
    let site = load_published_site_snapshot(
        state,
        &resolution.project_id,
        resolution.deployment_id.as_deref(),
    )
    .await?;

    let pages = load_published_site_page_listing(state, &site).await?;

    // Find the root page (index or home) from metadata, then fetch only that
    // one body. Listing views never pull page-content JSON from PostgreSQL.
    let root_page_id = pages
        .iter()
        .find(|page| page.path == "/")
        .or_else(|| {
            pages.iter().find(|page| {
                page.parent_id.is_none()
                    && (page.slug == "index" || page.slug == "home" || page.slug == "readme")
            })
        })
        .map(|page| page.page_id.clone());

    let base_url = site_base_url(state, resolution, &site.project);
    match root_page_id {
        Some(page_id) => {
            let page = DeploymentQueries::get_snapshot_page(
                &state.biz_context.pool,
                &site.deployment_id,
                &page_id,
            )
            .await?
            .filter(|page| page.is_published && page.kind.eq_ignore_ascii_case("PAGE"))
            .ok_or_else(|| AppError::NotFound("Published root page not found".to_string()))?;
            let html = render_page(
                state,
                &page,
                &site.project,
                &base_url,
                &site.language_code,
                site.language_is_rtl,
            )
            .await?;
            Ok(Html(html))
        }
        None => {
            // No root page found, show only metadata from the same immutable release.
            let html = render_project_listing(
                state,
                &site.project,
                &pages,
                &base_url,
                &site.language_code,
                site.language_is_rtl,
            )
            .await?;
            Ok(Html(html))
        }
    }
}

/// Serve specific page
async fn serve_page(
    state: &Arc<SitesAppState>,
    resolution: &HostResolutionResult,
    path: &str,
) -> Result<Html<String>, AppError> {
    let site = load_published_site_snapshot(
        state,
        &resolution.project_id,
        resolution.deployment_id.as_deref(),
    )
    .await?;

    // The path/language/release index resolves one immutable body rather than
    // loading every document body for a single-page request.
    let normalized_path = path.trim_matches('/');
    let snapshot_path = format!("/{normalized_path}");
    let page = DeploymentQueries::get_snapshot_page_by_path(
        &state.biz_context.pool,
        &site.deployment_id,
        &site.language_id,
        &snapshot_path,
    )
    .await?
    .filter(|page| page.is_published);

    match page {
        Some(page) => {
            let base_url = site_base_url(state, resolution, &site.project);
            let html = render_page(
                state,
                &page,
                &site.project,
                &base_url,
                &site.language_code,
                site.language_is_rtl,
            )
            .await?;
            Ok(Html(html))
        }
        None => serve_not_found(),
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Render a page to HTML
async fn render_page(
    state: &Arc<SitesAppState>,
    page: &cms_entity::page::Page,
    project: &cms_entity::project::Project,
    base_url: &str,
    language_code: &str,
    language_is_rtl: bool,
) -> Result<String, AppError> {
    let markdown_renderer = get_markdown_renderer(state);
    let seo_generator = get_seo_generator(base_url);

    // Generate SEO metadata
    let metadata = seo_generator.generate_page_metadata(page, project);
    let seo_tags = seo_generator.generate_meta_tags(&metadata);
    let structured_data = seo_generator.generate_structured_data(&metadata, true);

    // Render markdown to HTML
    let content = markdown_renderer.render_page(page, project);

    // Generate full HTML
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="{}" dir="{}">
<head>
{}
{}
{}
</head>
<body>
{}
</body>
</html>"#,
        language_code,
        if language_is_rtl { "rtl" } else { "ltr" },
        seo_tags,
        structured_data,
        get_security_headers(),
        content
    );

    Ok(html)
}

/// Render project listing page
async fn render_project_listing(
    _state: &Arc<SitesAppState>,
    project: &cms_entity::project::Project,
    pages: &[cms_db::deployment::DeploymentSnapshotPageListing],
    base_url: &str,
    language_code: &str,
    language_is_rtl: bool,
) -> Result<String, AppError> {
    let seo_generator = get_seo_generator(base_url);

    // Generate SEO metadata
    let metadata = seo_generator.generate_project_metadata(project);
    let seo_tags = seo_generator.generate_meta_tags(&metadata);
    let structured_data = seo_generator.generate_structured_data(&metadata, false);

    // Generate page list using the nested path saved in the release.
    let mut page_list = String::new();
    for page in pages {
        let href = if page.path.starts_with('/') {
            page.path.clone()
        } else {
            format!("/{}", page.path)
        };
        page_list.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>\n",
            escape_html(&href),
            escape_html(&page.title)
        ));
    }

    // Generate full HTML. Escape user-controlled project metadata before
    // interpolating it into text nodes.
    let project_name = escape_html(&project.name);
    let project_description = escape_html(project.description.as_deref().unwrap_or(""));
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="{}" dir="{}">
<head>
{}
{}
{}
</head>
<body>
    <header>
        <h1>{}</h1>
        <p>{}</p>
    </header>
    <main>
        <h2>Pages</h2>
        <ul>
{}
        </ul>
    </main>
    <footer>
        <hr>
        <p>Powered by <a href="https://cms.com">CMS</a></p>
    </footer>
</body>
</html>"#,
        language_code,
        if language_is_rtl { "rtl" } else { "ltr" },
        seo_tags,
        structured_data,
        get_security_headers(),
        project_name,
        project_description,
        page_list
    );

    Ok(html)
}

/// Get security headers for published sites
fn get_security_headers() -> String {
    let security = SiteSecurityHeaders::default();
    let mut headers = String::new();

    for (name, value) in security.get_headers() {
        headers.push_str(&format!(
            "<meta http-equiv=\"{}\" content=\"{}\">\n",
            name, value
        ));
    }

    headers
}

/// Return 404 Not Found response
fn serve_not_found() -> Result<Html<String>, AppError> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>404 - Not Found</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            text-align: center;
            padding: 50px;
        }
        h1 {
            font-size: 50px;
            margin-bottom: 10px;
        }
        p {
            font-size: 20px;
            color: #666;
        }
    </style>
</head>
<body>
    <h1>404</h1>
    <p>Page not found</p>
</body>
</html>"#
        .to_string();
    Err(cms_error::AppError::NotFound(html))
}

/// Robots.txt handler - generates custom robots.txt per project
async fn robots_txt_handler(
    State(state): State<Arc<SitesAppState>>,
    Extension(host_resolver): Extension<Arc<HostResolver>>,
    headers: HeaderMap,
) -> Result<String, AppError> {
    match host_resolver.resolve(&headers).await? {
        Some(result) => {
            let site = load_published_site_snapshot(
                &state,
                &result.project_id,
                result.deployment_id.as_deref(),
            )
            .await?;

            // Generate robots.txt only for a project with a visible release.
            let base_url = if result.is_custom_domain {
                format!("https://{}", result.hostname)
            } else {
                format!("https://{}.{}", site.project.slug, site_host(&state))
            };
            Ok(format!(
                "User-agent: *\nAllow: /\nSitemap: {}/sitemap.xml\n\nDisallow: /api/\nDisallow: /admin/\nDisallow: /private/\n",
                base_url
            ))
        }
        None => {
            // Default robots.txt
            Ok(
                "User-agent: *\nAllow: /\nSitemap: /sitemap.xml\n\nDisallow: /api/\nDisallow: /admin/\nDisallow: /private/\n"
                    .to_string(),
            )
        }
    }
}

/// Sitemap.xml handler - generates URLs from the active immutable release.
async fn sitemap_xml_handler(
    State(state): State<Arc<SitesAppState>>,
    Extension(host_resolver): Extension<Arc<HostResolver>>,
    headers: HeaderMap,
) -> Result<String, AppError> {
    match host_resolver.resolve(&headers).await? {
        Some(result) => {
            let site = load_published_site_snapshot(
                &state,
                &result.project_id,
                result.deployment_id.as_deref(),
            )
            .await?;

            let base_url = if result.is_custom_domain {
                format!("https://{}", result.hostname)
            } else {
                format!("https://{}.{}", site.project.slug, site_host(&state))
            };

            let pages = load_published_site_page_listing(&state, &site).await?;
            let sitemap_pages = pages
                .into_iter()
                .map(|page| SitemapPageMetadata {
                    path: page.path,
                    updated_at: page.updated_at,
                })
                .collect::<Vec<_>>();
            let sitemap_gen = SitemapGenerator::new(base_url);
            Ok(sitemap_gen.generate_project_sitemap_from_metadata(&site.project, &sitemap_pages))
        }
        None => {
            // Default empty sitemap
            Ok("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n</urlset>".to_string())
        }
    }
}

/// Security.txt handler
async fn security_txt_handler() -> String {
    "Contact: security@cms.com\nEncryption: https://cms.com/.well-known/pgp-key.txt\nAcknowledgments: https://cms.com/security/acknowledgments\nPolicy: https://cms.com/security/policy\nHiring: https://cms.com/jobs".to_string()
}

/// PGP key handler
async fn pgp_key_handler() -> String {
    // In a real implementation, this would serve the actual PGP key
    "-----BEGIN PGP PUBLIC KEY BLOCK-----\n\n-----END PGP PUBLIC KEY BLOCK-----".to_string()
}

/// Asset handler for published site assets
async fn asset_handler(
    State(state): State<Arc<SitesAppState>>,
    Path(path): Path<String>,
) -> Result<Response, StatusCode> {
    let candidate = format!("assets/{}", path);
    let spa_res = serve_spa_file(&candidate);
    if spa_res.status() == StatusCode::OK
        && spa_res
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v != "text/html; charset=utf-8")
            .unwrap_or(false)
    {
        return Ok(spa_res);
    }
    let static_server = StaticFileServer::new(state.storage.clone());
    static_server.serve_file(&candidate).await
}

/// CSS handler
async fn css_handler(
    State(state): State<Arc<SitesAppState>>,
    Path(path): Path<String>,
) -> Result<Response, StatusCode> {
    let candidate = format!("css/{}", path);
    let spa_res = serve_spa_file(&candidate);
    if spa_res.status() == StatusCode::OK
        && spa_res
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v != "text/html; charset=utf-8")
            .unwrap_or(false)
    {
        return Ok(spa_res);
    }
    let static_server = StaticFileServer::new(state.storage.clone());
    static_server.serve_file(&candidate).await
}

/// JavaScript handler
async fn js_handler(
    State(state): State<Arc<SitesAppState>>,
    Path(path): Path<String>,
) -> Result<Response, StatusCode> {
    let candidate = format!("js/{}", path);
    let spa_res = serve_spa_file(&candidate);
    if spa_res.status() == StatusCode::OK
        && spa_res
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v != "text/html; charset=utf-8")
            .unwrap_or(false)
    {
        return Ok(spa_res);
    }
    let static_server = StaticFileServer::new(state.storage.clone());
    static_server.serve_file(&candidate).await
}

/// Font handler
async fn font_handler(
    State(state): State<Arc<SitesAppState>>,
    Path(path): Path<String>,
) -> Result<Response, StatusCode> {
    let candidate = format!("fonts/{}", path);
    let spa_res = serve_spa_file(&candidate);
    if spa_res.status() == StatusCode::OK
        && spa_res
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v != "text/html; charset=utf-8")
            .unwrap_or(false)
    {
        return Ok(spa_res);
    }
    let static_server = StaticFileServer::new(state.storage.clone());
    static_server.serve_file(&candidate).await
}

/// Image handler
async fn image_handler(
    State(state): State<Arc<SitesAppState>>,
    Path(path): Path<String>,
) -> Result<Response, StatusCode> {
    let candidate = format!("images/{}", path);
    let spa_res = serve_spa_file(&candidate);
    if spa_res.status() == StatusCode::OK
        && spa_res
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v != "text/html; charset=utf-8")
            .unwrap_or(false)
    {
        return Ok(spa_res);
    }
    let static_server = StaticFileServer::new(state.storage.clone());
    static_server.serve_file(&candidate).await
}

/// Favicon handler
async fn favicon_handler(State(state): State<Arc<SitesAppState>>) -> Result<Response, StatusCode> {
    let spa_res = serve_spa_file("favicon.ico");
    if spa_res.status() == StatusCode::OK
        && spa_res
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v != "text/html; charset=utf-8")
            .unwrap_or(false)
    {
        return Ok(spa_res);
    }
    let static_server = StaticFileServer::new(state.storage.clone());
    static_server.serve_file("favicon.ico").await
}

/// 32x32 favicon handler
async fn favicon_32_handler(
    State(state): State<Arc<SitesAppState>>,
) -> Result<Response, StatusCode> {
    let spa_res = serve_spa_file("favicon-32x32.png");
    if spa_res.status() == StatusCode::OK
        && spa_res
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v != "text/html; charset=utf-8")
            .unwrap_or(false)
    {
        return Ok(spa_res);
    }
    let static_server = StaticFileServer::new(state.storage.clone());
    static_server.serve_file("favicon-32x32.png").await
}

/// 16x16 favicon handler
async fn favicon_16_handler(
    State(state): State<Arc<SitesAppState>>,
) -> Result<Response, StatusCode> {
    let spa_res = serve_spa_file("favicon-16x16.png");
    if spa_res.status() == StatusCode::OK
        && spa_res
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v != "text/html; charset=utf-8")
            .unwrap_or(false)
    {
        return Ok(spa_res);
    }
    let static_server = StaticFileServer::new(state.storage.clone());
    static_server.serve_file("favicon-16x16.png").await
}

/// Apple touch icon handler
async fn apple_touch_icon_handler(
    State(state): State<Arc<SitesAppState>>,
) -> Result<Response, StatusCode> {
    let spa_res = serve_spa_file("apple-touch-icon.png");
    if spa_res.status() == StatusCode::OK
        && spa_res
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v != "text/html; charset=utf-8")
            .unwrap_or(false)
    {
        return Ok(spa_res);
    }
    let static_server = StaticFileServer::new(state.storage.clone());
    static_server.serve_file("apple-touch-icon.png").await
}

/// Site manifest handler
async fn manifest_handler(
    State(state): State<Arc<SitesAppState>>,
    Extension(host_resolver): Extension<Arc<HostResolver>>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let manifest = match host_resolver.resolve(&headers).await? {
        Some(result) => {
            let site = load_published_site_snapshot(
                &state,
                &result.project_id,
                result.deployment_id.as_deref(),
            )
            .await?;
            serde_json::json!({
                "name": site.project.name,
                "short_name": site.project.name,
                "description": site.project.description
                    .as_deref()
                    .unwrap_or("Documentation powered by CMS"),
                "start_url": "/",
                "display": "standalone",
                "background_color": "#ffffff",
                "theme_color": "#000000",
                "icons": [
                    {"src": "/apple-touch-icon.png", "sizes": "180x180", "type": "image/png"},
                    {"src": "/favicon-32x32.png", "sizes": "32x32", "type": "image/png"},
                    {"src": "/favicon-16x16.png", "sizes": "16x16", "type": "image/png"}
                ]
            })
        }
        None => {
            let spa_res = serve_spa_file("site.webmanifest");
            if spa_res.status() == StatusCode::OK {
                return Ok(spa_res);
            }
            serde_json::json!({
                "name": "CMS",
                "short_name": "CMS",
                "description": "Modern documentation platform",
                "start_url": "/",
                "display": "standalone",
                "background_color": "#ffffff",
                "theme_color": "#000000",
                "icons": [
                    {"src": "/apple-touch-icon.png", "sizes": "180x180", "type": "image/png"},
                    {"src": "/favicon-32x32.png", "sizes": "32x32", "type": "image/png"},
                    {"src": "/favicon-16x16.png", "sizes": "16x16", "type": "image/png"}
                ]
            })
        }
    };

    let body = serde_json::to_string_pretty(&manifest)?;
    let mut response = body.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/manifest+json"),
    );
    Ok(response)
}
