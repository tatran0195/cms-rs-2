//! Public handlers
//!
//! This module contains the actual implementation of public-facing handlers.
//! These handlers are for unauthenticated readers accessing published content.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use cms_biz::project::ProjectService;
use cms_entity::{page::PageResponse, project::ProjectResponse};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

/// Get a public project
///
/// Retrieve a project by organization slug and project slug.
/// This endpoint is publicly accessible without authentication.
#[utoipa::path(
    get,
    path = "/public/{org_slug}/{project_slug}",
    tag = "public",
    params(
        ("org_slug", Path, description = "Organization slug"),
        ("project_slug", Path, description = "Project slug"),
    ),
    responses(
        (status = 200, description = "Project found", body = ProjectResponse),
        (status = 404, description = "Project not found"),
    )
)]
pub async fn get_public_project_handler(
    State(state): State<Arc<AppState>>,
    Path((org_slug, project_slug)): Path<(String, String)>,
) -> Result<Json<ProjectResponse>, AppError> {
    let site = resolve_public_site_by_slugs(&state, &org_slug, &project_slug, None, None).await?;
    let page_count = site
        .pages
        .iter()
        .filter(|page| page.kind.eq_ignore_ascii_case("PAGE"))
        .count() as i64;
    let mut project: ProjectResponse = site.project.into();
    if let Some(count) = project._count.as_mut() {
        count.pages = page_count;
    }
    Ok(Json(project))
}

/// Get a public page
///
/// Retrieve a page by organization slug, project slug, and page path.
#[utoipa::path(
    get,
    path = "/public/{org_slug}/{project_slug}/{page_path}",
    tag = "public",
    params(
        ("org_slug", Path, description = "Organization slug"),
        ("project_slug", Path, description = "Project slug"),
        ("page_path", Path, description = "Page path or slug"),
    ),
    responses(
        (status = 200, description = "Page found", body = PageResponse),
        (status = 404, description = "Page not found"),
    )
)]
pub async fn get_public_page_handler(
    State(state): State<Arc<AppState>>,
    Path((org_slug, project_slug, page_path)): Path<(String, String, String)>,
) -> Result<Json<PageResponse>, AppError> {
    let site = resolve_public_site_by_slugs(&state, &org_slug, &project_slug, None, None).await?;
    let normalized_path = page_path.trim_matches('/');
    let page = site
        .pages
        .iter()
        .find(|page| {
            page.kind.eq_ignore_ascii_case("PAGE")
                && (page.path.trim_matches('/') == normalized_path
                    || page.slug == normalized_path
                    || page.id == page_path)
        })
        .cloned()
        .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;
    Ok(Json(page.into()))
}

/// List public pages handler
pub async fn list_public_pages_handler(
    State(state): State<Arc<AppState>>,
    Path((org_slug, project_slug)): Path<(String, String)>,
) -> Result<Json<Vec<PageResponse>>, AppError> {
    let site = resolve_public_site_by_slugs(&state, &org_slug, &project_slug, None, None).await?;
    let pages = site
        .pages
        .into_iter()
        .filter(|page| page.kind.eq_ignore_ascii_case("PAGE"))
        .map(Into::into)
        .collect();
    Ok(Json(pages))
}

/// Search public content handler
pub async fn search_public_content_handler(
    State(state): State<Arc<AppState>>,
    Path((org_slug, project_slug)): Path<(String, String)>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<Vec<PageResponse>>, AppError> {
    let site = resolve_public_site_by_slugs(
        &state,
        &org_slug,
        &project_slug,
        query.get("lang").and_then(serde_json::Value::as_str),
        query.get("version").and_then(serde_json::Value::as_str),
    )
    .await?;
    let search_term = query
        .get("q")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .trim();
    if search_term.len() > 256 {
        return Err(AppError::Validation("Search query is too long".to_string()));
    }
    if search_term.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let needle = search_term.to_lowercase();
    let mut results = site
        .pages
        .into_iter()
        .filter(|page| page.kind.eq_ignore_ascii_case("PAGE") && page.is_indexed)
        .filter_map(|page| {
            let title = page.title.to_lowercase();
            let slug = page.slug.to_lowercase();
            let content = page.content.to_lowercase();
            let description = page
                .description
                .as_deref()
                .unwrap_or_default()
                .to_lowercase();
            let score = if title == needle {
                0
            } else if title.starts_with(&needle) {
                1
            } else if title.contains(&needle) || slug.contains(&needle) {
                2
            } else if description.contains(&needle) {
                3
            } else if content.contains(&needle) {
                4
            } else {
                return None;
            };
            Some((score, page.path.clone(), page))
        })
        .collect::<Vec<_>>();
    results.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    let pages = results
        .into_iter()
        .take(50)
        .map(|(_, _, page)| page.into())
        .collect();
    Ok(Json(pages))
}

/// Get project sitemap handler
pub async fn get_project_sitemap_handler(
    State(state): State<Arc<AppState>>,
    Path((org_slug, project_slug)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let site = resolve_public_site_by_slugs(&state, &org_slug, &project_slug, None, None).await?;
    let urls = site
        .pages
        .iter()
        .filter(|page| page.kind.eq_ignore_ascii_case("PAGE"))
        .map(|page| {
            serde_json::json!({
                "loc": format!("/{}/{}{}", org_slug, project_slug, page.path),
                "lastmod": page.updated_at.to_rfc3339(),
                "title": page.title,
            })
        })
        .collect::<Vec<_>>();
    Ok(Json(serde_json::json!({ "urls": urls })))
}

/// Get public instance metadata handler
pub async fn get_public_meta_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Report which OAuth login providers the operator has configured so the SPA
    // shows the correct sign-in buttons.
    let oauth = state.config.auth.oauth.as_ref();
    let github = oauth.and_then(|o| o.github.as_ref()).is_some();
    let google = oauth.and_then(|o| o.google.as_ref()).is_some();

    Ok(Json(serde_json::json!({
        "providers": {
            "google": google,
            "github": github
        },
        "marketingAnalytics": null
    })))
}

/// Return all pages in depth-first tree order (position-sorted per parent level),
/// mirroring the editor sidebar's `flatten()` traversal so that `build_nav` and
/// `page_nav` both reflect the author-specified ordering.
fn pages_in_tree_order(pages: &[cms_entity::page::Page]) -> Vec<&cms_entity::page::Page> {
    use std::collections::HashMap;

    // Group page indices by parent_id, sorted by position within each group.
    let mut by_parent: HashMap<Option<&str>, Vec<usize>> = HashMap::new();
    for (i, p) in pages.iter().enumerate() {
        by_parent
            .entry(p.parent_id.as_deref())
            .or_default()
            .push(i);
    }
    for indices in by_parent.values_mut() {
        indices.sort_by_key(|&i| pages[i].position);
    }

    let mut out = Vec::with_capacity(pages.len());
    fn walk<'a>(
        pages: &'a [cms_entity::page::Page],
        by_parent: &HashMap<Option<&str>, Vec<usize>>,
        parent: Option<&str>,
        depth: usize,
        out: &mut Vec<&'a cms_entity::page::Page>,
    ) {
        if depth >= 128 {
            return;
        }
        if let Some(indices) = by_parent.get(&parent) {
            for &i in indices {
                out.push(&pages[i]);
                walk(pages, by_parent, Some(pages[i].id.as_str()), depth + 1, out);
            }
        }
    }
    walk(pages, &by_parent, None, 0, &mut out);
    out
}

