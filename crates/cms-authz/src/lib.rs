//! CMS Authz (Authorization)
//!
//! This crate provides trait-based authorization,
//! with CMS's actual enumerable rules instead of Casbin.
//!
//! CMS's authorization rules are:
//! - Organization membership check
//! - Project role threshold check
//! - Reader audience grant check
//!
//! These are simple enough that a general-purpose policy engine would add
//! indirection without adding capability.

use std::sync::Arc;

use async_trait::async_trait;
use cms_db::PgPool;
use cms_entity::common::MemberRole;
use cms_error::AppError;

/// Authorization trait
///
/// This trait defines the interface for authorization checks.
/// Implementations can be swapped via Arc<dyn Authz> in AppState.
#[async_trait]
pub trait Authz: Send + Sync {
    /// Require that the user is a member of the organization
    async fn require_org_member(&self, user_id: &str, org_id: &str) -> Result<(), AppError>;

    /// Require that the user has at least the specified role in the project
    async fn require_project_role(
        &self,
        user_id: &str,
        project_id: &str,
        min_role: MemberRole,
    ) -> Result<(), AppError>;

    /// Require that the reader has a grant for the audience
    async fn require_audience_grant(
        &self,
        reader_id: &str,
        project_id: &str,
    ) -> Result<(), AppError>;

    /// Require that the reader has a grant for a specific branch
    async fn require_branch_grant(
        &self,
        reader_id: &str,
        project_id: &str,
        branch_id: &str,
    ) -> Result<(), AppError>;

    /// Require that the user is the owner of the organization
    async fn require_org_owner(&self, user_id: &str, org_id: &str) -> Result<(), AppError>;

    /// Require that the user is an admin of the organization
    async fn require_org_admin(&self, user_id: &str, org_id: &str) -> Result<(), AppError>;

    /// Require that the user has any access to the project (Guest level or above)
    async fn require_project_access(
        &self,
        user_id: &str,
        project_id: &str,
    ) -> Result<(), AppError> {
        self.require_project_role(user_id, project_id, MemberRole::Guest)
            .await
    }

    /// Require that the user is a member of the project (Member level or above)
    async fn require_project_member(
        &self,
        user_id: &str,
        project_id: &str,
    ) -> Result<(), AppError> {
        self.require_project_role(user_id, project_id, MemberRole::Member)
            .await
    }

    /// Require that the user has system administrative privileges
    async fn require_system_admin(&self, user_id: &str) -> Result<(), AppError>;
}

/// Production implementation of Authz
pub struct ProductionAuthz {
    pool: PgPool,
    system_admin_emails: Vec<String>,
}

impl ProductionAuthz {
    /// Create a new ProductionAuthz
    pub fn new(pool: PgPool) -> Self {
        Self::new_with_admin_emails(pool, Vec::new())
    }

    /// Construct production authorization with the operator email allow-list.
    pub fn new_with_admin_emails(pool: PgPool, system_admin_emails: Vec<String>) -> Self {
        Self {
            pool,
            system_admin_emails: system_admin_emails
                .into_iter()
                .map(|email| email.trim().to_ascii_lowercase())
                .filter(|email| !email.is_empty())
                .collect(),
        }
    }

    /// Get a user's role in an organization
    async fn get_org_role(
        &self,
        user_id: &str,
        org_id: &str,
    ) -> Result<Option<MemberRole>, AppError> {
        use cms_db::org::MemberQueries;

        let member = MemberQueries::get_by_user_and_org(&self.pool, user_id, org_id).await?;

        Ok(member.map(|m| m.role))
    }

    /// Get a user's role in a project
    async fn get_project_role(
        &self,
        user_id: &str,
        project_id: &str,
    ) -> Result<Option<MemberRole>, AppError> {
        // This would query the project membership
        // For now, we'll use organization membership as a proxy
        // In practice, CMS has project-specific roles
        use cms_db::project::ProjectQueries;

        let project = ProjectQueries::get_by_id(&self.pool, project_id)
            .await?
            .ok_or(AppError::NotFound("Project not found".to_string()))?;

        let org_id = project.organization_id;
        self.get_org_role(user_id, &org_id).await
    }

    /// Check if a reader has a grant for an audience
    async fn has_audience_grant(
        &self,
        reader_id: &str,
        project_id: &str,
    ) -> Result<bool, AppError> {
        use cms_db::reader_access::ReaderAudienceQueries;

        let has_grant =
            ReaderAudienceQueries::has_grant_for_project(&self.pool, reader_id, project_id).await?;

        Ok(has_grant)
    }
}

