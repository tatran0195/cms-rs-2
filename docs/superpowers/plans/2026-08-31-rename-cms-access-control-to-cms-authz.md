# Rename cms-access-control to cms-authz Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename crate `cms-access-control` to `cms-authz`, its primary trait `AccessControl` to `Authz`, and all associated structs, functions, fields, and references across the workspace.

**Architecture:** The authorization crate provides trait-based permissions (`Authz`, `ProductionAuthz`, `NoopAuthz`) passed via `BizContext` (`ctx.authz`) to business services and `AppState`. Renaming standardizes naming with `cms-auth` and modern Rust naming conventions.

**Tech Stack:** Rust, Cargo, Tokio, sqlx, axum.

## Global Constraints

- Preserve all authorization logic, method signatures in the `Authz` trait, and unit test assertions verbatim.
- Update all Cargo workspace members and intra-workspace dependency paths cleanly.
- Ensure all crates compile cleanly with `cargo check --workspace` and pass tests with `cargo test --workspace`.

---

### Task 1: Rename Crate Directory and Update Cargo Manifests

**Files:**
- Directory: `crates/cms-access-control` $\rightarrow$ `crates/cms-authz`
- Modify: `crates/cms-authz/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `crates/cms-api/Cargo.toml`
- Modify: `crates/cms-biz/Cargo.toml`
- Modify: `crates/cms-middleware/Cargo.toml`
- Modify: `crates/cms-mcp/Cargo.toml`

**Interfaces:**
- Produces: Crate `cms-authz` recognized by Cargo workspace and dependencies.

- [ ] **Step 1: Move directory to `crates/cms-authz` using git**

Run: `git mv crates/cms-access-control crates/cms-authz`

- [ ] **Step 2: Update `crates/cms-authz/Cargo.toml` package name**

```toml
[package]
name = "cms-authz"
version = "0.1.0"
edition = "2021"

[dependencies]
async-trait = { workspace = true }
cms-db = { workspace = true }
cms-entity = { workspace = true }
cms-error = { workspace = true }
tokio = { workspace = true }
```

- [ ] **Step 3: Update root `Cargo.toml`**

Replace `"crates/cms-access-control"` with `"crates/cms-authz"` in `members`, and replace `cms-access-control = { path = "crates/cms-access-control" }` with `cms-authz = { path = "crates/cms-authz" }` in `[workspace.dependencies]`.

- [ ] **Step 4: Update consumer `Cargo.toml` files**

In `crates/cms-api/Cargo.toml`, `crates/cms-biz/Cargo.toml`, `crates/cms-middleware/Cargo.toml`, and `crates/cms-mcp/Cargo.toml`:
Replace `cms-access-control = { path = "../cms-access-control" }` with `cms-authz = { path = "../cms-authz" }`.

- [ ] **Step 5: Commit changes**

```bash
git add Cargo.toml Cargo.lock crates/
git commit -m "refactor(authz): rename crate cms-access-control to cms-authz"
```

---

### Task 2: Refactor `cms-authz` Crate Types and Tests

**Files:**
- Modify: `crates/cms-authz/src/lib.rs`

**Interfaces:**
- Consumes: `cms-db`, `cms-entity`, `cms-error`
- Produces:
  - `pub trait Authz: Send + Sync`
  - `pub struct ProductionAuthz`
  - `pub struct NoopAuthz`
  - `pub fn create_authz(pool: PgPool) -> Result<Arc<dyn Authz>, AppError>`

- [ ] **Step 1: Refactor `crates/cms-authz/src/lib.rs`**

Update `crates/cms-authz/src/lib.rs` with `Authz`, `ProductionAuthz`, `NoopAuthz`, `create_authz`, and update the unit tests:

```rust
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
}

