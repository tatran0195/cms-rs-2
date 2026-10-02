//! Organization entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::common::{Id, MemberRole, PaginatedResponse, Timestamp};

/// Organization entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Organization {
    pub id: Id,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Organization create request
#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
pub struct CreateOrganizationRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Organization name must be between 1 and 100 characters"
    ))]
    pub name: String,
    #[serde(default)]
    #[validate(length(max = 500, message = "Description must be at most 500 characters"))]
    pub description: Option<String>,
}

/// Organization update request
#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
pub struct UpdateOrganizationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(
        min = 1,
        max = 100,
        message = "Organization name must be between 1 and 100 characters"
    ))]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 500, message = "Description must be at most 500 characters"))]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url(message = "Invalid logo URL"))]
    pub logo: Option<String>,
}

/// Organization response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationResponse {
    pub id: Id,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Organization> for OrganizationResponse {
    fn from(org: Organization) -> Self {
        Self {
            id: org.id,
            name: org.name,
            slug: org.slug,
            description: org.description,
            logo: org.logo,
            created_at: org.created_at,
            updated_at: org.updated_at,
        }
    }
}

/// Member entity (user's membership in an organization)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Member {
    pub id: Id,
    pub user_id: Id,
    pub organization_id: Id,
    pub role: MemberRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Member response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MemberResponse {
    pub id: Id,
    pub user_id: Id,
    pub organization_id: Id,
    pub role: MemberRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Member> for MemberResponse {
    fn from(member: Member) -> Self {
        Self {
            id: member.id,
            user_id: member.user_id,
            organization_id: member.organization_id,
            role: member.role,
            created_at: member.created_at,
            updated_at: member.updated_at,
        }
    }
}

/// Member with user information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MemberWithUserResponse {
    #[serde(flatten)]
    pub member: MemberResponse,
    pub user: crate::auth::UserResponse,
}

/// Invitation entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Invitation {
    pub id: Id,
    pub organization_id: Id,
    pub email: String,
    pub role: MemberRole,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Invitation create request
#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
pub struct CreateInvitationRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[serde(default = "default_invitation_role")]
    pub role: MemberRole,
}

fn default_invitation_role() -> MemberRole {
    MemberRole::Member
}

/// Invitation response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InvitationResponse {
    pub id: Id,
    pub organization_id: Id,
    pub email: String,
    pub role: MemberRole,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl From<Invitation> for InvitationResponse {
    fn from(invitation: Invitation) -> Self {
        Self {
            id: invitation.id,
            organization_id: invitation.organization_id,
            email: invitation.email,
            role: invitation.role,
            expires_at: invitation.expires_at,
            created_at: invitation.created_at,
        }
    }
}

/// Accept invitation request
#[derive(Debug, Clone, Deserialize, Serialize, Validate, utoipa::ToSchema)]
pub struct AcceptInvitationRequest {
    #[validate(length(min = 1, message = "Token is required"))]
    pub token: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: Option<String>,
}

/// List members query parameters
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListMembersQuery {
    #[serde(default)]
    pub role: Option<MemberRole>,
    #[serde(default)]
    pub search: Option<String>,
}

/// List members response
pub type ListMembersResponse = PaginatedResponse<MemberWithUserResponse>;

/// List invitations response
pub type ListInvitationsResponse = PaginatedResponse<InvitationResponse>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_member_role_default() {
        let request = CreateInvitationRequest {
            email: "test@example.com".to_string(),
            role: MemberRole::Admin,
        };
        assert_eq!(request.role, MemberRole::Admin);
    }
}