/// Build a hierarchical nav tree from a flat list of pages.
///
/// Pages whose `parent_id` points at another page become children; pages with no
/// parent (or a parent not present in the set) become top-level nodes. Nodes that
/// have children are marked as `GROUP`, leaves as `PAGE`, and ordering follows the
/// page `position` within each level (DFS, matching the editor sidebar order).
fn build_nav(pages: &[cms_entity::page::Page]) -> Vec<serde_json::Value> {
    use std::collections::HashMap;

    // Group page indices by parent_id, sorted by position within each group.
    let mut by_parent: HashMap<Option<&str>, Vec<usize>> = HashMap::new();
    for (i, p) in pages.iter().enumerate() {
        by_parent
            .entry(p.parent_id.as_deref())
            .or_default()
            .push(i);
    }
    for indices in by_parent.values_mut() {
        indices.sort_by_key(|&i| pages[i].position);
    }

    // Recursively materialize nodes.
    fn build(
        pages: &[cms_entity::page::Page],
        by_parent: &HashMap<Option<&str>, Vec<usize>>,
        parent: Option<&str>,
        depth: usize,
    ) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        if depth >= 128 {
            return out;
        }
        if let Some(indices) = by_parent.get(&parent) {
            for &i in indices {
                let p = &pages[i];
                let kid = build(pages, by_parent, Some(p.id.as_str()), depth + 1);
                let kind = if p.kind == "GROUP" || !kid.is_empty() {
                    "GROUP"
                } else {
                    "PAGE"
                };
                out.push(serde_json::json!({
                    "id": p.id,
                    "kind": kind,
                    "title": p.title,
                    "path": p.path.trim_matches('/'),
                    "icon": p.icon.clone(),
                    "tag": null,
                    "children": kid,
                }));
            }
        }
        out
    }

    build(pages, &by_parent, None, 0)
}

/// Extract a table-of-contents from a Markdown page: heading text + an anchor id.
fn headings_from_markdown(content: &str) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix('#') {
            // Only stack up to h6.
            let level = trimmed.bytes().take_while(|b| *b == b'#').count();
            if !(1..=6).contains(&level) {
                continue;
            }
            let text = rest.trim();
            if text.is_empty() {
                continue;
            }
            let id = format!("h{}-{}", level, idx);
            out.push(serde_json::json!({
                "level": level,
                "text": text,
                "id": id,
            }));
        }
    }
    out
}

/// Build `prev`/`next` navigation for a page given the project's ordered page list.
///
/// Pages are traversed in depth-first tree order (position-sorted per parent level),
/// matching the editor sidebar and the published `build_nav` ordering so that
/// prev/next arrows follow the same sequence the reader sees in the sidebar.
fn page_nav(
    pages: &[cms_entity::page::Page],
    current_path: &str,
) -> (Option<serde_json::Value>, Option<serde_json::Value>) {
    // Walk the full tree (including GROUPs) in DFS order, then keep only PAGEs.
    let ordered: Vec<_> = pages_in_tree_order(pages)
        .into_iter()
        .filter(|p| p.kind.eq_ignore_ascii_case("PAGE"))
        .collect();
    let cur = ordered
        .iter()
        .position(|p| p.path.trim_matches('/') == current_path);
    let entry = |p: &cms_entity::page::Page| serde_json::json!({ "title": p.title, "path": p.path.trim_matches('/') });
    if let Some(i) = cur {
        let prev = i.checked_sub(1).map(|j| entry(ordered[j]));
        let next = (i + 1 < ordered.len()).then(|| entry(ordered[i + 1]));
        (prev, next)
    } else {
        (None, None)
    }
}

struct ResolvedPublicSite {
    project: cms_entity::project::Project,
    branch: cms_entity::branch::Branch,
    deployment_id: String,
    branches: Vec<cms_db::deployment::DeploymentBranchRelease>,
    languages: Vec<cms_entity::language::Language>,
    language: cms_entity::language::Language,
    pages: Vec<cms_entity::page::Page>,
    all_pages: Vec<cms_db::deployment::DeploymentSnapshotPageSummary>,
    language_config: Option<serde_json::Value>,
    primary_domain: Option<String>,
    version_number: u64,
}

