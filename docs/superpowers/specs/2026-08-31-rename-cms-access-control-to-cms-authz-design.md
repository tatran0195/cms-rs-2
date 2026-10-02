# Rename cms-access-control to cms-authz Design

## Overview
Rename the authorization crate `cms-access-control` to `cms-authz` and its primary trait `AccessControl` to `Authz`. This refactoring aligns the crate naming with the authentication crate `cms-auth`, providing clean architectural symmetry (`cms-auth` for authentication, `cms-authz` for authorization), concise naming, and consistent field nomenclature across the workspace.

## Scope of Changes

### 1. Crate Directory and Manifest
- Rename directory `crates/cms-access-control` to `crates/cms-authz`.
- Update `crates/cms-authz/Cargo.toml`:
  - `name = "cms-authz"`
- Update root `Cargo.toml`:
  - `members`: replace `"crates/cms-access-control"` with `"crates/cms-authz"`
  - `[workspace.dependencies]`: replace `cms-access-control = { path = "crates/cms-access-control" }` with `cms-authz = { path = "crates/cms-authz" }`
- Update consumer crate manifests (`crates/cms-api/Cargo.toml`, `crates/cms-biz/Cargo.toml`, `crates/cms-middleware/Cargo.toml`, `crates/cms-mcp/Cargo.toml`):
  - Replace `cms-access-control = { path = "../cms-access-control" }` with `cms-authz = { path = "../cms-authz" }`

### 2. Trait, Struct, and Function Renaming in `cms-authz`
In `crates/cms-authz/src/lib.rs`:
- Trait: `pub trait AccessControl` $\rightarrow$ `pub trait Authz`
- Production struct: `pub struct ProductionAccessControl` $\rightarrow$ `pub struct ProductionAuthz`
- Noop struct: `pub struct NoopAccessControl` $\rightarrow$ `pub struct NoopAuthz`
- Constructor: `pub fn create_access_control(...)` $\rightarrow$ `pub fn create_authz(...)`
- Crate doc comments and unit tests updated accordingly.

### 3. Struct Fields & Consumers
- In `crates/cms-biz/src/lib.rs`:
  - `BizContext.access_control` $\rightarrow$ `BizContext.authz` (type `std::sync::Arc<dyn cms_authz::Authz>`)
  - Update `BizContext::new(pool, authz)` parameter.
- In `crates/cms-middleware/src/app_state.rs`:
  - Use `cms_authz::ProductionAuthz` and pass `authz` into `BizContext`.
- In `crates/cms-mcp/src/lib.rs`:
  - Use `cms_authz::NoopAuthz`.
- Across all `crates/cms-biz/src/*.rs` service modules:
  - Replace `ctx.access_control.<method>` with `ctx.authz.<method>`
  - Replace `use cms_access_control::...` with `use cms_authz::...`
- Across `crates/cms-api/src/**/*.rs`:
  - Update any direct references/imports if present.
- In `README.md` and documentation:
  - Update references from `cms-access-control` to `cms-authz`.

## Verification Plan
1. `cargo check --workspace` to ensure all crates resolve dependencies and type signatures.
2. `cargo test --workspace` to ensure all unit and integration tests compile and pass.
3. `cargo run --bin cms-server -- --help` (or check build) to ensure binary crate builds cleanly.