impl ProductionAuthz {
    /// Create a new ProductionAuthz
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
        use cms_db::org::MemberQueries;
        let memberships = MemberQueries::get_by_user(&self.pool, user_id).await?;
        let is_admin = memberships
            .iter()
            .any(|m| matches!(m.role, MemberRole::Owner | MemberRole::Admin));
        if !is_admin {
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
```

- [ ] **Step 2: Run tests in `cms-authz`**

Run: `cargo test -p cms-authz`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/cms-authz/src/lib.rs
git commit -m "refactor(authz): update Authz trait, ProductionAuthz, and NoopAuthz"
```

---

### Task 3: Refactor `cms-biz` Crate to use `cms-authz` and `ctx.authz`

**Files:**
- Modify: `crates/cms-biz/src/lib.rs`
- Modify: `crates/cms-biz/src/integration.rs`
- Modify: `crates/cms-biz/src/openapi.rs`
- Modify: all service files in `crates/cms-biz/src/` (`analytics.rs`, `asset.rs`, `branch.rs`, `comment.rs`, `deployment.rs`, `domain.rs`, `entitlement.rs`, `export.rs`, `git.rs`, `language.rs`, `mcp.rs`, `org.rs`, `page.rs`, `platform_event.rs`, `project.rs`, `reader_access.rs`, `search.rs`, `theme.rs`, `usage.rs`)

**Interfaces:**
- Consumes: `cms_authz::Authz`
- Produces: `BizContext { pub pool: PgPool, pub authz: Arc<dyn cms_authz::Authz> }`

- [ ] **Step 1: Update `BizContext` definition in `crates/cms-biz/src/lib.rs`**

```rust
pub struct BizContext {
    pub pool: PgPool,
    pub authz: std::sync::Arc<dyn cms_authz::Authz>,
}

impl BizContext {
    pub fn new(
        pool: PgPool,
        authz: std::sync::Arc<dyn cms_authz::Authz>,
    ) -> Self {
        Self {
            pool,
            authz,
        }
    }
}
```

- [ ] **Step 2: Update imports in `integration.rs` and `openapi.rs`**

Replace `use cms_access_control::AccessControl;` with `use cms_authz::Authz;`.

- [ ] **Step 3: Update `ctx.access_control` calls to `ctx.authz` across all `cms-biz` files**

Replace all occurrences of `ctx.access_control` with `ctx.authz` in:
- `analytics.rs`
- `asset.rs`
- `branch.rs`
- `comment.rs`
- `deployment.rs`
- `domain.rs`
- `entitlement.rs`
- `export.rs`
- `git.rs`
- `integration.rs`
- `language.rs`
- `mcp.rs`
- `openapi.rs`
- `org.rs`
- `page.rs`
- `platform_event.rs`
- `project.rs`
- `reader_access.rs`
- `search.rs`
- `theme.rs`
- `usage.rs`

- [ ] **Step 4: Check compilation and tests of `cms-biz`**

Run: `cargo check -p cms-biz`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cms-biz/
git commit -m "refactor(biz): use cms-authz and update BizContext.authz"
```

---

### Task 4: Refactor `cms-middleware`, `cms-mcp`, and `cms-api`

**Files:**
- Modify: `crates/cms-middleware/src/app_state.rs`
- Modify: `crates/cms-mcp/src/lib.rs`
- Modify: `crates/cms-api/src/auth/middleware.rs` (if any references exist)
- Modify: `crates/cms-api/src/admin/handlers.rs` (if any references exist)
- Modify: `crates/cms-api/src/org/handlers.rs` (if any references exist)

**Interfaces:**
- Consumes: `cms_authz::*`

- [ ] **Step 1: Update `crates/cms-middleware/src/app_state.rs`**

```rust
        let authz = Arc::new(cms_authz::ProductionAuthz::new(
            pool.clone(),
        ));
        let biz_context = BizContext::new(pool, authz);
```

- [ ] **Step 2: Update `crates/cms-mcp/src/lib.rs`**

Replace `Arc::new(cms_access_control::NoopAccessControl)` with `Arc::new(cms_authz::NoopAuthz)`.

- [ ] **Step 3: Update `crates/cms-api` references if any**

Verify any imports and handlers in `crates/cms-api`.

- [ ] **Step 4: Run workspace check**

Run: `cargo check --workspace`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cms-middleware/ crates/cms-mcp/ crates/cms-api/
git commit -m "refactor: update middleware, mcp, and api to use cms-authz"
```

---

### Task 5: Documentation & Final Workspace Verification

**Files:**
- Modify: `README.md`
- Modify: `docs/IMPLEMENTATION_SUMMARY.md`

- [ ] **Step 1: Update documentation files**

Update references from `cms-access-control` to `cms-authz` in `README.md` and `docs/IMPLEMENTATION_SUMMARY.md`.

- [ ] **Step 2: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add README.md docs/
git commit -m "docs: update crate references to cms-authz"
```