async fn resolve_public_site(
    state: &AppState,
    project_id: &str,
    requested_language: Option<&str>,
    requested_version: Option<&str>,
) -> Result<ResolvedPublicSite, AppError> {
    // Visibility remains immediately revocable even though content itself is
    // frozen in a release snapshot.
    let live_project =
        cms_db::project::ProjectQueries::get_by_id(&state.biz_context.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Site not found".to_string()))?;
    if !live_project.is_public
        || cms_db::project::ProjectQueries::is_taken_down(&state.biz_context.pool, project_id)
            .await?
            .unwrap_or(false)
    {
        return Err(AppError::NotFound("Site not found".to_string()));
    }

    let branches = cms_db::deployment::DeploymentQueries::get_latest_branch_releases(
        &state.biz_context.pool,
        project_id,
    )
    .await?;
    if branches.is_empty() {
        return Err(AppError::NotFound(
            "Site has no published release".to_string(),
        ));
    }
    let requested_release = requested_version.and_then(|candidate| {
        branches.iter().find(|branch| {
            branch.branch_slug.eq_ignore_ascii_case(candidate)
                || branch.branch_id.eq_ignore_ascii_case(candidate)
        })
    });
    let branch_release = requested_release
        .or_else(|| branches.iter().find(|branch| branch.is_default))
        .ok_or_else(|| {
            AppError::NotFound("Site has no published default-branch release".to_string())
        })?;

    let saved_snapshot = cms_db::deployment::DeploymentQueries::get_snapshot_by_deployment(
        &state.biz_context.pool,
        &branch_release.deployment_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound("Published release not found".to_string()))?;
    let deployment_id = saved_snapshot.deployment_id.clone();
    let snapshot: cms_entity::deployment::DeploymentSnapshotContent =
        serde_json::from_value(saved_snapshot.snapshot).map_err(|error| {
            AppError::Internal(anyhow::anyhow!("Published snapshot is invalid: {error}"))
        })?;
    if snapshot.project.id != project_id || snapshot.branch.id != branch_release.branch_id {
        return Err(AppError::Conflict(
            "Published snapshot scope does not match the requested site".to_string(),
        ));
    }

    let languages = snapshot
        .languages
        .into_iter()
        .filter(|language| language.enabled)
        .collect::<Vec<_>>();
    let default_language = languages
        .iter()
        .find(|language| language.is_default)
        .or_else(|| languages.first())
        .cloned()
        .ok_or_else(|| {
            AppError::NotFound("Published release has no enabled language".to_string())
        })?;
    let language = requested_language
        .and_then(|code| {
            languages
                .iter()
                .find(|language| language.code.eq_ignore_ascii_case(code))
                .cloned()
        })
        .unwrap_or(default_language);
    let all_pages = cms_db::deployment::DeploymentQueries::get_snapshot_page_summaries(
        &state.biz_context.pool,
        &deployment_id,
    )
    .await?;
    let pages = cms_db::deployment::DeploymentQueries::get_snapshot_pages(
        &state.biz_context.pool,
        &deployment_id,
        Some(&language.id),
    )
    .await?;

    let translation = snapshot
        .translations
        .into_iter()
        .find(|translation| translation.language_id == language.id);
    let mut language_config = language.config.clone().filter(serde_json::Value::is_object);
    if let Some(translation) = translation {
        if translation.name.is_some() || translation.description.is_some() {
            let config = language_config.get_or_insert_with(|| serde_json::json!({}));
            if let Some(config) = config.as_object_mut() {
                if let Some(name) = translation.name {
                    config.insert("name".to_string(), serde_json::Value::String(name));
                }
                if let Some(description) = translation.description {
                    config.insert(
                        "description".to_string(),
                        serde_json::Value::String(description),
                    );
                }
            }
        }
    }
    if language_config
        .as_ref()
        .and_then(serde_json::Value::as_object)
        .is_some_and(serde_json::Map::is_empty)
    {
        language_config = None;
    }

    let primary_domain = cms_db::domain::DomainQueries::get_primary_for_project_branch(
        &state.biz_context.pool,
        project_id,
        &snapshot.branch.id,
    )
    .await?
    .map(|domain| domain.hostname);

    Ok(ResolvedPublicSite {
        project: snapshot.project,
        branch: snapshot.branch,
        deployment_id,
        branches,
        languages,
        language,
        pages,
        all_pages,
        language_config,
        primary_domain,
        version_number: saved_snapshot.version.max(0) as u64,
    })
}

async fn resolve_public_site_by_slugs(
    state: &AppState,
    organization_slug: &str,
    project_slug: &str,
    requested_language: Option<&str>,
    requested_version: Option<&str>,
) -> Result<ResolvedPublicSite, AppError> {
    let public_project =
        ProjectService::get_public_project(&state.biz_context, organization_slug, project_slug)
            .await?;
    resolve_public_site(
        state,
        &public_project.project.id,
        requested_language,
        requested_version,
    )
    .await
}

fn public_versions(
    branches: &[cms_db::deployment::DeploymentBranchRelease],
) -> Vec<serde_json::Value> {
    branches
        .iter()
        .map(|branch| {
            serde_json::json!({
                "id": branch.branch_id,
                "name": branch.branch_name,
                "slug": branch.branch_slug,
                "isDefault": branch.is_default,
                "version": branch.version,
            })
        })
        .collect()
}

fn snapshot_language_alternates(
    site: &ResolvedPublicSite,
    current: &cms_entity::page::Page,
) -> Vec<serde_json::Value> {
    site.languages
        .iter()
        .filter_map(|language| {
            let alternate = site.all_pages.iter().find(|candidate| {
                if candidate.language_id.as_deref() != Some(language.id.as_str()) {
                    return false;
                }
                match current
                    .translation_key
                    .as_deref()
                    .filter(|key| !key.trim().is_empty())
                {
                    Some(key) => {
                        candidate.translation_key.as_deref() == Some(key)
                            || (candidate
                                .translation_key
                                .as_deref()
                                .is_none_or(str::is_empty)
                                && candidate.path == current.path)
                    }
                    None => candidate.path == current.path,
                }
            })?;
            Some(serde_json::json!({
                "code": language.code,
                "isDefault": language.is_default,
                "path": alternate.path.trim_matches('/'),
            }))
        })
        .collect()
}

fn page_breadcrumbs(
    pages: &[cms_entity::page::Page],
    current: &cms_entity::page::Page,
) -> Vec<serde_json::Value> {
    use std::collections::{HashMap, HashSet};
    let by_id = pages
        .iter()
        .map(|page| (page.id.as_str(), page))
        .collect::<HashMap<_, _>>();
    let mut chain = Vec::new();
    let mut seen = HashSet::new();
    let mut page = current;
    for _ in 0..128 {
        if !seen.insert(page.id.as_str()) {
            break;
        }
        chain.push(serde_json::json!({
            "title": page.title,
            "path": page.path.trim_matches('/'),
        }));
        let Some(parent_id) = page.parent_id.as_deref() else {
            break;
        };
        let Some(parent) = by_id.get(parent_id) else {
            break;
        };
        page = parent;
    }
    chain.reverse();
    chain
}

/// Get public site shell
pub async fn get_public_site_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let requested_language = query.get("lang").and_then(serde_json::Value::as_str);
    let requested_version = query.get("version").and_then(serde_json::Value::as_str);
    let site = resolve_public_site(&state, &id, requested_language, requested_version).await?;
    let nav = build_nav(&site.pages);
    let languages: Vec<serde_json::Value> = site
        .languages
        .iter()
        .map(|language| {
            serde_json::json!({
                "code": language.code,
                "label": language.name,
                "direction": if language.is_rtl { "RTL" } else { "LTR" },
                "isDefault": language.is_default,
                "enabled": true,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "data": {
            "project": {
                "id": site.project.id,
                "name": site.project.name,
                "slug": site.project.slug,
                "description": site.project.description,
                "config": site.project.config,
                "primaryDomain": site.primary_domain,
            },
            "nav": nav,
            "languages": languages,
            "versions": public_versions(&site.branches),
            "activeLanguage": site.language.code,
            "activeVersion": site.branch.slug,
            "languageConfig": site.language_config,
            "version": site.version_number,
            "generatedAt": chrono::Utc::now().to_rfc3339(),
            "openapi": null
        }
    })))
}