#[async_trait]
impl Authz for ProductionAuthz {
    async fn require_org_member(&self, user_id: &str, org_id: &str) -> Result<(), AppError> {
        let role = self.get_org_role(user_id, org_id).await?;

        if role.is_none() {
            return Err(AppError::AccessDenied(
                "User is not a member of this organization".to_string(),
            ));
        }

        Ok(())
    }

    async fn require_project_role(
        &self,
        user_id: &str,
        project_id: &str,
        min_role: MemberRole,
    ) -> Result<(), AppError> {
        let role = self.get_project_role(user_id, project_id).await?;

        if let Some(user_role) = role {
            if user_role >= min_role {
                return Ok(());
            }
        }

        Err(AppError::InsufficientRole(format!(
            "User requires at least {:?} role for this project",
            min_role
        )))
    }

    async fn require_audience_grant(
        &self,
        reader_id: &str,
        project_id: &str,
    ) -> Result<(), AppError> {
        let has_grant = self.has_audience_grant(reader_id, project_id).await?;

        if !has_grant {
            return Err(AppError::AccessDenied(
                "Reader does not have access to this project".to_string(),
            ));
        }

        Ok(())
    }

    async fn require_branch_grant(
        &self,
        reader_id: &str,
        project_id: &str,
        branch_id: &str,
    ) -> Result<(), AppError> {
        // Check if reader has grant for this specific branch
        use cms_db::reader_access::AudienceGrantQueries;

        let has_grant = AudienceGrantQueries::has_grant_for_branch(
            &self.pool, reader_id, project_id, branch_id,
        )
        .await?;

        if !has_grant {
            return Err(AppError::AccessDenied(
                "Reader does not have access to this branch".to_string(),
            ));
        }

        Ok(())
    }

    async fn require_org_owner(&self, user_id: &str, org_id: &str) -> Result<(), AppError> {
        let role = self.get_org_role(user_id, org_id).await?;

        if role != Some(MemberRole::Owner) {
            return Err(AppError::InsufficientRole(
                "User must be the organization owner".to_string(),
            ));
        }

        Ok(())
    }

    async fn require_org_admin(&self, user_id: &str, org_id: &str) -> Result<(), AppError> {
        let role = self.get_org_role(user_id, org_id).await?;

        match role {
            Some(MemberRole::Owner) | Some(MemberRole::Admin) => Ok(()),
            _ => Err(AppError::InsufficientRole(
                "User must be an organization admin".to_string(),
            )),
        }
    }

    async fn require_system_admin(&self, user_id: &str) -> Result<(), AppError> {
        let user = cms_db::auth::UserQueries::get_by_id(&self.pool, user_id)
            .await?
            .ok_or(AppError::Forbidden)?;
        let database_admin = cms_db::auth::UserQueries::is_system_admin(&self.pool, user_id).await?;
        let configured_admin = self
            .system_admin_emails
            .iter()
            .any(|email| email == &user.email.trim().to_ascii_lowercase());
        if !database_admin && !configured_admin {
            return Err(AppError::Forbidden);
        }
        Ok(())
    }
}

/// No-op authorization for testing
pub struct NoopAuthz;

#[async_trait]
impl Authz for NoopAuthz {
    async fn require_org_member(&self, _user_id: &str, _org_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn require_project_role(
        &self,
        _user_id: &str,
        _project_id: &str,
        _min_role: MemberRole,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn require_audience_grant(
        &self,
        _reader_id: &str,
        _project_id: &str,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn require_branch_grant(
        &self,
        _reader_id: &str,
        _project_id: &str,
        _branch_id: &str,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn require_org_owner(&self, _user_id: &str, _org_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn require_org_admin(&self, _user_id: &str, _org_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn require_system_admin(&self, _user_id: &str) -> Result<(), AppError> {
        Ok(())
    }
}

/// Create an authorization implementation based on configuration
pub fn create_authz(pool: PgPool) -> Result<Arc<dyn Authz>, AppError> {
    // For now, we always use the production implementation
    // In the future, this could be configured
    Ok(Arc::new(ProductionAuthz::new(pool)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_member_role_ordering() {
        assert!(MemberRole::Owner > MemberRole::Admin);
        assert!(MemberRole::Admin > MemberRole::Member);
        assert!(MemberRole::Member > MemberRole::Guest);
        assert!(MemberRole::Owner >= MemberRole::Owner);
        assert!(MemberRole::Admin >= MemberRole::Member);
    }

    #[test]
    fn test_noop_authz() {
        let authz = NoopAuthz;

        // All checks should pass
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            authz.require_org_member("user-1", "org-1").await.unwrap();
            authz
                .require_project_role("user-1", "proj-1", MemberRole::Admin)
                .await
                .unwrap();
            authz
                .require_audience_grant("reader-1", "proj-1")
                .await
                .unwrap();
            authz.require_org_owner("user-1", "org-1").await.unwrap();
        });
    }
}
