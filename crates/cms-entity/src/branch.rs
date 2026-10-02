//! Branch entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, PaginatedResponse, Timestamp};

/// Branch entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Branch {
    pub id: Id,
    pub project_id: Id,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub is_default: bool,
    pub is_protected: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Branch create request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateBranchRequest {
    pub project_id: Id,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub is_protected: bool,
}

/// Branch update request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateBranchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_protected: Option<bool>,
}

/// Branch response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BranchResponse {
    pub id: Id,
    pub project_id: Id,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub is_default: bool,
    pub is_protected: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Branch> for BranchResponse {
    fn from(branch: Branch) -> Self {
        Self {
            id: branch.id,
            project_id: branch.project_id,
            name: branch.name,
            slug: branch.slug,
            description: branch.description,
            is_default: branch.is_default,
            is_protected: branch.is_protected,
            created_at: branch.created_at,
            updated_at: branch.updated_at,
        }
    }
}

/// Branch with project information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BranchWithProjectResponse {
    #[serde(flatten)]
    pub branch: BranchResponse,
    pub project: crate::project::ProjectResponse,
}

/// List branches query parameters
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListBranchesQuery {
    #[serde(default)]
    pub project_id: Id,
    #[serde(default)]
    pub search: Option<String>,
}

/// List branches response
pub type ListBranchesResponse = PaginatedResponse<BranchResponse>;

/// Set default branch request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct SetDefaultBranchRequest {
    pub branch_id: Id,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_response_conversion() {
        let branch = Branch {
            id: "branch-1".to_string(),
            project_id: "proj-1".to_string(),
            name: "main".to_string(),
            slug: "main".to_string(),
            description: Some("Main branch".to_string()),
            is_default: true,
            is_protected: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: BranchResponse = branch.into();
        assert_eq!(response.id, "branch-1");
        assert!(response.is_default);
    }
}