/// Get public site page
pub async fn get_public_site_page_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let raw_path = query
        .get("path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .trim_matches('/');
    let requested_language = query.get("lang").and_then(serde_json::Value::as_str);
    let requested_version = query.get("version").and_then(serde_json::Value::as_str);
    let site = resolve_public_site(&state, &id, requested_language, requested_version).await?;

    // Non-default versions are mounted beneath their branch slug. Only strip the
    // prefix after resolving it to a real branch; an invalid candidate falls back
    // to the default branch and remains part of the page path.
    let path = if !site.branch.is_default
        && (raw_path == site.branch.slug || raw_path.starts_with(&format!("{}/", site.branch.slug)))
    {
        raw_path
            .strip_prefix(&site.branch.slug)
            .unwrap_or_default()
            .trim_matches('/')
    } else {
        raw_path
    };

    let page = if path.is_empty() {
        site.pages
            .iter()
            .filter(|page| page.kind.eq_ignore_ascii_case("PAGE"))
            .min_by(|left, right| {
                left.position
                    .cmp(&right.position)
                    .then(left.path.cmp(&right.path))
            })
            .cloned()
    } else {
        site.pages
            .iter()
            .find(|page| page.path.trim_matches('/') == path)
            .cloned()
            .map(|candidate| {
                if candidate.kind.eq_ignore_ascii_case("GROUP") {
                    let prefix = format!("{}/", candidate.path.trim_matches('/'));
                    site.pages
                        .iter()
                        .filter(|child| {
                            child.kind.eq_ignore_ascii_case("PAGE")
                                && child.path.trim_matches('/').starts_with(&prefix)
                        })
                        .min_by(|left, right| {
                            left.position
                                .cmp(&right.position)
                                .then(left.path.cmp(&right.path))
                        })
                        .cloned()
                        .unwrap_or(candidate)
                } else {
                    candidate
                }
            })
    }
    .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

    let clean_path = page.path.trim_matches('/').to_string();
    let (prev, next) = page_nav(&site.pages, &clean_path);
    let lang_alternates = snapshot_language_alternates(&site, &page);
    let breadcrumbs = page_breadcrumbs(&site.pages, &page);
    let headings = headings_from_markdown(&page.content);

    Ok(Json(serde_json::json!({
        "data": {
            "project": {
                "id": site.project.id,
                "name": site.project.name,
                "slug": site.project.slug,
                "description": site.project.description,
                "config": site.project.config,
                "primaryDomain": site.primary_domain,
            },
            "page": {
                "id": page.id,
                "createdAt": page.created_at.to_rfc3339(),
                "updatedAt": page.updated_at.to_rfc3339(),
                "title": page.title,
                "description": page.description.unwrap_or_default(),
                "icon": page.icon,
                "path": clean_path,
                "content": page.content,
                "headings": headings,
                "config": page.config
            },
            "activeLanguage": site.language.code,
            "activeVersion": site.branch.slug,
            "version": site.version_number,
            "versions": public_versions(&site.branches),
            "languageConfig": site.language_config,
            "languages": lang_alternates,
            "breadcrumbs": breadcrumbs,
            "prev": prev,
            "next": next
        }
    })))
}

/// Get a public page by id
///
/// Resolves a specific published page by its id and returns it in the SPA page
/// shape (used for direct page/direct link navigation and cross-site fetching).
pub async fn get_public_pages_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let saved_snapshot = cms_db::deployment::DeploymentQueries::get_latest_snapshot_for_page(
        &state.biz_context.pool,
        &id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;
    let deployment_id = saved_snapshot.deployment_id.clone();
    let snapshot: cms_entity::deployment::DeploymentSnapshotContent =
        serde_json::from_value(saved_snapshot.snapshot).map_err(|error| {
            AppError::Internal(anyhow::anyhow!("Published snapshot is invalid: {error}"))
        })?;
    let requested_page = cms_db::deployment::DeploymentQueries::get_snapshot_page(
        &state.biz_context.pool,
        &deployment_id,
        &id,
    )
    .await?
    .filter(|page| page.is_published)
    .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;
    let language_code = requested_page
        .language_id
        .as_deref()
        .and_then(|language_id| {
            snapshot
                .languages
                .iter()
                .find(|language| language.id == language_id)
        })
        .or_else(|| {
            snapshot
                .languages
                .iter()
                .find(|language| language.is_default)
        })
        .map(|language| language.code.clone())
        .ok_or_else(|| AppError::NotFound("Published language not found".to_string()))?;
    let site = resolve_public_site(
        &state,
        &snapshot.project.id,
        Some(&language_code),
        Some(&snapshot.branch.id),
    )
    .await?;
    let page = site
        .pages
        .iter()
        .find(|page| page.id == requested_page.id)
        .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;
    let clean_path = page.path.trim_matches('/').to_string();
    let (prev, next) = page_nav(&site.pages, &clean_path);
    let languages = snapshot_language_alternates(&site, page);
    let breadcrumbs = page_breadcrumbs(&site.pages, page);
    let headings = headings_from_markdown(&page.content);

    Ok(Json(serde_json::json!({
        "data": {
            "project": {
                "id": site.project.id,
                "name": site.project.name,
                "slug": site.project.slug,
                "description": site.project.description,
                "config": site.project.config,
                "primaryDomain": site.primary_domain,
            },
            "page": {
                "id": page.id,
                "createdAt": page.created_at.to_rfc3339(),
                "updatedAt": page.updated_at.to_rfc3339(),
                "title": page.title,
                "description": page.description.clone().unwrap_or_default(),
                "icon": page.icon,
                "path": clean_path,
                "content": page.content,
                "headings": headings,
                "config": page.config,
            },
            "activeLanguage": site.language.code,
            "activeVersion": site.branch.slug,
            "version": site.version_number,
            "versions": public_versions(&site.branches),
            "languageConfig": site.language_config,
            "languages": languages,
            "breadcrumbs": breadcrumbs,
            "prev": prev,
            "next": next,
        }
    })))
}

