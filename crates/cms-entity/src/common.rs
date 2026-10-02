//! Common types used across the application

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Base identifier type for most entities
pub type Id = String;

/// Timestamp type
pub type Timestamp = DateTime<Utc>;

/// UUID type alias
pub type UuidString = String;

/// Pagination request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct PaginationRequest {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

fn default_page() -> u64 {
    1
}
fn default_page_size() -> u64 {
    20
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: u64, page: u64, page_size: u64) -> Self {
        let total_pages = if page_size > 0 {
            total.div_ceil(page_size)
        } else {
            0
        };

        Self {
            data,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}

/// Sort order
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

/// Sort direction for queries
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SortRequest {
    pub field: String,
    #[serde(default)]
    pub order: SortOrder,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
}

impl HealthResponse {
    pub fn ok(version: &str) -> Self {
        Self {
            status: "ok".to_string(),
            timestamp: Utc::now(),
            version: version.to_string(),
        }
    }
}

/// API response envelope (for non-paginated responses)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApiResponse<T> {
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

/// Empty response for DELETE and other operations that don't return data
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EmptyResponse;

/// Success message response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SuccessResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl SuccessResponse {
    pub fn ok() -> Self {
        Self {
            success: true,
            message: None,
        }
    }

    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: Some(message.into()),
        }
    }
}

/// Member role in an organization
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    utoipa::ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum MemberRole {
    Guest,
    /// Viewer = read-only access (alias for Guest level in access checks)
    Viewer,
    /// Editor = full content edit (alias for Member level in access checks)
    Editor,
    #[default]
    Member,
    Admin,
    Owner,
}

impl sqlx::Type<sqlx::Postgres> for MemberRole {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("\"MemberRole\"")
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for MemberRole {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        match s {
            "OWNER" | "owner" => Ok(MemberRole::Owner),
            "ADMIN" | "admin" => Ok(MemberRole::Admin),
            "MEMBER" | "member" => Ok(MemberRole::Member),
            "GUEST" | "guest" => Ok(MemberRole::Guest),
            "VIEWER" | "viewer" => Ok(MemberRole::Viewer),
            "EDITOR" | "editor" => Ok(MemberRole::Editor),
            other => Err(format!("unknown MemberRole: {}", other).into()),
        }
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Postgres> for MemberRole {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = match self {
            MemberRole::Owner => "OWNER",
            MemberRole::Admin => "ADMIN",
            MemberRole::Member | MemberRole::Editor => "MEMBER",
            MemberRole::Guest | MemberRole::Viewer => "GUEST",
        };
        <&str as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&s, buf)
    }
}

/// Project role
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProjectRole {
    Admin,
    Editor,
    #[default]
    Viewer,
}

/// Entity audit information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuditInfo {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<Id>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<Id>,
}

impl AuditInfo {
    pub fn new(created_by: Option<Id>, updated_by: Option<Id>) -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            created_by,
            updated_by,
        }
    }
}

/// Filter operators for query parameters
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum FilterOperator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    StartsWith,
    EndsWith,
    In,
    NotIn,
    IsNull,
    IsNotNull,
}

/// Filter condition for queries
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FilterCondition {
    pub field: String,
    pub operator: FilterOperator,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination() {
        let items: Vec<i32> = (0..100).collect();
        let response = PaginatedResponse::new(items, 100, 1, 10);

        assert_eq!(response.total, 100);
        assert_eq!(response.page, 1);
        assert_eq!(response.page_size, 10);
        assert_eq!(response.total_pages, 10);
    }

    #[test]
    fn test_member_role_serialization() {
        let role = MemberRole::Owner;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"owner\"");

        let deserialized: MemberRole = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, MemberRole::Owner);
    }
}
