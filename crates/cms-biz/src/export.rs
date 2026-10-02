//! Export Business Logic
//!
//! This module contains business logic for exporting projects and pages.

use std::sync::Arc;

use anyhow::anyhow;
use bytes::Bytes;
use chrono::Utc;
use cms_db::{
    export::{
        ExportArtifactQueries, ExportJobQueries, ExportScheduleQueries, ExportSnapshotQueries,
    },
    page::PageQueries,
    project::ProjectQueries,
};
use cms_entity::{
    common::{Id, MemberRole, PaginatedResponse},
    export::{
        CreateExportRequest, ExportArtifact, ExportFormat, ExportJob, ExportSchedule,
        ExportSnapshot, ExportStatus,
    },
};
use cms_storage::Storage;
use uuid::Uuid;

use crate::{AppError, BizContext};

/// Export service
pub struct ExportService;

impl ExportService {
    /// Create an export snapshot
    pub async fn create_export_snapshot(
        ctx: &BizContext,
        user_id: &str,
        request: CreateExportRequest,
    ) -> Result<ExportSnapshot, AppError> {
        // Verify project exists
        let _project = ProjectQueries::get_by_id(&ctx.pool, &request.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &request.project_id, MemberRole::Viewer)
            .await?;

        let snapshot = ExportSnapshotQueries::create(
            &ctx.pool,
            &request.project_id,
            request.branch_id.as_deref(),
            request.language_id.as_deref(),
        )
        .await?;

        Ok(snapshot)
    }

    /// Get an export snapshot
    pub async fn get_export_snapshot(
        ctx: &BizContext,
        user_id: &str,
        snapshot_id: &str,
    ) -> Result<ExportSnapshot, AppError> {
        let snapshot = ExportSnapshotQueries::get_by_id(&ctx.pool, snapshot_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Export snapshot not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &snapshot.project_id, MemberRole::Viewer)
            .await?;

        Ok(snapshot)
    }