/// Get public site changelog
///
/// Returns the site's releases (deployments) newest-first as SPA change entries,
/// backed by the real Deployment rows for the project.
pub async fn get_public_site_changelog_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let project = cms_db::project::ProjectQueries::get_by_id(&state.biz_context.pool, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Site not found".to_string()))?;
    if !project.is_public {
        return Err(AppError::NotFound("Site not found".to_string()));
    }

    let releases = cms_db::deployment::DeploymentQueries::get_releases_by_project(
        &state.biz_context.pool,
        &id,
        50,
    )
    .await?;

    let items: Vec<serde_json::Value> = releases
        .iter()
        .map(|release| {
            let summary = release.build_logs.clone().unwrap_or_else(|| {
                format!("Published {} v{}", release.branch_name, release.version)
            });
            serde_json::json!({
                "id": release.deployment_id,
                "slug": format!("{}-v{}", release.branch_slug, release.version),
                "title": summary.lines().next().unwrap_or("Site update"),
                "summary": summary,
                "date": release.created_at.to_rfc3339(),
                "version": release.version,
                "branch": release.branch_slug,
                "changes": [],
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "data": items })))
}

/// Track public site events
///
/// Records a real analytics event (e.g. page_view) against the project so the
/// site's public traffic appears in the dashboard.
pub async fn post_public_site_events_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(event): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_biz::analytics::AnalyticsService;

    let project = cms_db::project::ProjectQueries::get_by_id(&state.biz_context.pool, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Site not found".to_string()))?;
    if !project.is_public {
        return Err(AppError::NotFound("Site not found".to_string()));
    }

    let event_type = event
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("page_view")
        .trim();
    if event_type.is_empty()
        || event_type.len() > 64
        || !event_type.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
    {
        return Err(AppError::Validation(
            "Invalid analytics event type".to_string(),
        ));
    }
    let metadata = event
        .get("metadata")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    if !metadata.is_object() || serde_json::to_vec(&metadata)?.len() > 8 * 1024 {
        return Err(AppError::Validation(
            "Analytics metadata must be an object no larger than 8 KiB".to_string(),
        ));
    }
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok());

    AnalyticsService::record_event(
        &state.biz_context,
        None,
        Some(&id),
        None,
        event_type,
        metadata,
        None,
        user_agent,
    )
    .await?;

    Ok(Json(serde_json::json!({ "data": { "success": true } })))
}

/// Search public site
///
/// Searches only the selected immutable release and language, then returns
/// bounded reader-facing snippets from the captured page bodies.
pub async fn search_public_site_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let site = resolve_public_site(
        &state,
        &id,
        query.get("lang").and_then(serde_json::Value::as_str),
        query.get("version").and_then(serde_json::Value::as_str),
    )
    .await?;
    let q = query
        .get("q")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .trim();
    if q.len() > 256 {
        return Err(AppError::Validation("Search query is too long".to_string()));
    }
    if q.is_empty() {
        return Ok(Json(serde_json::json!({ "data": { "hits": [] } })));
    }
    let limit = query
        .get("limit")
        .and_then(|value| value.as_i64().or_else(|| value.as_str()?.parse().ok()))
        .unwrap_or(10)
        .clamp(1, 50) as usize;
    let needle = q.to_lowercase();

    let mut results = site
        .pages
        .iter()
        .filter(|page| page.kind.eq_ignore_ascii_case("PAGE") && page.is_indexed)
        .filter_map(|page| {
            let title = page.title.to_lowercase();
            let slug = page.slug.to_lowercase();
            let path = page.path.to_lowercase();
            let description = page
                .description
                .as_deref()
                .unwrap_or_default()
                .to_lowercase();
            let content = page.content.to_lowercase();
            let score: f64 = if title == needle {
                1.0
            } else if title.starts_with(&needle) {
                0.9
            } else if title.contains(&needle) || slug.contains(&needle) {
                0.8
            } else if path.contains(&needle) {
                0.75
            } else if description.contains(&needle) {
                0.7
            } else if content.contains(&needle) {
                0.6
            } else {
                return None;
            };
            let snippet = page
                .content
                .lines()
                .find(|line| line.to_lowercase().contains(&needle))
                .unwrap_or(&page.title)
                .chars()
                .take(280)
                .collect::<String>();
            Some((score, page.path.clone(), page, snippet))
        })
        .collect::<Vec<_>>();
    results.sort_by(|left, right| right.0.total_cmp(&left.0).then(left.1.cmp(&right.1)));
    let hits = results
        .into_iter()
        .take(limit)
        .map(|(score, _, page, snippet)| {
            serde_json::json!({
                "id": page.id,
                "title": page.title,
                "path": page.path,
                "snippet": snippet,
                "score": score,
                "heading": null,
                "icon": page.icon,
                "direction": if site.language.is_rtl { "RTL" } else { "LTR" },
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(serde_json::json!({ "data": { "hits": hits } })))
}

/// Answer from published public site content
///
/// Returns an extractive, snapshot-grounded answer and matching source pages.
/// It deliberately avoids project-wide indexes that may contain draft or newer
/// editor content.
pub async fn answer_public_site_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let question = body
        .get("question")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|question| !question.is_empty())
        .ok_or_else(|| AppError::InvalidInput("question is required".to_string()))?;
    if question.len() > 2_000 {
        return Err(AppError::Validation("Question is too long".to_string()));
    }
    let requested_language = body
        .get("lang")
        .or_else(|| body.get("language"))
        .and_then(serde_json::Value::as_str);
    let requested_version = body.get("version").and_then(serde_json::Value::as_str);
    let site = resolve_public_site(&state, &id, requested_language, requested_version).await?;
    let terms = question
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return Err(AppError::InvalidInput(
            "question contains no searchable terms".to_string(),
        ));
    }

    let mut matches = site
        .pages
        .iter()
        .filter(|page| page.kind.eq_ignore_ascii_case("PAGE") && page.is_indexed)
        .filter_map(|page| {
            let title = page.title.to_lowercase();
            let description = page
                .description
                .as_deref()
                .unwrap_or_default()
                .to_lowercase();
            let content = page.content.to_lowercase();
            let matched = terms
                .iter()
                .filter(|term| {
                    title.contains(term.as_str())
                        || description.contains(term.as_str())
                        || content.contains(term.as_str())
                })
                .count();
            if matched == 0 {
                return None;
            }
            let best_line = page
                .content
                .lines()
                .max_by_key(|line| {
                    let lower = line.to_lowercase();
                    terms
                        .iter()
                        .filter(|term| lower.contains(term.as_str()))
                        .count()
                })
                .unwrap_or(&page.title);
            let snippet = best_line.chars().take(400).collect::<String>();
            let score = matched as f64 / terms.len() as f64;
            Some((matched, page.path.clone(), page, snippet, score))
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));
    let selected = matches.into_iter().take(3).collect::<Vec<_>>();
    let sources = selected
        .iter()
        .map(|(_, _, page, snippet, score)| {
            serde_json::json!({
                "id": page.id,
                "title": page.title,
                "path": page.path,
                "snippet": snippet,
                "score": score,
                "heading": null,
                "icon": page.icon,
                "direction": if site.language.is_rtl { "RTL" } else { "LTR" },
            })
        })
        .collect::<Vec<_>>();
    let answer = if selected.is_empty() {
        "I couldn't find relevant information in the published documentation.".to_string()
    } else {
        let excerpts = selected
            .iter()
            .map(|(_, _, page, snippet, _)| format!("- {}: {}", page.title, snippet))
            .collect::<Vec<_>>()
            .join("\n");
        format!("Relevant published documentation:\n{excerpts}")
    };
    let confidence = selected
        .first()
        .map(|(_, _, _, _, score)| *score)
        .unwrap_or(0.0);

    Ok(Json(serde_json::json!({
        "data": {
            "answer": answer,
            "sources": sources,
            "confidence": confidence,
            "mode": "extractive",
            "version": site.version_number,
        }
    })))
}

