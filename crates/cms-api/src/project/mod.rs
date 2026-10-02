//! Project API module
//!
//! This module contains handlers for project routes.

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use cms_middleware::app_state::AppState;

pub mod handlers;

use handlers::*;

use crate::extractors::UserId;

/// Create the project router
pub fn router(state: Arc<AppState>) -> Router {
    use axum::routing::patch;
    Router::new()
        .route("/", get(list_projects_handler))
        .route("/", post(create_project_handler))
        .route("/{id}", get(get_project_handler))
        .route("/{id}", put(update_project_handler))
        .route("/{id}", patch(update_project_handler))
        .route("/{id}", delete(delete_project_handler))
        // Settings routes
        .route("/{id}/settings", get(get_project_settings_handler))
        .route("/{id}/settings", put(update_project_settings_handler))
        .route("/{id}/settings", patch(update_project_settings_handler))
        // Addons routes
        .route("/{id}/addons", get(list_project_addons_handler))
        // Project pages subroutes
        .route("/{project_id}/pages", get(list_project_pages_handler))
        .route("/{project_id}/pages", post(create_project_page_handler))
        .route("/{project_id}/pages/reorder", post(reorder_project_pages_handler))
        .route("/{project_id}/pages/{id}", get(get_project_page_handler))
        .route("/{project_id}/pages/{id}", put(update_project_page_handler))
        .route("/{project_id}/pages/{id}", patch(update_project_page_handler))
        .route("/{project_id}/pages/{id}", delete(delete_project_page_handler))
        // Project branches subroutes
        .route("/{project_id}/branches", get(list_project_branches_handler))
        .route("/{project_id}/branches", post(create_project_branch_handler))
        // Project languages subroutes
        .route("/{project_id}/languages", get(list_project_languages_handler))
        .route("/{project_id}/languages", post(create_project_language_handler))
        .route("/{project_id}/languages/{id}", patch(update_project_language_handler))
        .route("/{project_id}/languages/{id}", put(update_project_language_handler))
        .route("/{project_id}/languages/{id}", delete(delete_project_language_handler))
        // Project deployments subroutes
        .route("/{project_id}/deployments", get(list_project_deployments_handler))
        // Project domains subroutes
        .route("/{project_id}/domains", get(list_project_domains_handler))
        // Project assets subroutes
        .route("/{project_id}/assets", get(list_project_assets_handler))
        .route("/{project_id}/assets/presign", post(presign_project_asset_handler))
        .route("/{project_id}/assets/confirm", post(confirm_project_asset_handler))
        // Project analytics
        .route("/{project_id}/analytics", get(get_project_analytics_handler))
        // Project settings extended
        .route("/{project_id}/settings/usage", get(get_project_settings_usage_handler))
        .route("/{project_id}/settings/search", get(get_project_search_settings_handler))
        .route("/{project_id}/settings/search", patch(update_project_search_settings_handler))
        .route("/{project_id}/settings/search/diagnostics", get(get_project_search_diagnostics_handler))
        .route("/{project_id}/settings/search/reindex", post(reindex_project_search_handler))
        .route("/{project_id}/settings/git/import", post(action_project_git_handler))
        .route("/{project_id}/settings/git/webhook-secret", post(action_project_git_handler))
        .route("/{project_id}/settings/import/mintlify", post(action_project_git_handler))
        .route("/{project_id}/settings/import/ghost", post(action_project_git_handler))
        // Branch merge
        .route("/{project_id}/branches/{branch_id}/merge", post(merge_project_branch_handler))
        .route("/{project_id}/branches/{branch_id}", delete(delete_project_branch_handler))
        // Deployments extended
        .route("/{project_id}/deployments", post(create_project_deployment_handler))
        .route("/{project_id}/deployments/latest", get(get_latest_project_deployment_handler))
        .route("/{project_id}/deployments/changes", get(get_deployment_changes_handler))
        .route("/{project_id}/deployments/{id}/rollback", post(rollback_deployment_handler))
        // Domains extended
        .route("/{project_id}/domains", post(add_project_domain_handler))
        .route("/{project_id}/domains/{id}", delete(delete_project_domain_handler))
        .route("/{project_id}/domains/{id}/verify", post(verify_project_domain_handler))
        .route("/{project_id}/domains/{id}/primary", post(set_primary_project_domain_handler))
        // Project members
        .route("/{project_id}/members", get(list_project_members_handler))
        .route("/{project_id}/members/invite", post(invite_project_member_handler))
        .route("/{project_id}/members/{id}", delete(remove_project_member_handler))
        .route("/{project_id}/members/{id}/role", patch(update_project_member_role_handler))
        .route("/{project_id}/members/transfer-ownership", post(transfer_project_ownership_handler))
        .route("/{project_id}/members/invitations/{id}", delete(cancel_project_invitation_handler))
        // Project comments
        .route("/{project_id}/comments", get(list_project_comments_handler))
        .route("/{project_id}/comments", post(create_project_comment_handler))
        .route("/{project_id}/comments/{id}", patch(update_project_comment_handler))
        .route("/{project_id}/comments/{id}", delete(delete_project_comment_handler))
        // Project OpenAPI
        .route("/{project_id}/openapi", get(get_project_openapi_handler))
        .route("/{project_id}/openapi", put(save_project_openapi_handler))
        .route("/{project_id}/openapi", delete(delete_project_openapi_handler))
        .route("/{project_id}/openapi/sync", post(sync_project_openapi_handler))
        // Project reader access
        .route("/{project_id}/reader-access", get(get_project_reader_access_handler))
        .route("/{project_id}/reader-access/mode", put(update_project_reader_access_handler))
        .route("/{project_id}/reader-access/audiences", post(create_reader_audience_handler))
        .route("/{project_id}/reader-access/audiences/{audience_id}", delete(delete_reader_audience_handler))
        .route("/{project_id}/reader-access/readers/invite", post(invite_project_reader_handler))
        .route("/{project_id}/reader-access/readers/{reader_id}/revoke", post(revoke_project_reader_handler))
        .route("/{project_id}/reader-access/jwt", put(configure_reader_jwt_handler))
        .route("/{project_id}/reader-access/jwt/test", post(test_reader_jwt_handler))
        .route("/{project_id}/reader-access/emergency-revoke", post(emergency_revoke_reader_handler))
        // Project git
        .route("/{project_id}/git", get(get_project_git_status_handler))
        .route("/{project_id}/git/authorize", post(action_project_git_handler))
        .route("/{project_id}/git/connection", put(action_project_git_handler))
        .route("/{project_id}/git/connection", delete(delete_project_git_connection_handler))
        .route("/{project_id}/git/operations", post(action_project_git_handler))
        .route("/{project_id}/git/webhook-secret", post(action_project_git_handler))
        .route("/{project_id}/git/conflicts/{conflict_id}/resolve", post(resolve_project_git_conflict_handler))
        // Project integrations
        .route("/{project_id}/integrations", get(list_project_integrations_handler))
        .route("/{project_id}/integrations", post(create_project_integration_handler))
        .route("/{project_id}/integrations/{provider_id}", patch(update_project_integration_handler))
        .route("/{project_id}/integrations/{provider_id}", delete(delete_project_integration_handler))
        .route("/{project_id}/integrations/{provider_id}/verify", post(verify_project_integration_handler))
        .route("/{project_id}/integrations/{provider_id}/delete-confirmation", post(delete_project_integration_confirmation_handler))
        // Project AI
        .route("/{project_id}/ai", post(action_project_ai_handler))
        // Project theme template
        .route("/{project_id}/theme-template", get(get_project_theme_template_handler))
        .route("/{project_id}/theme-template/import", post(import_project_theme_template_handler))
        .route("/{project_id}/theme-repository", get(get_project_theme_repository_handler))
        .route("/{project_id}/export", get(get_project_export_download_handler))
        // Project exports
        .route("/{project_id}/exports", get(list_project_exports_handler))
        .route("/{project_id}/exports", post(trigger_project_export_handler))
        .route("/{project_id}/exports/schedules", get(list_project_export_schedules_handler))
        .route("/{project_id}/exports/schedules", post(create_project_export_schedule_handler))
        .route("/{project_id}/exports/{id}/cancel", post(cancel_project_export_handler))
        .route("/{project_id}/exports/{id}/artifacts/{artifact_id}/download", get(download_project_export_artifact_handler))
        .route("/{project_id}/exports/schedules/{schedule_id}", patch(update_project_export_schedule_handler))
        .route("/{project_id}/exports/schedules/{schedule_id}/run", post(run_project_export_schedule_handler))
        // Project API keys
        .route("/{project_id}/api-keys", get(list_project_api_keys_handler))
        .route("/{project_id}/api-keys", post(create_project_api_key_handler))
        .route("/{project_id}/api-keys/{id}", delete(delete_project_api_key_handler))
        .route("/{project_id}/api-keys/{id}/rotate", post(rotate_project_api_key_handler))
        // Project Addons extended
        .route("/{project_id}/addons/{addon_id}", patch(update_project_addon_handler))
        .route("/{project_id}/addons/{addon_id}/activate", post(activate_project_addon_handler))
        .route("/{project_id}/addons/{addon_id}/deactivate", post(deactivate_project_addon_handler))
        .with_state(state)
}
