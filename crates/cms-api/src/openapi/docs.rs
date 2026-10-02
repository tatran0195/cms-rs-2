use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme},
    Modify, OpenApi,
};

/// Define the OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        // Admin endpoints
        crate::admin::handlers::list_all_organizations_handler,
        crate::admin::handlers::get_organization_stats_handler,
        crate::admin::handlers::get_system_stats_handler,
        crate::admin::handlers::get_system_health_handler,

        // Analytics endpoints
        crate::analytics::handlers::track_event_handler,
        crate::analytics::handlers::list_analytics_events_handler,
        crate::analytics::handlers::query_analytics_handler,
        crate::analytics::handlers::get_analytics_dashboard_handler,
        crate::analytics::handlers::get_page_views_handler,

        // Asset endpoints
        crate::asset::handlers::list_assets_handler,
        crate::asset::handlers::upload_asset_handler,
        crate::asset::handlers::upload_asset_multipart_handler,
        crate::asset::handlers::get_asset_handler,
        crate::asset::handlers::update_asset_handler,
        crate::asset::handlers::delete_asset_handler,

        // Auth endpoints
        crate::auth::handlers::login_handler,
        crate::auth::handlers::register_handler,

        // Branch endpoints
        crate::branch::handlers::list_branches_handler,
        crate::branch::handlers::create_branch_handler,
        crate::branch::handlers::get_branch_handler,
        crate::branch::handlers::update_branch_handler,
        crate::branch::handlers::delete_branch_handler,

        // Comment endpoints
        crate::comment::handlers::list_comments_handler,
        crate::comment::handlers::create_comment_handler,
        crate::comment::handlers::get_comment_handler,
        crate::comment::handlers::update_comment_handler,
        crate::comment::handlers::delete_comment_handler,
        crate::comment::handlers::resolve_comment_handler,
        crate::comment::handlers::list_page_comments_handler,

        // Deployment endpoints
        crate::deployment::handlers::list_deployments_handler,
        crate::deployment::handlers::create_deployment_handler,
        crate::deployment::handlers::get_deployment_handler,
        crate::deployment::handlers::get_deployment_logs_handler,
        crate::deployment::handlers::retry_deployment_handler,
        crate::deployment::handlers::cancel_deployment_handler,

        // Domain endpoints
        crate::domain::handlers::list_domains_handler,
        crate::domain::handlers::create_domain_handler,
        crate::domain::handlers::get_domain_handler,
        crate::domain::handlers::update_domain_handler,
        crate::domain::handlers::delete_domain_handler,
        crate::domain::handlers::verify_domain_handler,
        crate::domain::handlers::check_domain_availability_handler,
        crate::domain::handlers::set_primary_domain_handler,

        // Export endpoints
        crate::export::handlers::list_export_jobs_handler,
        crate::export::handlers::create_export_job_handler,
        crate::export::handlers::get_export_job_handler,
        crate::export::handlers::download_export_handler,
        crate::export::handlers::list_export_schedules_handler,
        crate::export::handlers::create_export_schedule_handler,
        crate::export::handlers::update_export_schedule_handler,
        crate::export::handlers::delete_export_schedule_handler,

        // Git endpoints
        crate::git::handlers::create_git_connection_handler,
        crate::git::handlers::get_git_connection_handler,
        crate::git::handlers::list_git_connections_handler,
        crate::git::handlers::update_git_connection_handler,
        crate::git::handlers::delete_git_connection_handler,
        crate::git::handlers::trigger_git_sync_handler,
        crate::git::handlers::list_git_sync_operations_handler,
        crate::git::handlers::get_git_sync_operation_handler,
        crate::git::handlers::get_git_sync_status_handler,

        // Integration endpoints
        crate::integration::handlers::list_integrations_handler,
        crate::integration::handlers::create_integration_handler,
        crate::integration::handlers::get_integration_handler,
        crate::integration::handlers::update_integration_handler,
        crate::integration::handlers::delete_integration_handler,
        crate::integration::handlers::enable_integration_handler,
        crate::integration::handlers::disable_integration_handler,
        crate::integration::handlers::test_integration_handler,

        // Language endpoints
        crate::language::handlers::list_languages_handler,
        crate::language::handlers::create_language_handler,
        crate::language::handlers::get_language_handler,
        crate::language::handlers::update_language_handler,
        crate::language::handlers::delete_language_handler,
        crate::language::handlers::set_default_language_handler,

        // Mcp endpoints
        crate::mcp::handlers::list_mcp_audit_events_handler,
        crate::mcp::handlers::get_mcp_server_info_handler,
        crate::mcp::handlers::list_mcp_tools_handler,
        crate::mcp::handlers::call_mcp_tool_handler,
        crate::mcp::handlers::list_mcp_resources_handler,
        crate::mcp::handlers::read_mcp_resource_handler,

        // Notification endpoints
        crate::notification::handlers::list_notifications_handler,
        crate::notification::handlers::get_notification_handler,
        crate::notification::handlers::mark_notification_read_handler,
        crate::notification::handlers::mark_all_notifications_read_handler,
        crate::notification::handlers::archive_notification_handler,
        crate::notification::handlers::get_notification_count_handler,

        // Openapi endpoints
        crate::openapi::handlers::list_openapi_documents_handler,
        crate::openapi::handlers::create_openapi_document_handler,
        crate::openapi::handlers::get_openapi_document_handler,
        crate::openapi::handlers::update_openapi_document_handler,
        crate::openapi::handlers::delete_openapi_document_handler,
        crate::openapi::handlers::parse_openapi_document_handler,
        crate::openapi::handlers::get_openapi_content_handler,

        // Org endpoints
        crate::org::handlers::list_orgs_handler,
        crate::org::handlers::create_org_handler,

        // Page endpoints
        crate::page::handlers::list_pages_handler,
        crate::page::handlers::create_page_handler,
        crate::page::handlers::get_page_handler,
        crate::page::handlers::update_page_handler,
        crate::page::handlers::delete_page_handler,

        // Platform_event endpoints
        crate::platform_event::handlers::list_platform_events_handler,
        crate::platform_event::handlers::create_platform_event_handler,
        crate::platform_event::handlers::get_platform_event_handler,

        // Project endpoints
        crate::project::handlers::list_projects_handler,
        crate::project::handlers::create_project_handler,

        // Public endpoints
        crate::public::handlers::get_public_project_handler,
        crate::public::handlers::get_public_page_handler,

        // Reader_access endpoints
        crate::reader_access::handlers::create_audience_handler,
        crate::reader_access::handlers::list_audiences_handler,
        crate::reader_access::handlers::get_audience_handler,
        crate::reader_access::handlers::update_audience_handler,
        crate::reader_access::handlers::delete_audience_handler,
        crate::reader_access::handlers::create_audience_grant_handler,
        crate::reader_access::handlers::list_audience_grants_handler,
        crate::reader_access::handlers::delete_audience_grant_handler,
        crate::reader_access::handlers::create_reader_invitation_handler,
        crate::reader_access::handlers::accept_reader_invitation_handler,

        // Search endpoints
        crate::search::handlers::search_handler,
        crate::search::handlers::reindex_handler,
        crate::search::handlers::list_search_index_runs_handler,
        crate::search::handlers::get_search_index_run_handler,
        crate::search::handlers::get_search_status_handler,

        // Theme endpoints
        crate::theme::handlers::list_themes_handler,
        crate::theme::handlers::create_theme_handler,
        crate::theme::handlers::get_theme_handler,
        crate::theme::handlers::update_theme_handler,
        crate::theme::handlers::delete_theme_handler,
        crate::theme::handlers::get_theme_css_handler,
        crate::theme::handlers::set_project_theme_handler,

        // Usage endpoints
        crate::usage::handlers::list_usage_plans_handler,
        crate::usage::handlers::get_usage_plan_handler,
        crate::usage::handlers::list_usage_meters_handler,
        crate::usage::handlers::get_usage_meter_handler,
        crate::usage::handlers::list_usage_entitlements_handler,
        crate::usage::handlers::get_organization_usage_plan_handler,
        crate::usage::handlers::update_organization_usage_plan_handler,
        crate::usage::handlers::track_usage_event_handler,
        crate::usage::handlers::get_usage_summary_handler,

    ),
    info(
        title = "CMS API",
        version = "1.0.0",
        description = "CMS REST API for managing documentation projects, pages, and deployments",
    ),
    servers(
        (url = "https://api.cms.com", description = "Production"),
        (url = "http://localhost:3000", description = "Development"),
    ),
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = []),
        ("cookieAuth" = []),
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "orgs", description = "Organization management"),
        (name = "projects", description = "Project management"),
        (name = "pages", description = "Page management"),
        (name = "branches", description = "Branch management"),
        (name = "languages", description = "Language management"),
        (name = "git", description = "Git integration"),
        (name = "integrations", description = "Third-party integrations"),
        (name = "deployments", description = "Deployment management"),
        (name = "domains", description = "Custom domain management"),
        (name = "reader-access", description = "Reader access management"),
        (name = "comments", description = "Page comments"),
        (name = "search", description = "Search functionality"),
        (name = "export", description = "Export functionality"),
        (name = "openapi", description = "OpenAPI document management"),
        (name = "usage", description = "Usage tracking and plans"),
        (name = "notifications", description = "User notifications"),
        (name = "assets", description = "Asset management"),
        (name = "analytics", description = "Analytics tracking"),
        (name = "platform-events", description = "Platform event log"),
        (name = "themes", description = "Theme and styling management"),
        (name = "admin", description = "System administration"),
        (name = "mcp", description = "Model Context Protocol endpoints"),
        (name = "public", description = "Public access endpoints")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// Security addon to add security schemes to OpenAPI documentation
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();

        // Add Bearer JWT authentication
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );

        // Add API Key authentication
        components.add_security_scheme(
            "apiKeyAuth",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-API-Key"))),
        );

        // Add Cookie authentication (for session-based auth)
        components.add_security_scheme(
            "cookieAuth",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("cms_session"))),
        );
    }
}

use axum::{
    http::header,
    response::{Html, IntoResponse},
};

pub async fn serve_openapi_spec() -> impl IntoResponse {
    let spec = ApiDoc::openapi().to_json().unwrap_or_default();
    ([(header::CONTENT_TYPE, "application/json")], spec)
}

pub async fn serve_openapi_yaml() -> impl IntoResponse {
    let spec = ApiDoc::openapi().to_yaml().unwrap_or_default();
    ([(header::CONTENT_TYPE, "application/yaml")], spec)
}

pub async fn serve_swagger_ui() -> impl IntoResponse {
    Html(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>CMS API - Swagger UI</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
        window.onload = () => {
            window.ui = SwaggerUIBundle({
                url: '/api-docs/openapi.json',
                dom_id: '#swagger-ui',
            });
        };
    </script>
</body>
</html>"#,
    )
}

pub async fn serve_redoc() -> impl IntoResponse {
    Html(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>CMS API - ReDoc</title>
    <link href="https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700" rel="stylesheet">
</head>
<body>
    <redoc spec-url='/api-docs/openapi.json'></redoc>
    <script src="https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"></script>
</body>
</html>"#,
    )
}
