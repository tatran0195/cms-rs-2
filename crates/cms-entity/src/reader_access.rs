//! Reader access entity types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::{Id, Timestamp};

/// Reader entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Reader {
    pub id: Id,
    pub email: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Reader response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReaderResponse {
    pub id: Id,
    pub email: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Reader> for ReaderResponse {
    fn from(reader: Reader) -> Self {
        Self {
            id: reader.id,
            email: reader.email,
            name: reader.name,
            created_at: reader.created_at,
            updated_at: reader.updated_at,
        }
    }
}

/// Create reader request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateReaderRequest {
    pub email: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// Update reader request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateReaderRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Audience entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Audience {
    pub id: Id,
    pub project_id: Id,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Audience response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AudienceResponse {
    pub id: Id,
    pub project_id: Id,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Audience> for AudienceResponse {
    fn from(audience: Audience) -> Self {
        Self {
            id: audience.id,
            project_id: audience.project_id,
            name: audience.name,
            description: audience.description,
            created_at: audience.created_at,
            updated_at: audience.updated_at,
        }
    }
}

/// Create audience request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateAudienceRequest {
    pub project_id: Id,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Update audience request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateAudienceRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Reader audience entity (many-to-many between Reader and Audience)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReaderAudience {
    pub id: Id,
    pub reader_id: Id,
    pub audience_id: Id,
    pub created_at: DateTime<Utc>,
}

/// Reader audience response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReaderAudienceResponse {
    pub id: Id,
    pub reader_id: Id,
    pub audience_id: Id,
    pub created_at: DateTime<Utc>,
}

impl From<ReaderAudience> for ReaderAudienceResponse {
    fn from(ra: ReaderAudience) -> Self {
        Self {
            id: ra.id,
            reader_id: ra.reader_id,
            audience_id: ra.audience_id,
            created_at: ra.created_at,
        }
    }
}

/// Audience grant entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AudienceGrant {
    pub id: Id,
    pub audience_id: Id,
    pub project_id: Id,
    pub branch_id: Option<Id>,
    pub language_id: Option<Id>,
    pub created_at: DateTime<Utc>,
}

/// Audience grant response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AudienceGrantResponse {
    pub id: Id,
    pub audience_id: Id,
    pub project_id: Id,
    pub branch_id: Option<Id>,
    pub language_id: Option<Id>,
    pub created_at: DateTime<Utc>,
}

impl From<AudienceGrant> for AudienceGrantResponse {
    fn from(grant: AudienceGrant) -> Self {
        Self {
            id: grant.id,
            audience_id: grant.audience_id,
            project_id: grant.project_id,
            branch_id: grant.branch_id,
            language_id: grant.language_id,
            created_at: grant.created_at,
        }
    }
}

/// Create audience grant request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateAudienceGrantRequest {
    pub audience_id: Id,
    pub project_id: Id,
    #[serde(default)]
    pub branch_id: Option<Id>,
    #[serde(default)]
    pub language_id: Option<Id>,
}

/// Reader invitation entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReaderInvitation {
    pub id: Id,
    pub audience_id: Id,
    pub email: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Reader invitation response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReaderInvitationResponse {
    pub id: Id,
    pub audience_id: Id,
    pub email: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl From<ReaderInvitation> for ReaderInvitationResponse {
    fn from(invitation: ReaderInvitation) -> Self {
        Self {
            id: invitation.id,
            audience_id: invitation.audience_id,
            email: invitation.email,
            expires_at: invitation.expires_at,
            created_at: invitation.created_at,
        }
    }
}

/// Create reader invitation request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateReaderInvitationRequest {
    pub audience_id: Id,
    pub email: String,
}

/// Reader session entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReaderSession {
    pub id: Id,
    pub reader_id: Id,
    pub session_token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Reader session response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReaderSessionResponse {
    pub id: Id,
    pub reader_id: Id,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl From<ReaderSession> for ReaderSessionResponse {
    fn from(session: ReaderSession) -> Self {
        Self {
            id: session.id,
            reader_id: session.reader_id,
            expires_at: session.expires_at,
            created_at: session.created_at,
        }
    }
}

/// JWT access provider entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct JwtAccessProvider {
    pub id: Id,
    pub name: String,
    pub issuer: String,
    pub audience: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// JWT access provider response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct JwtAccessProviderResponse {
    pub id: Id,
    pub name: String,
    pub issuer: String,
    pub audience: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<JwtAccessProvider> for JwtAccessProviderResponse {
    fn from(provider: JwtAccessProvider) -> Self {
        Self {
            id: provider.id,
            name: provider.name,
            issuer: provider.issuer,
            audience: provider.audience,
            created_at: provider.created_at,
            updated_at: provider.updated_at,
        }
    }
}

/// JWT replay entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct JwtReplay {
    pub id: Id,
    pub jwt_id: String,
    pub provider_id: Id,
    pub used_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Reader audit log entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReaderAuditLog {
    pub id: Id,
    pub reader_id: Id,
    pub project_id: Id,
    pub action: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Reader audit log response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReaderAuditLogResponse {
    pub id: Id,
    pub reader_id: Id,
    pub project_id: Id,
    pub action: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl From<ReaderAuditLog> for ReaderAuditLogResponse {
    fn from(log: ReaderAuditLog) -> Self {
        Self {
            id: log.id,
            reader_id: log.reader_id,
            project_id: log.project_id,
            action: log.action,
            metadata: log.metadata,
            created_at: log.created_at,
        }
    }
}

/// List readers query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListReadersQuery {
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

/// List audiences query
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ListAudiencesQuery {
    #[serde(default)]
    pub project_id: Option<Id>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

/// Create invitation request (invites a reader to access a project)
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateInvitationRequest {
    pub email: String,
    pub project_id: Id,
    pub audience_id: Id,
    #[serde(default)]
    pub expires_in_days: Option<i64>,
}

/// Accept invitation request
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct AcceptInvitationRequest {
    pub token: String,
    #[serde(default)]
    pub name: Option<String>,
}