/// Track public marketing events
pub async fn post_public_marketing_events_handler(
    State(_state): State<Arc<AppState>>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({ "data": { "success": true } })))
}

/// Get public invitation
///
/// Resolves a pending member invitation by id/token and returns it (with the
/// owning organization name) so the SPA can render the accept-invitation screen.
pub async fn get_public_invitation_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    use cms_db::org::{InvitationQueries, OrganizationQueries};

    // Prefer a token match; if absent, look the row up by id for invitation URLs
    // issued by the platform admin and legacy client flows.
    let row = match InvitationQueries::get_by_token(&state.biz_context.pool, &id).await? {
        Some(invitation) => Some(invitation),
        None => InvitationQueries::get_by_id(&state.biz_context.pool, &id).await?,
    };

    let org_name = match &row {
        Some(inv) => OrganizationQueries::get_by_id(&state.biz_context.pool, &inv.organization_id)
            .await?
            .map(|o| o.name)
            .unwrap_or_else(|| "cms Workspace".to_string()),
        None => "cms Workspace".to_string(),
    };

    let inviter_name = "Admin".to_string();
    let data = match row {
        Some(inv) => serde_json::json!({
            "id": inv.id,
            "organizationName": org_name,
            "inviterName": inviter_name,
            "email": inv.email,
            "role": format!("{:?}", inv.role).to_lowercase(),
            "expiresAt": inv.expires_at.to_rfc3339(),
        }),
        None => serde_json::json!(null),
    };

    Ok(Json(serde_json::json!({ "data": data })))
}

/// Get public git preview
///
/// Resolves a preview deployment by its share token. Previews are tied to git
/// pull-request deployments; when the token matches a stored preview the site
/// shell is returned. Falls back to `null` when the preview cannot be found.
pub async fn get_public_git_preview_handler(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // A preview token is the opaque GitPreview ID, not the deployment ID. Do
    // not allow guessing a deployment's public identifier to read its content.
    let preview =
        cms_db::git::GitPreviewQueries::get_by_id(&state.biz_context.pool, &token).await?;
    let data = match preview {
        Some(preview) => match preview.deployment_id.as_deref() {
            Some(deployment_id) => {
                let deployment = cms_db::deployment::DeploymentQueries::get_by_id(
                    &state.biz_context.pool,
                    deployment_id,
                )
                .await?;
                match deployment {
                    Some(deployment) => {
                        let has_snapshot =
                            cms_db::deployment::DeploymentQueries::get_snapshot_by_deployment(
                                &state.biz_context.pool,
                                deployment_id,
                            )
                            .await?
                            .is_some();
                        if !has_snapshot {
                            serde_json::Value::Null
                        } else {
                            let project = cms_db::project::ProjectQueries::get_by_id(
                                &state.biz_context.pool,
                                &deployment.project_id,
                            )
                            .await?
                            .ok_or_else(|| AppError::NotFound("Site not found".to_string()))?;
                            serde_json::json!({
                                "project": { "id": project.id, "name": project.name, "slug": project.slug },
                                "deployment": { "id": deployment.id, "status": format!("{:?}", deployment.status).to_lowercase() },
                                "token": preview.id,
                            })
                        }
                    }
                    None => serde_json::Value::Null,
                }
            }
            None => serde_json::Value::Null,
        },
        None => serde_json::Value::Null,
    };

    Ok(Json(serde_json::json!({ "data": data })))
}

#[derive(Debug, serde::Deserialize)]
pub struct TlsAuthorizationQuery {
    pub domain: String,
}

