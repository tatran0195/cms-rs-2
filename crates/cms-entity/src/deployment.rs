//! Deployment entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, Timestamp};

/// Deployment status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "\"DeploymentStatus\"", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeploymentStatus {
    Pending,
    Building,
    Deploying,
    Active,
    Failed,
    Deleted,
}

/// Deployment entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Deployment {
    pub id: Id,
    pub project_id: Id,
    pub branch_id: Option<Id>,
    pub status: DeploymentStatus,
    pub build_logs: Option<String>,
    pub error_message: Option<String>,
    pub deployed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Deployment response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DeploymentResponse {
    pub id: Id,
    pub project_id: Id,
    pub branch_id: Option<Id>,
    pub status: DeploymentStatus,
    pub build_logs: Option<String>,
    pub error_message: Option<String>,
    pub deployed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Deployment> for DeploymentResponse {
    fn from(deployment: Deployment) -> Self {
        Self {
            id: deployment.id,
            project_id: deployment.project_id,
            branch_id: deployment.branch_id,
            status: deployment.status,
            build_logs: deployment.build_logs,
            error_message: deployment.error_message,
            deployed_at: deployment.deployed_at,
            created_at: deployment.created_at,
            updated_at: deployment.updated_at,
        }
    }
}

/// Create deployment request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateDeploymentRequest {
    pub project_id: Id,
    #[serde(default)]
    pub branch_id: Option<Id>,
}

/// Update deployment request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateDeploymentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<DeploymentStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_logs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<Id>,
}

/// List deployments query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListDeploymentsQuery {
    #[serde(default)]
    pub project_id: Option<Id>,
    #[serde(default)]
    pub status: Option<DeploymentStatus>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

/// Immutable content captured by a successful deployment. Public readers use
/// this instead of mutable editor tables so edits are only visible after the
/// next successful release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentSnapshotContent {
    pub project: crate::project::Project,
    pub branch: crate::branch::Branch,
    pub branches: Vec<crate::branch::Branch>,
    pub languages: Vec<crate::language::Language>,
    pub translations: Vec<crate::language::ProjectTranslation>,
    #[serde(default)]
    pub openapi: Option<String>,
    pub pages: Vec<crate::page::Page>,
}
