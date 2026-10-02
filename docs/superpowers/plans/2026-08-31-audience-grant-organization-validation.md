# Audience Grant Organization and Resource Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement real multi-tenant organization boundary verification, target project role checking, and branch/language scoping validation for `ReaderAccessService::create_audience_grant`.

**Architecture:** Look up the source audience's project and target project using `ProjectQueries::get_by_id`, ensure `audience_project.organization_id == target_project.organization_id`, require `MemberRole::Admin` on `target_project`, and validate that any provided `branch_id` and/or `language_id` belong to `target_project`.

**Tech Stack:** Rust, sqlx, tokio, cms-biz, cms-db, cms-entity, cms-error, cms-access-control.

## Global Constraints

- Must return `AppError::NotFound` if audience, audience's project, target project, branch, or language is not found.
- Must return `AppError::AccessDenied` if target project does not belong to the same organization as the audience's project.
- Must return `AppError::Validation` if the specified branch or language does not belong to the target project.
- Must require `MemberRole::Admin` on the target project via `ctx.access_control.require_project_role`.

---

### Task 1: Update `ReaderAccessService::create_audience_grant` in `crates/cms-biz/src/reader_access.rs`

**Files:**
- Modify: `crates/cms-biz/src/reader_access.rs:13-20`, `crates/cms-biz/src/reader_access.rs:228-260`

**Interfaces:**
- Consumes: `ProjectQueries::get_by_id`, `BranchQueries::get_by_id`, `LanguageQueries::get_by_id`, `AudienceGrantQueries::create`
- Produces: `ReaderAccessService::create_audience_grant(ctx: &BizContext, user_id: &str, audience_id: &str, project_id: &str, branch_id: Option<&str>, language_id: Option<&str>) -> Result<AudienceGrantResponse, AppError>`

- [ ] **Step 1: Update imports in `crates/cms-biz/src/reader_access.rs`**

Add `BranchQueries` and `LanguageQueries` to imports from `cms_db`:
```rust
use cms_db::{
    branch::BranchQueries,
    language::LanguageQueries,
    project::ProjectQueries,
    reader_access::{
        AudienceGrantQueries, AudienceQueries, JwtAccessProviderQueries, JwtReplayQueries,
        ReaderAudienceQueries, ReaderAuditLogQueries, ReaderInvitationQueries, ReaderQueries,
        ReaderSessionQueries,
    },
};
```

- [ ] **Step 2: Implement full validation in `create_audience_grant`**

Replace `create_audience_grant` body with:
```rust
    /// Create an audience grant
    pub async fn create_audience_grant(
        ctx: &BizContext,
        user_id: &str,
        audience_id: &str,
        project_id: &str,
        branch_id: Option<&str>,
        language_id: Option<&str>,
    ) -> Result<AudienceGrantResponse, AppError> {
        let audience = AudienceQueries::get_by_id(&ctx.pool, audience_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Audience not found".to_string()))?;

        // Check if user has admin role in the audience's source project
        ctx.access_control
            .require_project_role(user_id, &audience.project_id, MemberRole::Admin)
            .await?;

        // Fetch source project
        let audience_project = ProjectQueries::get_by_id(&ctx.pool, &audience.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Audience project not found".to_string()))?;

        // Fetch target project
        let target_project = ProjectQueries::get_by_id(&ctx.pool, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Target project not found".to_string()))?;

        // Verify target project belongs to the same organization as the audience project
        if audience_project.organization_id != target_project.organization_id {
            return Err(AppError::AccessDenied(
                "Audience and target project must belong to the same organization".to_string(),
            ));
        }

        // Check if user has admin role in the target project
        ctx.access_control
            .require_project_role(user_id, project_id, MemberRole::Admin)
            .await?;

        // Verify branch belongs to the target project if specified
        if let Some(bid) = branch_id {
            let branch = BranchQueries::get_by_id(&ctx.pool, bid)
                .await?
                .ok_or_else(|| AppError::NotFound("Branch not found".to_string()))?;
            if branch.project_id != target_project.id {
                return Err(AppError::Validation(
                    "Branch does not belong to the target project".to_string(),
                ));
            }
        }

        // Verify language belongs to the target project if specified
        if let Some(lid) = language_id {
            let language = LanguageQueries::get_by_id(&ctx.pool, lid)
                .await?
                .ok_or_else(|| AppError::NotFound("Language not found".to_string()))?;
            if language.project_id != target_project.id {
                return Err(AppError::Validation(
                    "Language does not belong to the target project".to_string(),
                ));
            }
        }

        let grant = AudienceGrantQueries::create(
            &ctx.pool,
            audience_id,
            project_id,
            branch_id,
            language_id,
        )
        .await?;

        Ok(grant.into())
    }
```

- [ ] **Step 3: Verify compilation and tests**

Run: `cargo check --package cms-biz` and `cargo test --workspace`
Expected: Success with 0 errors.

- [ ] **Step 4: Commit changes**

```bash
git add crates/cms-biz/src/reader_access.rs docs/superpowers/plans/2026-08-31-audience-grant-organization-validation.md
git commit -m "feat(cms-biz): implement organization and resource validation for audience grants"
```