/// Caddy on-demand TLS authorization callback. This is a fail-closed gate:
/// a verified domain must point at an active, public, non-taken-down release.
///
/// Configure Caddy to call `/api/public/domains/tls-authorize?domain={host}`;
/// CMS never handles ACME challenges or stores private keys. Caddy owns
/// certificate issuance, renewal, and storage.
pub async fn authorize_custom_domain_tls_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TlsAuthorizationQuery>,
) -> Result<StatusCode, AppError> {
    let pool = &state.biz_context.pool;
    let domain = cms_db::domain::DomainQueries::get_verified_by_hostname(pool, &query.domain)
        .await?
        .ok_or_else(|| AppError::NotFound("TLS authorization denied".to_string()))?;
    let deployment = cms_db::deployment::DeploymentQueries::get_by_id(pool, &domain.deployment_id)
        .await?
        .filter(|deployment| {
            matches!(
                deployment.status,
                cms_entity::deployment::DeploymentStatus::Active
            )
        })
        .ok_or_else(|| AppError::NotFound("TLS authorization denied".to_string()))?;
    let project = cms_db::project::ProjectQueries::get_by_id(pool, &deployment.project_id)
        .await?
        .filter(|project| project.is_public)
        .ok_or_else(|| AppError::NotFound("TLS authorization denied".to_string()))?;
    if cms_db::project::ProjectQueries::is_taken_down(pool, &project.id)
        .await?
        .unwrap_or(true)
    {
        return Err(AppError::NotFound("TLS authorization denied".to_string()));
    }
    if cms_db::deployment::DeploymentQueries::get_snapshot_by_deployment(pool, &deployment.id)
        .await?
        .is_none()
    {
        return Err(AppError::NotFound("TLS authorization denied".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Resolve public custom domain
///
/// Maps a custom hostname to its project (via the Domain → Deployment chain) so a
/// site can be served from a bound domain. Returns `null` when the host is unknown.
pub async fn get_public_domains_resolve_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let host = query
        .get("host")
        .and_then(|v| v.as_str())
        .or_else(|| query.get("hostname").and_then(|v| v.as_str()))
        .map(|s| s.trim().trim_end_matches('.').to_lowercase())
        .filter(|s| !s.is_empty());

    let Some(host) = host else {
        return Ok(Json(serde_json::json!({ "data": null })));
    };

    let domain =
        cms_db::domain::DomainQueries::get_verified_by_hostname(&state.biz_context.pool, &host)
            .await?;
    let data = match domain {
        Some(domain) => {
            let deployment = cms_db::deployment::DeploymentQueries::get_by_id(
                &state.biz_context.pool,
                &domain.deployment_id,
            )
            .await?;
            match deployment {
                Some(deployment) => {
                    let project = cms_db::project::ProjectQueries::get_by_id(
                        &state.biz_context.pool,
                        &deployment.project_id,
                    )
                    .await?;
                    let snapshot =
                        cms_db::deployment::DeploymentQueries::get_snapshot_by_deployment(
                            &state.biz_context.pool,
                            &deployment.id,
                        )
                        .await?;
                    let taken_down = cms_db::project::ProjectQueries::is_taken_down(
                        &state.biz_context.pool,
                        &deployment.project_id,
                    )
                    .await?
                    .unwrap_or(true);
                    match (
                        project.filter(|project| project.is_public),
                        snapshot,
                        taken_down,
                    ) {
                        (Some(project), Some(_), false) => serde_json::json!({
                            "projectId": project.id,
                            "domain": domain.hostname,
                            "isPrimary": domain.is_primary,
                            "verified": domain.verified_at.is_some(),
                        }),
                        _ => serde_json::Value::Null,
                    }
                }
                None => serde_json::Value::Null,
            }
        }
        None => serde_json::Value::Null,
    };

    Ok(Json(serde_json::json!({ "data": data })))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Find a project's published pages plus its OpenAPI doc for the machine endpoints.
async fn site_pages(
    state: &Arc<AppState>,
    id: &str,
) -> Result<Vec<cms_entity::page::Page>, AppError> {
    let site = resolve_public_site(state, id, None, None).await?;
    cms_db::deployment::DeploymentQueries::get_snapshot_pages(
        &state.biz_context.pool,
        &site.deployment_id,
        Some(&site.language.id),
    )
    .await
    .map(|pages| {
        pages
            .into_iter()
            .filter(|page| page.kind.eq_ignore_ascii_case("PAGE"))
            .collect()
    })
}

/// `GET /sites/:id/markdown` — concatenated Markdown of all the site's pages.
pub async fn get_public_site_markdown_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let pages = site_pages(&state, &id).await?;
    let mut out = String::new();
    for p in pages {
        out.push_str(&format!("# {}\n\n{}\n\n", p.title, p.content));
    }
    Ok(([(axum::http::header::CONTENT_TYPE, "text/markdown")], out))
}

/// `GET /sites/:id/openapi.json` — the site's stored OpenAPI reference document.
pub async fn get_public_site_openapi_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let site = resolve_public_site(&state, &id, None, None).await?;
    let snapshot = cms_db::deployment::DeploymentQueries::get_snapshot_by_deployment(
        &state.biz_context.pool,
        &site.deployment_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound("Published release not found".to_string()))?;
    let body = snapshot
        .snapshot
        .get("openapi")
        .and_then(serde_json::Value::as_str)
        .filter(|content| !content.trim().is_empty())
        .unwrap_or("{}")
        .to_string();
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body,
    ))
}

/// `GET /sites/:id/changelog/rss.xml` — RSS feed of the site's deployments.
pub async fn get_public_site_changelog_rss_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    resolve_public_site(&state, &id, None, None).await?;
    let releases = cms_db::deployment::DeploymentQueries::get_releases_by_project(
        &state.biz_context.pool,
        &id,
        50,
    )
    .await?;

    let mut items = String::new();
    for release in releases {
        let title = xml_escape(
            release
                .build_logs
                .as_deref()
                .and_then(|logs| logs.lines().next())
                .unwrap_or("Site update"),
        );
        let date = release.created_at.to_rfc3339();
        items.push_str(&format!(
            "<item><title>{}</title><pubDate>{}</pubDate><guid>{}</guid></item>",
            title,
            date,
            xml_escape(&release.deployment_id)
        ));
    }

    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss version=\"2.0\"><channel><title>Site changelog</title>{}</channel></rss>",
        items
    );
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/rss+xml")],
        body,
    ))
}

