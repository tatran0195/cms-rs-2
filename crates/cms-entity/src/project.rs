//! Project entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::common::{Id, PaginatedResponse, Timestamp};

/// Project entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Project {
    pub id: Id,
    pub organization_id: Id,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub is_public: bool,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Project create request
#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
pub struct CreateProjectRequest {
    #[serde(alias = "organizationId", default)]
    pub organization_id: Option<Id>,
    #[validate(length(
        min = 1,
        max = 100,
        message = "Project name must be between 1 and 100 characters"
    ))]
    pub name: String,
    #[serde(default)]
    #[validate(length(max = 500, message = "Description must be at most 500 characters"))]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(alias = "isPublic", default)]
    pub is_public: bool,
}

/// Project update request
#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
pub struct UpdateProjectRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(
        min = 1,
        max = 100,
        message = "Project name must be between 1 and 100 characters"
    ))]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(
        min = 1,
        max = 63,
        message = "Project slug must be between 1 and 63 characters"
    ))]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 500, message = "Description must be at most 500 characters"))]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(alias = "isPublic", skip_serializing_if = "Option::is_none")]
    pub is_public: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

/// Project count response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCountResponse {
    #[serde(default)]
    pub pages: i64,
    #[serde(default)]
    pub deployments: i64,
    #[serde(default)]
    pub domains: i64,
}

/// Project response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResponse {
    pub id: Id,
    #[serde(alias = "organization_id")]
    pub organization_id: Id,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(alias = "is_public")]
    pub is_public: bool,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    #[serde(alias = "created_at")]
    pub created_at: DateTime<Utc>,
    #[serde(alias = "updated_at")]
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub _count: Option<ProjectCountResponse>,
}

impl From<Project> for ProjectResponse {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            organization_id: project.organization_id,
            name: project.name,
            slug: project.slug,
            description: project.description,
            icon: project.icon,
            is_public: project.is_public,
            config: project.config,
            created_at: project.created_at,
            updated_at: project.updated_at,
            _count: Some(ProjectCountResponse {
                pages: 0,
                deployments: 0,
                domains: 0,
            }),
        }
    }
}

/// Project with organization information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProjectWithOrgResponse {
    #[serde(flatten)]
    pub project: ProjectResponse,
    pub organization: crate::org::OrganizationResponse,
}

/// Project settings (extended configuration)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProjectSettings {
    pub project_id: Id,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub default_language: Option<String>,
    #[serde(default)]
    pub custom_domain: Option<String>,
    #[serde(default)]
    pub search_enabled: bool,
    #[serde(default)]
    pub comments_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Project settings update request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateProjectSettingsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments_enabled: Option<bool>,
}

/// Project addon entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProjectAddon {
    pub id: Id,
    pub project_id: Id,
    pub addon_type: String,
    pub config: serde_json::Value,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Project addon response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProjectAddonResponse {
    pub id: Id,
    pub project_id: Id,
    pub addon_type: String,
    pub config: serde_json::Value,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ProjectAddon> for ProjectAddonResponse {
    fn from(addon: ProjectAddon) -> Self {
        Self {
            id: addon.id,
            project_id: addon.project_id,
            addon_type: addon.addon_type,
            config: addon.config,
            is_enabled: addon.is_enabled,
            created_at: addon.created_at,
            updated_at: addon.updated_at,
        }
    }
}

/// List projects query parameters
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListProjectsQuery {
    #[serde(default)]
    pub organization_id: Option<Id>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub is_public: Option<bool>,
    #[serde(default)]
    pub page: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
}

/// List projects response
pub type ListProjectsResponse = PaginatedResponse<ProjectResponse>;

/// Project audit event
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProjectAuditEvent {
    pub id: Id,
    pub project_id: Id,
    pub action: String,
    pub user_id: Option<Id>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_response_conversion() {
        let project = Project {
            id: "proj-1".to_string(),
            organization_id: "org-1".to_string(),
            name: "Test Project".to_string(),
            slug: "test-project".to_string(),
            description: Some("A test project".to_string()),
            icon: None,
            is_public: false,
            config: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: ProjectResponse = project.into();
        assert_eq!(response.id, "proj-1");
        assert_eq!(response.name, "Test Project");
        assert_eq!(response.slug, "test-project");
    }
}
