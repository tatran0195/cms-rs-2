//! Search Business Logic
//!
//! This module contains business logic for search operations,
//! including hybrid FTS+vector search with Japanese tokenization.

use std::sync::Arc;

use cms_db::{page::PageQueries, project::ProjectQueries, search_index::SearchIndexRunQueries};
use cms_entity::{
    common::{Id, MemberRole, PaginatedResponse},
    search::{
        IndexPageRequest, ListSearchIndexRunsQuery, RagAnswer, ReindexRequest, SearchIndexRun,
        SearchIndexRunResponse, SearchIndexRunStatus, SearchOptions, SearchRequest, SearchResponse,
        SearchResultItem,
    },
};

use crate::{AppError, BizContext};

/// Search service
pub struct SearchService;

impl SearchService {
    /// Search for pages in a project
    pub async fn search(
        ctx: &BizContext,
        search_engine: Arc<dyn cms_search::SearchEngine>,
        project_id: &str,
        request: SearchRequest,
    ) -> Result<SearchResponse, AppError> {
        // Verify project exists and the request cannot accidentally target a
        // different project than the authenticated handler scope.
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;
        if request.project_id != project_id {
            return Err(AppError::InvalidInput("Search project scope mismatch".to_string()));
        }

        let query = request.query.trim();
        if query.is_empty() {
            return Err(AppError::InvalidInput("Search query cannot be empty".to_string()));
        }
        if query.len() > 256 {
            return Err(AppError::InvalidInput("Search query is too long".to_string()));
        }
        let limit = request.limit.clamp(1, 100);
        let offset = request.offset.unwrap_or(0).clamp(0, 400);
        // The search engine interface does not yet support native branch or
        // language filters. Overfetch a bounded window, then authorize every
        // hit against the relational source before returning it.
        let fetch_limit = (limit + offset).clamp(limit, 500) as usize;
        let opts = SearchOptions {
            limit: fetch_limit,
            min_score: 0.0,
            fts_weight: 0.5,
        };

        let hits = search_engine.hybrid_query(project_id, query, opts).await?;
        let hit_ids = hits
            .iter()
            .map(|hit| hit.page_id.as_str())
            .collect::<Vec<_>>();
        let scoped_pages = PageQueries::get_searchable_by_ids_in_scope(
            &ctx.pool,
            &hit_ids,
            project_id,
            request.branch_id.as_deref(),
            request.language_id.as_deref(),
        )
        .await?;
        let allowed = scoped_pages
            .into_iter()
            .map(|page| (page.id.clone(), page))
            .collect::<std::collections::HashMap<_, _>>();
        let mut scoped_hits = hits
            .into_iter()
            .filter_map(|hit| {
                let page = allowed.get(&hit.page_id)?;
                Some(SearchResultItem {
                    page_id: hit.page_id,
                    project_id: project_id.to_string(),
                    title: page.title.clone(),
                    path: page.path.clone(),
                    score: hit.score,
                    chunk_text: hit.chunk_text,
                    chunk_index: hit.chunk_index,
                    metadata: hit.metadata,
                })
            })
            .skip(offset as usize)
            .take(limit as usize)
            .collect::<Vec<_>>();
        let total = scoped_hits.len() as i64;

        Ok(SearchResponse {
            query: query.to_string(),
            results: std::mem::take(&mut scoped_hits),
            total,
            limit,
            offset,
        })
    }