/// `GET /sites/:id/sitemap.xml` — XML sitemap of the site's pages.
pub async fn get_public_site_sitemap_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let pages = site_pages(&state, &id).await?;
    let mut urls = String::new();
    for p in pages {
        let loc = format!("https://{}.app/{}", p.project_id, p.path.trim_matches('/'));
        urls.push_str(&format!("<url><loc>{}</loc></url>", xml_escape(&loc)));
    }
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">{}</urlset>",
        urls
    );
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        body,
    ))
}

/// `GET /sites/:id/robots.txt` — site robots.txt.
pub async fn get_public_site_robots_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let pages = site_pages(&state, &id).await?;
    let mut sitemap = String::new();
    if let Some(p) = pages.first() {
        sitemap = format!("Sitemap: https://{}.app/sitemap.xml\n", p.project_id);
    }
    let body = format!("User-agent: *\nAllow: /\n{}", sitemap);
    Ok(([(axum::http::header::CONTENT_TYPE, "text/plain")], body))
}

/// `GET /sites/:id/llms.txt` — a concise index of the site's pages for LLMs.
pub async fn get_public_site_llms_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let pages = site_pages(&state, &id).await?;
    let mut out = String::from("# Site\n\n");
    for p in pages {
        out.push_str(&format!("- [{}](/{})\n", p.title, p.path.trim_matches('/')));
    }
    Ok(([(axum::http::header::CONTENT_TYPE, "text/plain")], out))
}

/// `GET /sites/:id/llms-full.txt` — the full concatenated content of all pages.
pub async fn get_public_site_llms_full_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let pages = site_pages(&state, &id).await?;
    let mut out = String::new();
    for p in pages {
        out.push_str(&format!("# {}\n\n{}\n\n", p.title, p.content));
    }
    Ok(([(axum::http::header::CONTENT_TYPE, "text/plain")], out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_page(
        id: &str,
        parent_id: Option<&str>,
        path: &str,
        title: &str,
        kind: &str,
        position: i32,
    ) -> cms_entity::page::Page {
        cms_entity::page::Page {
            id: id.to_string(),
            project_id: "test-proj".to_string(),
            branch_id: "test-branch".to_string(),
            language_id: None,
            parent_id: parent_id.map(str::to_string),
            kind: kind.to_string(),
            path: path.to_string(),
            slug: path.trim_start_matches('/').to_string(),
            title: title.to_string(),
            description: None,
            content: "content".to_string(),
            icon: None,
            config: None,
            translation_key: None,
            position,
            is_published: true,
            is_indexed: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_pages_in_tree_order_respects_position_over_alphabetical_path() {
        let pages = vec![
            make_page("c-advanced", None, "/c-advanced", "Advanced", "GROUP", 2),
            make_page("c-setup", Some("c-advanced"), "/c-advanced/setup", "Setup", "PAGE", 0),
            make_page("z-intro", None, "/z-intro", "Introduction", "PAGE", 0),
            make_page("a-guide", None, "/a-guide", "Guide", "PAGE", 1),
            make_page("a-sub2", Some("a-guide"), "/a-guide/sub-2", "Sub 2", "PAGE", 1),
            make_page("a-sub1", Some("a-guide"), "/a-guide/sub-1", "Sub 1", "PAGE", 0),
        ];

        let ordered = pages_in_tree_order(&pages);
        let ids: Vec<&str> = ordered.iter().map(|p| p.id.as_str()).collect();

        assert_eq!(
            ids,
            vec!["z-intro", "a-guide", "a-sub1", "a-sub2", "c-advanced", "c-setup"]
        );
    }

    #[test]
    fn test_build_nav_reflects_tree_position_order() {
        let pages = vec![
            make_page("p2", None, "/b-page", "B Page", "PAGE", 1),
            make_page("p1", None, "/a-page", "A Page", "PAGE", 0),
        ];

        let nav = build_nav(&pages);
        assert_eq!(nav.len(), 2);
        assert_eq!(nav[0]["id"], "p1");
        assert_eq!(nav[1]["id"], "p2");
    }

    #[test]
    fn test_page_nav_prev_next_matches_sidebar_order() {
        let pages = vec![
            make_page("c-advanced", None, "/c-advanced", "Advanced", "GROUP", 2),
            make_page("c-setup", Some("c-advanced"), "/c-advanced/setup", "Setup", "PAGE", 0),
            make_page("z-intro", None, "/z-intro", "Introduction", "PAGE", 0),
            make_page("a-guide", None, "/a-guide", "Guide", "PAGE", 1),
            make_page("a-sub2", Some("a-guide"), "/a-guide/sub-2", "Sub 2", "PAGE", 1),
            make_page("a-sub1", Some("a-guide"), "/a-guide/sub-1", "Sub 1", "PAGE", 0),
        ];

        // First page: /z-intro
        let (prev, next) = page_nav(&pages, "z-intro");
        assert_eq!(prev, None);
        assert_eq!(next.unwrap()["path"], "a-guide");

        // Middle page: /a-guide
        let (prev, next) = page_nav(&pages, "a-guide");
        assert_eq!(prev.unwrap()["path"], "z-intro");
        assert_eq!(next.unwrap()["path"], "a-guide/sub-1");

        // Next page: /a-guide/sub-1
        let (prev, next) = page_nav(&pages, "a-guide/sub-1");
        assert_eq!(prev.unwrap()["path"], "a-guide");
        assert_eq!(next.unwrap()["path"], "a-guide/sub-2");

        // Next page: /a-guide/sub-2 -> skips GROUP /c-advanced -> goes to /c-advanced/setup
        let (prev, next) = page_nav(&pages, "a-guide/sub-2");
        assert_eq!(prev.unwrap()["path"], "a-guide/sub-1");
        assert_eq!(next.unwrap()["path"], "c-advanced/setup");

        // Last page: /c-advanced/setup
        let (prev, next) = page_nav(&pages, "c-advanced/setup");
        assert_eq!(prev.unwrap()["path"], "a-guide/sub-2");
        assert_eq!(next, None);
    }
}
