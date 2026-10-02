//! Export handlers
//!
//! This module contains the actual implementation of export handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::export::ExportService;
use cms_entity::{
    common::{Id, PaginatedResponse},
    export::{
        CreateExportJobRequest, CreateExportScheduleRequest, ExportJobResponse,
        ExportScheduleResponse, ListExportJobsQuery, UpdateExportScheduleRequest,
    },
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List export jobs
///
/// Returns a paginated list of export jobs filtered by snapshot, status, and format.
#[utoipa::path(
    get,
    path = "/export/jobs",
    tag = "export",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("snapshot_id", Query, description = "Filter by snapshot ID"),
        ("status", Query, description = "Filter by job status"),
        ("format", Query, description = "Filter by export format"),
        ("limit", Query, description = "Number of items per page"),
        ("offset", Query, description = "Pagination offset"),
    ),
    responses(
        (status = 200, description = "List of export jobs", body = PaginatedResponse<ExportJobResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_export_jobs_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListExportJobsQuery>,
) -> Result<Json<PaginatedResponse<ExportJobResponse>>, AppError> {
    let snapshot_id = query.snapshot_id.as_deref().unwrap_or("");
    let result = ExportService::list_export_jobs(
        &state.biz_context,
        &auth.user.id,
        snapshot_id,
        query.limit.unwrap_or(1) as u64,
        query.offset.unwrap_or(20) as u64,
    )
    .await?;

    Ok(Json(PaginatedResponse::new(
        result.data.into_iter().map(|j| j.into()).collect(),
        result.total,
        result.page,
        result.page_size,
    )))
}

/// Create a new export job
///
/// Creates a new export job for a project snapshot.
#[utoipa::path(
    post,
    path = "/export/jobs",
    tag = "export",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreateExportJobRequest,
    responses(
        (status = 200, description = "Export job created successfully", body = ExportJobResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_export_job_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateExportJobRequest>,
) -> Result<Json<ExportJobResponse>, AppError> {
    let job = ExportService::create_export_job(
        &state.biz_context,
        &auth.user.id,
        &request.snapshot_id,
        request.format,
    )
    .await?;

    Ok(Json(job.into()))
}

/// Get a specific export job
///
/// Retrieves an export job by its unique identifier.
#[utoipa::path(
    get,
    path = "/export/jobs/{job_id}",
    tag = "export",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("job_id", Path, description = "The ID of the export job to retrieve"),
    ),
    responses(
        (status = 200, description = "Export job found", body = ExportJobResponse),
        (status = 404, description = "Export job not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_export_job_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(job_id): Path<Id>,
) -> Result<Json<ExportJobResponse>, AppError> {
    let job = ExportService::get_export_job(&state.biz_context, &auth.user.id, &job_id).await?;

    Ok(Json(job.into()))
}

/// Download export artifact
///
/// Returns a download URL for the exported artifact.
#[utoipa::path(
    get,
    path = "/export/jobs/{job_id}/download",
    tag = "export",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("job_id", Path, description = "The ID of the export job"),
    ),
    responses(
        (status = 200, description = "Download URL", body = serde_json::Value),
        (status = 404, description = "Export job not found or not ready"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn download_export_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(job_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    let download_url =
        ExportService::get_download_url(&state.biz_context, &auth.user.id, &job_id).await?;

    Ok(Json(serde_json::json!({"download_url": download_url})))
}

/// List export schedules for a project
///
/// Returns all export schedules for a specific project.
#[utoipa::path(
    get,
    path = "/export/projects/{project_id}/schedules",
    tag = "export",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Path, description = "The ID of the project"),
    ),
    responses(
        (status = 200, description = "List of export schedules", body = Vec<ExportScheduleResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Project not found"),
    )
)]
pub async fn list_export_schedules_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<Id>,
) -> Result<Json<Vec<ExportScheduleResponse>>, AppError> {
    let schedules =
        ExportService::list_export_schedules(&state.biz_context, &auth.user.id, &project_id)
            .await?;

    Ok(Json(schedules.into_iter().map(|s| s.into()).collect()))
}

/// Create a new export schedule
///
/// Creates a new recurring export schedule for a project.
#[utoipa::path(
    post,
    path = "/export/projects/{project_id}/schedules",
    tag = "export",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("project_id", Path, description = "The ID of the project"),
    ),
    request_body = CreateExportScheduleRequest,
    responses(
        (status = 200, description = "Export schedule created successfully", body = ExportScheduleResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_export_schedule_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(project_id): Path<Id>,
    Json(request): Json<CreateExportScheduleRequest>,
) -> Result<Json<ExportScheduleResponse>, AppError> {
    let schedule = ExportService::create_export_schedule(
        &state.biz_context,
        &auth.user.id,
        &project_id,
        request.format,
        &request.frequency,
        request.day_of_week,
        request.day_of_month,
        &request.time_of_day,
    )
    .await?;

    Ok(Json(schedule.into()))
}

/// Update an export schedule
///
/// Updates an export schedule by its ID.
#[utoipa::path(
    put,
    path = "/export/schedules/{schedule_id}",
    tag = "export",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("schedule_id", Path, description = "The ID of the export schedule to update"),
    ),
    request_body = UpdateExportScheduleRequest,
    responses(
        (status = 200, description = "Export schedule updated successfully", body = ExportScheduleResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Export schedule not found"),
    )
)]
pub async fn update_export_schedule_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(schedule_id): Path<Id>,
    Json(request): Json<UpdateExportScheduleRequest>,
) -> Result<Json<ExportScheduleResponse>, AppError> {
    let schedule = ExportService::update_export_schedule(
        &state.biz_context,
        &auth.user.id,
        &schedule_id,
        request,
    )
    .await?;

    Ok(Json(schedule.into()))
}

/// Delete an export schedule
///
/// Permanently deletes an export schedule by its ID.
#[utoipa::path(
    delete,
    path = "/export/schedules/{schedule_id}",
    tag = "export",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("schedule_id", Path, description = "The ID of the export schedule to delete"),
    ),
    responses(
        (status = 200, description = "Export schedule deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Export schedule not found"),
    )
)]
pub async fn delete_export_schedule_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(schedule_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    ExportService::delete_export_schedule(&state.biz_context, &auth.user.id, &schedule_id).await?;

    Ok(Json(
        serde_json::json!({"success": true, "id": schedule_id}),
    ))
}
