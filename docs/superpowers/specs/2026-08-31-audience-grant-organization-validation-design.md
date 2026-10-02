# Audience Grant Organization and Resource Validation Design

## Overview
Implement production-grade tenant isolation, permission, and resource consistency checks for `ReaderAccessService::create_audience_grant` in `crates/cms-biz/src/reader_access.rs`.

## Context & Problem
When creating an `AudienceGrant`, an audience associated with a source project (`audience.project_id`) is granted access to documentation in a target project (`project_id`), with optional scoping to a specific branch (`branch_id`) or language (`language_id`).

Previously, lines 246-247 of `crates/cms-biz/src/reader_access.rs` contained placeholder comments without actually validating that the target project belongs to the same organization, that the user possesses administration rights on the target project, or that the branch and language belong to the target project.

## Proposed Changes

### 1. Source & Target Project Resolution
- Query the source project: `ProjectQueries::get_by_id(&ctx.pool, &audience.project_id)`. If missing, return `AppError::NotFound("Audience project not found")`.
- Query the target project: `ProjectQueries::get_by_id(&ctx.pool, project_id)`. If missing, return `AppError::NotFound("Target project not found")`.

### 2. Multi-Tenant Organization Boundary Verification
- Verify that `audience_project.organization_id == target_project.organization_id`.
- If mismatch, return `AppError::AccessDenied("Audience and target project must belong to the same organization")`.

### 3. User Permission Verification on Target Project
- Verify that the calling user has at least `MemberRole::Admin` on the target project via `ctx.access_control.require_project_role(user_id, project_id, MemberRole::Admin)`.

### 4. Optional Branch and Language Scoping Validation
- If `branch_id` is provided:
  - Query `BranchQueries::get_by_id(&ctx.pool, branch_id)`. If not found, return `AppError::NotFound("Branch not found")`.
  - Validate `branch.project_id == target_project.id`. If mismatch, return `AppError::Validation("Branch does not belong to the target project")`.
- If `language_id` is provided:
  - Query `LanguageQueries::get_by_id(&ctx.pool, language_id)`. If not found, return `AppError::NotFound("Language not found")`.
  - Validate `language.project_id == target_project.id`. If mismatch, return `AppError::Validation("Language does not belong to the target project")`.

## Files Affected
- `crates/cms-biz/src/reader_access.rs`

## Verification Plan
1. Compile the workspace using `cargo check` and `cargo test`.
2. Ensure unit/integration tests compile cleanly and verify with `cargo test --workspace`.