    /// Index a page for search
    pub async fn index_page(
        ctx: &BizContext,
        search_engine: Arc<dyn cms_search::SearchEngine>,
        request: IndexPageRequest,
    ) -> Result<(), AppError> {
        // Verify page exists
        let page = PageQueries::get_by_id(&ctx.pool, &request.page_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

        // Index the page
        search_engine.index_page(&page).await?;

        Ok(())
    }

    /// Remove a page from the search index
    pub async fn remove_page_from_index(
        ctx: &BizContext,
        search_engine: Arc<dyn cms_search::SearchEngine>,
        page_id: &str,
    ) -> Result<(), AppError> {
        // Verify page exists
        let _page = PageQueries::get_by_id(&ctx.pool, page_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

        // Remove from index
        search_engine.remove_page(page_id).await?;

        Ok(())
    }

    /// Get RAG answer for a question
    pub async fn get_rag_answer(
        ctx: &BizContext,
        search_engine: Arc<dyn cms_search::SearchEngine>,
        project_id: &str,
        question: &str,
    ) -> Result<RagAnswer, AppError> {
        // Verify project exists
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Get RAG answer
        search_engine.rag_answer(project_id, question).await
    }

    /// List search index runs
    pub async fn list_index_runs(
        ctx: &BizContext,
        user_id: &str,
        query: ListSearchIndexRunsQuery,
    ) -> Result<PaginatedResponse<SearchIndexRunResponse>, AppError> {
        let project_id = query.project_id.as_deref().ok_or_else(|| {
            AppError::InvalidInput("project_id query param is required".to_string())
        })?;

        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;

        let runs =
            SearchIndexRunQueries::get_by_project(&ctx.pool, project_id, query.limit, query.offset)
                .await?;

        let total = SearchIndexRunQueries::count_by_project(&ctx.pool, project_id).await?;

        let limit = query.limit.unwrap_or(20) as u64;
        let offset = query.offset.unwrap_or(0) as u64;
        let page = offset.checked_div(limit).map_or(1, |d| d + 1);

        Ok(PaginatedResponse::new(
            runs.into_iter().map(|r| r.into()).collect(),
            total as u64,
            page,
            limit,
        ))
    }

    /// Get a specific search index run
    pub async fn get_index_run(
        ctx: &BizContext,
        user_id: &str,
        run_id: &str,
    ) -> Result<SearchIndexRunResponse, AppError> {
        let run = SearchIndexRunQueries::get_by_id(&ctx.pool, run_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Search index run not found: {}", run_id)))?;

        ctx.authz
            .require_project_role(user_id, &run.project_id, MemberRole::Viewer)
            .await?;

        Ok(run.into())
    }

    /// Get search status for a project
    pub async fn get_search_status(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;

        let latest_run =
            SearchIndexRunQueries::get_latest_by_project(&ctx.pool, project_id).await?;
        let total_runs = SearchIndexRunQueries::count_by_project(&ctx.pool, project_id).await?;

        Ok(serde_json::json!({
            "status": "ready",
            "project_id": project_id,
            "total_index_runs": total_runs,
            "latest_run": latest_run.map(|r| serde_json::json!({
                "id": r.id,
                "status": format!("{:?}", r.status).to_lowercase(),
                "pages_indexed": r.pages_indexed,
                "completed_at": r.completed_at,
            })),
        }))
    }

    /// Reindex all pages for a project into the search engine
    pub async fn reindex(
        ctx: &BizContext,
        search_engine: Arc<dyn cms_search::SearchEngine>,
        user_id: &str,
        request: ReindexRequest,
    ) -> Result<SearchIndexRunResponse, AppError> {
        ctx.authz
            .require_project_role(user_id, &request.project_id, MemberRole::Admin)
            .await?;

        let branch = if let Some(branch_id) = request.branch_id.as_deref() {
            let branch = cms_db::branch::BranchQueries::get_by_id(&ctx.pool, branch_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;
            if branch.project_id != request.project_id {
                return Err(AppError::InvalidInput(
                    "Branch does not belong to the requested project".to_string(),
                ));
            }
            branch
        } else {
            cms_db::branch::BranchQueries::get_default(&ctx.pool, &request.project_id)
                .await?
                .ok_or_else(|| AppError::Conflict("Project has no default branch".to_string()))?
        };
        let language_id = if let Some(language_id) = request.language_id.as_deref() {
            let language = cms_db::language::LanguageQueries::get_by_id(&ctx.pool, language_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Language not found".to_string()))?;
            if language.project_id != request.project_id || !language.enabled {
                return Err(AppError::InvalidInput(
                    "Language is disabled or does not belong to the requested project".to_string(),
                ));
            }
            Some(language_id)
        } else {
            None
        };

        // Create a run using the fully resolved branch and language scope.
        let run = SearchIndexRunQueries::create(
            &ctx.pool,
            &request.project_id,
            Some(&branch.id),
            language_id,
            SearchIndexRunStatus::Processing,
        )
        .await?;

        let pages = PageQueries::get_by_project_branch_and_language(
            &ctx.pool,
            &request.project_id,
            &branch.id,
            language_id,
            None,
            Some(true),
            None,
            None,
            None,
        )
        .await?;

        let mut indexed_count = 0;
        let mut failures = Vec::new();
        for page_item in pages {
            if !page_item
                .kind
                .as_deref()
                .unwrap_or("PAGE")
                .eq_ignore_ascii_case("PAGE")
            {
                continue;
            }
            match PageQueries::get_by_id(&ctx.pool, &page_item.id).await {
                Ok(Some(page)) if page.is_published && page.is_indexed => {
                    match search_engine.index_page(&page).await {
                        Ok(()) => indexed_count += 1,
                        Err(error) => failures.push(format!("{}: {}", page.id, error)),
                    }
                }
                Ok(Some(_)) => {}
                Ok(None) => failures.push(format!("{}: page disappeared during reindex", page_item.id)),
                Err(error) => failures.push(format!("{}: {}", page_item.id, error)),
            }
        }

        let completed = if failures.is_empty() {
            SearchIndexRunQueries::mark_completed(&ctx.pool, &run.id, indexed_count).await?
        } else {
            let error_message = failures
                .iter()
                .take(20)
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join("; ");
            let error_message = error_message.chars().take(4_000).collect::<String>();
            SearchIndexRunQueries::mark_failed(
                &ctx.pool,
                &run.id,
                indexed_count,
                &error_message,
            )
            .await?
        };

        tracing::info!(
            "Reindex completed for project {} branch {}: {} page(s) indexed, {} failure(s)",
            request.project_id,
            branch.slug,
            indexed_count,
            failures.len()
        );

        Ok(completed.into())
    }
}

/// Process search job (for worker)
pub async fn process_search_job(
    pool: &cms_db::PgPool,
    search_engine: Arc<dyn cms_search::SearchEngine>,
    payload: &serde_json::Value,
) -> Result<(), AppError> {
    let job_type = payload.get("type").and_then(|v| v.as_str());
    let _project_id = payload
        .get("project_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing project_id".to_string()))?;

    match job_type {
        Some("index_page") => {
            let page_id = payload
                .get("page_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::InvalidInput("Missing page_id".to_string()))?;

            let page = PageQueries::get_by_id(pool, page_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Page not found".to_string()))?;

            search_engine.index_page(&page).await?;
        }
        Some("remove_page") => {
            let page_id = payload
                .get("page_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::InvalidInput("Missing page_id".to_string()))?;

            search_engine.remove_page(page_id).await?;
        }
        _ => {
            return Err(AppError::InvalidInput(
                "Unknown search job type".to_string(),
            ))
        }
    }

    Ok(())
}
