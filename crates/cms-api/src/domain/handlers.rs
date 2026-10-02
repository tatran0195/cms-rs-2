//! Domain handlers
//!
//! This module contains the actual implementation of domain handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use cms_biz::domain::DomainService;
use cms_entity::{
    common::{Id, PaginatedResponse},
    domain::{
        CreateDomainRequest, DomainResponse, DomainVerificationResult, ListDomainsQuery,
        UpdateDomainRequest, VerifyDomainRequest,
    },
};
use cms_error::AppError;
use cms_middleware::app_state::AppState;
use utoipa::ToSchema;

use crate::auth::AuthExtractor;

/// List domains
///
/// Returns all domains filtered by deployment, primary status, and verification status.
#[utoipa::path(
    get,
    path = "/domains",
    tag = "domains",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("deployment_id", Query, description = "Filter by deployment ID"),
        ("is_primary", Query, description = "Filter by primary status"),
        ("is_verified", Query, description = "Filter by verification status"),
    ),
    responses(
        (status = 200, description = "List of domains", body = Vec<DomainResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn list_domains_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Query(query): Query<ListDomainsQuery>,
) -> Result<Json<Vec<DomainResponse>>, AppError> {
    let domains = DomainService::list_domains(
        &state.biz_context,
        &auth.user.id,
        query.deployment_id.as_deref(),
        query.is_primary,
        query.is_verified,
    )
    .await?;

    Ok(Json(domains))
}

/// Create a new custom domain
///
/// Adds a custom domain to a deployment.
#[utoipa::path(
    post,
    path = "/domains",
    tag = "domains",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    request_body = CreateDomainRequest,
    responses(
        (status = 200, description = "Domain created successfully", body = DomainResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
    )
)]
pub async fn create_domain_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Json(request): Json<CreateDomainRequest>,
) -> Result<Json<DomainResponse>, AppError> {
    let domain = DomainService::create_domain(&state.biz_context, &auth.user.id, request).await?;
    state.invalidate_host_resolution_cache();

    Ok(Json(domain))
}

/// Get a specific domain by ID
///
/// Retrieves a domain by its unique identifier.
#[utoipa::path(
    get,
    path = "/domains/{domain_id}",
    tag = "domains",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("domain_id", Path, description = "The ID of the domain to retrieve"),
    ),
    responses(
        (status = 200, description = "Domain found", body = DomainResponse),
        (status = 404, description = "Domain not found"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_domain_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(domain_id): Path<Id>,
) -> Result<Json<DomainResponse>, AppError> {
    let domain = DomainService::get_domain(&state.biz_context, &auth.user.id, &domain_id).await?;

    Ok(Json(domain))
}

/// Update a domain
///
/// Updates a domain by its ID with the provided fields.
#[utoipa::path(
    put,
    path = "/domains/{domain_id}",
    tag = "domains",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("domain_id", Path, description = "The ID of the domain to update"),
    ),
    request_body = UpdateDomainRequest,
    responses(
        (status = 200, description = "Domain updated successfully", body = DomainResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Domain not found"),
    )
)]
pub async fn update_domain_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(domain_id): Path<Id>,
    Json(request): Json<UpdateDomainRequest>,
) -> Result<Json<DomainResponse>, AppError> {
    let domain =
        DomainService::update_domain(&state.biz_context, &auth.user.id, &domain_id, request)
            .await?;
    state.invalidate_host_resolution_cache();

    Ok(Json(domain))
}

/// Delete a domain
///
/// Permanently deletes a domain by its ID.
#[utoipa::path(
    delete,
    path = "/domains/{domain_id}",
    tag = "domains",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("domain_id", Path, description = "The ID of the domain to delete"),
    ),
    responses(
        (status = 200, description = "Domain deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Domain not found"),
    )
)]
pub async fn delete_domain_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(domain_id): Path<Id>,
) -> Result<Json<serde_json::Value>, AppError> {
    DomainService::delete_domain(&state.biz_context, &auth.user.id, &domain_id).await?;
    state.invalidate_host_resolution_cache();

    Ok(Json(serde_json::json!({"success": true, "id": domain_id})))
}

/// Verify a domain
///
/// Verifies domain ownership using a verification token.
#[utoipa::path(
    post,
    path = "/domains/{domain_id}/verify",
    tag = "domains",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("domain_id", Path, description = "The ID of the domain to verify"),
    ),
    request_body = VerifyDomainRequest,
    responses(
        (status = 200, description = "Domain verification result", body = DomainVerificationResult),
        (status = 400, description = "Bad request - invalid verification token"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Domain not found"),
    )
)]
pub async fn verify_domain_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(domain_id): Path<Id>,
    Json(request): Json<VerifyDomainRequest>,
) -> Result<Json<DomainVerificationResult>, AppError> {
    let result = DomainService::verify_domain(
        &state.biz_context,
        &auth.user.id,
        &domain_id,
        &request.verification_token,
        &state.config.domain_verification.resolver_url,
        state.config.domain_verification.timeout_seconds,
    )
    .await?;
    state.invalidate_host_resolution_cache();

    Ok(Json(result))
}

/// Check domain availability
///
/// Checks if a hostname is available for use.
#[utoipa::path(
    get,
    path = "/domains/availability/{hostname}",
    tag = "domains",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("hostname", Path, description = "The hostname to check for availability"),
    ),
    responses(
        (status = 200, description = "Domain availability result", body = serde_json::Value),
        (status = 400, description = "Bad request - invalid hostname"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn check_domain_availability_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(hostname): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let is_available = DomainService::is_domain_available(&state.biz_context, &hostname).await?;

    Ok(Json(
        serde_json::json!({"available": is_available, "hostname": hostname}),
    ))
}

/// Set primary domain for a deployment
///
/// Sets the specified domain as the primary domain for a deployment.
#[utoipa::path(
    post,
    path = "/domains/set-primary/{deployment_id}",
    tag = "domains",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    params(
        ("deployment_id", Path, description = "The ID of the deployment"),
    ),
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Primary domain set successfully", body = DomainResponse),
        (status = 400, description = "Bad request - domain_id is required"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - user may not have permission"),
        (status = 404, description = "Deployment not found"),
    )
)]
pub async fn set_primary_domain_handler(
    State(state): State<Arc<AppState>>,
    auth: AuthExtractor,
    Path(deployment_id): Path<Id>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<DomainResponse>, AppError> {
    let domain_id: String = serde_json::from_value(
        request
            .get("domain_id")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|_| AppError::BadRequest("Invalid domain_id".to_string()))?;

    let domain = DomainService::set_primary_domain(
        &state.biz_context,
        &auth.user.id,
        &deployment_id,
        &domain_id,
    )
    .await?;
    state.invalidate_host_resolution_cache();

    Ok(Json(domain))
}