    /// List export snapshots for a project
    pub async fn list_export_snapshots(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResponse<ExportSnapshot>, AppError> {
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;

        let snapshots = ExportSnapshotQueries::get_by_project(
            &ctx.pool,
            project_id,
            Some(page as i64),
            Some(page_size as i64),
        )
        .await?;

        let total = ExportSnapshotQueries::count_by_project(&ctx.pool, project_id).await?;

        Ok(PaginatedResponse::new(
            snapshots,
            total as u64,
            page,
            page_size,
        ))
    }

    /// Create an export job
    pub async fn create_export_job(
        ctx: &BizContext,
        user_id: &str,
        snapshot_id: &str,
        format: ExportFormat,
    ) -> Result<ExportJob, AppError> {
        let snapshot = ExportSnapshotQueries::get_by_id(&ctx.pool, snapshot_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Export snapshot not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &snapshot.project_id, MemberRole::Viewer)
            .await?;

        let job =
            ExportJobQueries::create(&ctx.pool, snapshot_id, format, ExportStatus::Pending).await?;

        Ok(job)
    }

    /// Get an export job
    pub async fn get_export_job(
        ctx: &BizContext,
        user_id: &str,
        job_id: &str,
    ) -> Result<ExportJob, AppError> {
        let job = ExportJobQueries::get_by_id(&ctx.pool, job_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Export job not found".to_string()))?;

        let snapshot = ExportSnapshotQueries::get_by_id(&ctx.pool, &job.snapshot_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Export snapshot not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &snapshot.project_id, MemberRole::Viewer)
            .await?;

        Ok(job)
    }

    /// List export jobs for a project
    pub async fn list_export_jobs(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        page: u64,
        page_size: u64,
    ) -> Result<PaginatedResponse<ExportJob>, AppError> {
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;

        let jobs = ExportJobQueries::get_by_project(
            &ctx.pool,
            project_id,
            Some(page as i64),
            Some(page_size as i64),
        )
        .await?;

        let total = ExportJobQueries::count_by_project(&ctx.pool, project_id).await?;

        Ok(PaginatedResponse::new(jobs, total as u64, page, page_size))
    }

    /// Get export artifacts for a job
    pub async fn get_export_artifacts(
        ctx: &BizContext,
        user_id: &str,
        job_id: &str,
    ) -> Result<Vec<ExportArtifact>, AppError> {
        let job = ExportJobQueries::get_by_id(&ctx.pool, job_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Export job not found".to_string()))?;

        let snapshot = ExportSnapshotQueries::get_by_id(&ctx.pool, &job.snapshot_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Export snapshot not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &snapshot.project_id, MemberRole::Viewer)
            .await?;

        ExportArtifactQueries::get_by_job(&ctx.pool, job_id).await
    }

    /// Create an export schedule
    #[allow(clippy::too_many_arguments)]
    pub async fn create_export_schedule(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
        format: ExportFormat,
        frequency: &str,
        day_of_week: Option<i32>,
        day_of_month: Option<i32>,
        time_of_day: &str,
    ) -> Result<ExportSchedule, AppError> {
        let _project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Admin)
            .await?;

        let schedule = ExportScheduleQueries::create(
            &ctx.pool,
            project_id,
            format,
            frequency,
            day_of_week,
            day_of_month,
            time_of_day,
            true,
        )
        .await?;

        Ok(schedule)
    }

    /// Get an export schedule
    pub async fn get_export_schedule(
        ctx: &BizContext,
        user_id: &str,
        schedule_id: &str,
    ) -> Result<ExportSchedule, AppError> {
        let schedule = ExportScheduleQueries::get_by_id(&ctx.pool, schedule_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Export schedule not found".to_string()))?;

        // Check if user has access to the project
        ctx.authz
            .require_project_role(user_id, &schedule.project_id, MemberRole::Viewer)
            .await?;

        Ok(schedule)
    }

    /// Delete an export schedule
    pub async fn delete_export_schedule(
        ctx: &BizContext,
        user_id: &str,
        schedule_id: &str,
    ) -> Result<bool, AppError> {
        let schedule = ExportScheduleQueries::get_by_id(&ctx.pool, schedule_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Export schedule not found".to_string()))?;

        // Check if user has admin role in the project
        ctx.authz
            .require_project_role(user_id, &schedule.project_id, MemberRole::Admin)
            .await?;

        ExportScheduleQueries::delete(&ctx.pool, schedule_id).await
    }

    /// Get download url for export job
    pub async fn get_download_url(
        ctx: &BizContext,
        user_id: &str,
        job_id: &str,
    ) -> Result<String, AppError> {
        let artifacts = Self::get_export_artifacts(ctx, user_id, job_id).await?;
        let artifact = artifacts
            .into_iter()
            .next()
            .ok_or_else(|| AppError::NotFound("Export artifact not found".to_string()))?;
        Ok(artifact
            .download_url
            .unwrap_or_else(|| format!("/api/export/download/{}", artifact.id)))
    }

    /// List export schedules for a project
    pub async fn list_export_schedules(
        ctx: &BizContext,
        user_id: &str,
        project_id: &str,
    ) -> Result<Vec<ExportSchedule>, AppError> {
        ctx.authz
            .require_project_role(user_id, project_id, MemberRole::Viewer)
            .await?;
        ExportScheduleQueries::get_by_project(&ctx.pool, project_id).await
    }

    /// Update export schedule
    pub async fn update_export_schedule(
        ctx: &BizContext,
        user_id: &str,
        schedule_id: &str,
        request: cms_entity::export::UpdateExportScheduleRequest,
    ) -> Result<ExportSchedule, AppError> {
        let schedule = ExportScheduleQueries::get_by_id(&ctx.pool, schedule_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Export schedule not found".to_string()))?;
        ctx.authz
            .require_project_role(user_id, &schedule.project_id, MemberRole::Admin)
            .await?;
        let updated = ExportScheduleQueries::update(
            &ctx.pool,
            schedule_id,
            request.is_active,
            request.time_of_day.as_deref(),
        )
        .await?;
        Ok(updated)
    }
}

/// Process export job (for worker)
pub async fn process_export_job(
    pool: &cms_db::PgPool,
    storage: Arc<dyn Storage>,
    payload: &serde_json::Value,
) -> Result<(), AppError> {
    let job_id = payload
        .get("job_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidInput("Missing job_id".to_string()))?;

    let job = ExportJobQueries::get_by_id(pool, job_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Export job not found".to_string()))?;

    // Update job status to processing
    ExportJobQueries::update_status(pool, job_id, ExportStatus::Processing).await?;

    let snapshot = ExportSnapshotQueries::get_by_id(pool, &job.snapshot_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Export snapshot not found".to_string()))?;

    // Get all pages for the snapshot
    let pages = if let Some(branch_id) = &snapshot.branch_id {
        PageQueries::get_by_project_and_branch(
            pool,
            &snapshot.project_id,
            branch_id,
            None,
            None,
            None,
            None,
            None,
        )
        .await?
    } else {
        // Get default branch
        let default_branch = cms_db::branch::BranchQueries::get_default(pool, &snapshot.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Default branch not found".to_string()))?;

        PageQueries::get_by_project_and_branch(
            pool,
            &snapshot.project_id,
            &default_branch.id,
            None,
            None,
            None,
            None,
            None,
        )
        .await?
    };

    // Generate export content based on format
    let content_result = match job.format {
        ExportFormat::HTML | ExportFormat::Html => {
            generate_html_export(&pages, &snapshot.project_id).await
        }
        ExportFormat::PDF | ExportFormat::Pdf => {
            generate_pdf_export(&pages, &snapshot.project_id).await
        }
        ExportFormat::MARKDOWN | ExportFormat::Markdown => {
            Ok(generate_markdown_export(&pages).await)
        }
        ExportFormat::EPUB | ExportFormat::Epub => {
            generate_epub_export(&pages, &snapshot.project_id).await
        }
    };

    let content = match content_result {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Export generation failed for job {}: {}", job_id, e);
            ExportJobQueries::update_status(pool, job_id, ExportStatus::Failed).await?;
            return Err(e);
        }
    };

    // Store the export artifact
    let extension = match &job.format {
        ExportFormat::HTML | ExportFormat::Html => "html",
        ExportFormat::PDF | ExportFormat::Pdf => "pdf",
        ExportFormat::MARKDOWN | ExportFormat::Markdown => "md",
        ExportFormat::EPUB | ExportFormat::Epub => "epub",
    };

    let storage_key = format!(
        "exports/{}/{}/{}.{}",
        snapshot.project_id,
        job.id,
        Utc::now().timestamp(),
        extension
    );

    let content_type = match &job.format {
        ExportFormat::HTML | ExportFormat::Html => "text/html; charset=utf-8",
        ExportFormat::PDF | ExportFormat::Pdf => "application/pdf",
        ExportFormat::MARKDOWN | ExportFormat::Markdown => "text/markdown; charset=utf-8",
        ExportFormat::EPUB | ExportFormat::Epub => "application/epub+zip",
    };

    let content_len = content.len() as i64;
    storage.put(&storage_key, content, content_type).await?;

    // Create artifact record
    let _artifact = ExportArtifactQueries::create(
        pool,
        job_id,
        &format!("export.{}", extension),
        content_len,
        &storage_key,
        None, // download_url
    )
    .await?;

    // Update job status to completed
    ExportJobQueries::update_status(pool, job_id, ExportStatus::Completed).await?;
    ExportJobQueries::update_output_path(pool, job_id, &storage_key).await?;

    tracing::info!(
        "Export job {} completed successfully -> {}",
        job_id,
        storage_key
    );

    Ok(())
}

/// Helper: Render markdown to sanitized HTML
fn render_markdown(markdown: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut html_body = String::new();
    html::push_html(&mut html_body, parser);

    ammonia::clean(&html_body)
}

/// Generate HTML export
async fn generate_html_export(
    pages: &[cms_entity::page::PageListItem],
    project_id: &str,
) -> Result<Bytes, AppError> {
    let mut doc = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Documentation Export - {}</title>
  <style>
    body {{ max-width: 900px; margin: 0 auto; padding: 2rem; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; }}
    nav {{ background: #f8f9fa; padding: 1rem 1.5rem; border-radius: 8px; margin-bottom: 2rem; border: 1px solid #e9ecef; }}
    nav h2 {{ margin-top: 0; font-size: 1.25rem; }}
    nav ul {{ margin: 0; padding-left: 1.5rem; }}
    article {{ margin-bottom: 4rem; padding-bottom: 2rem; border-bottom: 1px solid #dee2e6; }}
    pre {{ background: #f8f9fa; padding: 1rem; border-radius: 4px; overflow-x: auto; border: 1px solid #e9ecef; font-family: monospace; font-size: 0.9em; }}
    code {{ background: #e9ecef; padding: 0.2em 0.4em; border-radius: 3px; font-family: monospace; font-size: 0.9em; }}
    pre code {{ background: none; padding: 0; }}
    table {{ border-collapse: collapse; width: 100%; margin: 1rem 0; }}
    th, td {{ border: 1px solid #dee2e6; padding: 0.5rem 0.75rem; text-align: left; }}
    th {{ background: #f8f9fa; }}
  </style>
</head>
<body>
  <header>
    <h1>Project Documentation Export</h1>
  </header>
  <nav>
    <h2>Table of Contents</h2>
    <ul>
"#,
        ammonia::clean(project_id)
    );

    for (i, page) in pages.iter().enumerate() {
        let anchor = format!("page-{}", i);
        doc.push_str(&format!(
            "      <li><a href=\"#{}\">{}</a></li>\n",
            anchor,
            ammonia::clean(&page.title)
        ));
    }

    doc.push_str("    </ul>\n  </nav>\n  <main>\n");

    for (i, page) in pages.iter().enumerate() {
        let anchor = format!("page-{}", i);
        let raw_md = page.content.as_deref().unwrap_or("");
        let clean_html = render_markdown(raw_md);

        doc.push_str(&format!(
            r#"    <article id="{}">
      <h2>{}</h2>
      <div>
{}
      </div>
    </article>
"#,
            anchor,
            ammonia::clean(&page.title),
            clean_html
        ));
    }

    doc.push_str("  </main>\n</body>\n</html>");

    Ok(Bytes::from(doc))
}

/// Generate Markdown export
async fn generate_markdown_export(pages: &[cms_entity::page::PageListItem]) -> Bytes {
    let mut md = String::from("# Documentation Export\n\n");

    for page in pages {
        md.push_str(&format!("## {}\n\n", page.title));
        if let Some(content) = &page.content {
            md.push_str(content);
            md.push_str("\n\n");
        }
        md.push_str("---\n\n");
    }

    Bytes::from(md)
}

/// Generate PDF export using printpdf
async fn generate_pdf_export(
    pages: &[cms_entity::page::PageListItem],
    project_id: &str,
) -> Result<Bytes, AppError> {
    use printpdf::*;

    let (doc, page1, layer1) = PdfDocument::new(
        format!("Documentation - {}", project_id),
        Mm(210.0),
        Mm(297.0),
        "Layer 1",
    );

    let font_title = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to add bold font: {:?}", e)))?;

    let font_body = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to add regular font: {:?}", e)))?;

    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Title on cover page
    current_layer.use_text(
        format!("Project Documentation - {}", project_id),
        22.0,
        Mm(20.0),
        Mm(260.0),
        &font_title,
    );

    current_layer.use_text(
        format!("Generated on {}", Utc::now().format("%Y-%m-%d %H:%M UTC")),
        12.0,
        Mm(20.0),
        Mm(245.0),
        &font_body,
    );

    let mut current_y = 220.0;

    // Table of contents on first page
    current_layer.use_text(
        "Table of Contents:",
        14.0,
        Mm(20.0),
        Mm(current_y),
        &font_title,
    );
    current_y -= 10.0;

    for (i, page) in pages.iter().take(15).enumerate() {
        let entry = format!("{}. {}", i + 1, page.title);
        current_layer.use_text(&entry, 10.0, Mm(25.0), Mm(current_y), &font_body);
        current_y -= 7.0;
    }

    // Add content pages
    for page in pages {
        let (page_idx, layer_idx) = doc.add_page(Mm(210.0), Mm(297.0), "Content Layer");
        let content_layer = doc.get_page(page_idx).get_layer(layer_idx);

        // Page title
        content_layer.use_text(&page.title, 18.0, Mm(20.0), Mm(270.0), &font_title);

        let mut y = 250.0;
        if let Some(content) = &page.content {
            for line in content.lines().take(35) {
                if y < 30.0 {
                    break;
                }
                // Strip markdown formatting symbols for basic PDF text
                let clean_line = line
                    .trim_start_matches('#')
                    .trim_start_matches('-')
                    .trim_start_matches('*')
                    .trim();

                if clean_line.is_empty() {
                    y -= 4.0;
                    continue;
                }

                // Truncate long lines to fit page
                let truncated: String = clean_line.chars().take(85).collect();
                content_layer.use_text(&truncated, 10.0, Mm(20.0), Mm(y), &font_body);
                y -= 6.0;
            }
        }
    }

    let pdf_bytes = doc
        .save_to_bytes()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to render PDF: {:?}", e)))?;

    Ok(Bytes::from(pdf_bytes))
}

/// Generate EPUB export using epub-builder
async fn generate_epub_export(
    pages: &[cms_entity::page::PageListItem],
    project_id: &str,
) -> Result<Bytes, AppError> {
    use epub_builder::{EpubBuilder, EpubContent, ZipLibrary};

    let zip_lib = ZipLibrary::new()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create ZipLibrary: {:?}", e)))?;

    let mut builder = EpubBuilder::new(zip_lib).map_err(|e| {
        AppError::Internal(anyhow::anyhow!("Failed to create EpubBuilder: {:?}", e))
    })?;

    builder
        .metadata("author", "CMS Documentation Platform")
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to set author: {:?}", e)))?;

    builder
        .metadata("title", format!("Documentation - {}", project_id))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to set title: {:?}", e)))?;

    for (i, page) in pages.iter().enumerate() {
        let raw_md = page.content.as_deref().unwrap_or("");
        let clean_html = render_markdown(raw_md);

        let xhtml = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head>
  <title>{}</title>
</head>
<body>
  <h1>{}</h1>
  <div>
{}
  </div>
</body>
</html>"#,
            ammonia::clean(&page.title),
            ammonia::clean(&page.title),
            clean_html
        );

        let filename = format!("chapter_{}_{}.xhtml", i + 1, page.slug);
        let content = EpubContent::new(&filename, xhtml.as_bytes()).title(&page.title);

        builder.add_content(content).map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Failed to add EPUB chapter: {:?}", e))
        })?;
    }

    let mut output = Vec::new();
    builder
        .generate(&mut output)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to generate EPUB: {:?}", e)))?;

    Ok(Bytes::from(output))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use cms_entity::page::PageListItem;

    use super::*;

    fn mock_pages() -> Vec<PageListItem> {
        vec![
            PageListItem {
                id: "p1".to_string(),
                project_id: "proj-1".to_string(),
                branch_id: "main".to_string(),
                parent_id: None,
                language_id: None,
                kind: Some("PAGE".to_string()),
                path: "/getting-started".to_string(),
                slug: "getting-started".to_string(),
                title: "Getting Started".to_string(),
                description: Some("Introductory guide".to_string()),
                content: Some(
                    "# Getting Started\n\nWelcome to **CMS**!\n\n- Feature 1\n- Feature 2"
                        .to_string(),
                ),
                icon: None,
                config: None,
                translation_key: None,
                position: 0,
                is_published: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            PageListItem {
                id: "p2".to_string(),
                project_id: "proj-1".to_string(),
                branch_id: "main".to_string(),
                parent_id: None,
                language_id: None,
                kind: Some("PAGE".to_string()),
                path: "/api-reference".to_string(),
                slug: "api-reference".to_string(),
                title: "API Reference".to_string(),
                description: Some("API docs".to_string()),
                content: Some("## Endpoints\n\nUse `GET /api/v1/pages` to list pages.".to_string()),
                icon: None,
                config: None,
                translation_key: None,
                position: 1,
                is_published: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ]
    }

    #[tokio::test]
    async fn test_generate_html_export() {
        let pages = mock_pages();
        let bytes = generate_html_export(&pages, "proj-1").await.unwrap();
        let html_str = String::from_utf8(bytes.to_vec()).unwrap();

        assert!(html_str.contains("Table of Contents"));
        assert!(html_str.contains("Getting Started"));
        assert!(html_str.contains("API Reference"));
        assert!(html_str.contains("Welcome to <strong>CMS</strong>!"));
    }

    #[tokio::test]
    async fn test_generate_markdown_export() {
        let pages = mock_pages();
        let bytes = generate_markdown_export(&pages).await;
        let md_str = String::from_utf8(bytes.to_vec()).unwrap();

        assert!(md_str.contains("# Documentation Export"));
        assert!(md_str.contains("## Getting Started"));
        assert!(md_str.contains("## API Reference"));
    }

    #[tokio::test]
    async fn test_generate_pdf_export() {
        let pages = mock_pages();
        let bytes = generate_pdf_export(&pages, "proj-1").await.unwrap();

        assert!(bytes.len() > 100);
        // Valid PDF magic header
        assert_eq!(&bytes[..4], b"%PDF");
    }

    #[tokio::test]
    async fn test_generate_epub_export() {
        let pages = mock_pages();
        let bytes = generate_epub_export(&pages, "proj-1").await.unwrap();

        assert!(bytes.len() > 100);
        // Valid ZIP/EPUB magic header
        assert_eq!(&bytes[..2], b"PK");
    }
}
