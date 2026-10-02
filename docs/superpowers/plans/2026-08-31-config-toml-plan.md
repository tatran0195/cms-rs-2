# TOML Configuration System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide comprehensive TOML configuration files (`config.toml`, `config.example.toml`, `config/dev.toml`, `config/deploy.toml`) and enhance `cms-config` to load TOML and environment overrides in a cascading hierarchy.

**Architecture:** Update `Cargo.toml` with `toml` support for the `config` crate, enhance `crates/cms-config/src/lib.rs` `Config::load` and `Config::load_from_path` functions to discover root and environment-profile TOML files, and provide fully annotated TOML configuration files for dev and deploy.

**Tech Stack:** Rust 2021, `config` crate (v0.15 with `toml` feature), `serde`.

## Global Constraints
- MSVC / Windows compatible path separators handling.
- Maintain full backwards compatibility for environment variables (`CMS_` prefix) and `.env` fallback files.
- Preserves zero clippy warnings / errors across workspace.

---

### Task 1: Enable TOML feature in `Cargo.toml`

**Files:**
- Modify: `Cargo.toml:85`
- Test: `cargo check -p cms-config`

**Interfaces:**
- Consumes: `workspace.dependencies.config`
- Produces: `config` crate with `toml` feature enabled for all workspace members

- [ ] **Step 1: Update `Cargo.toml` to include the `toml` feature for `config`**

Update `Cargo.toml` line 85 from:
```toml
config = "0.15.25"
```
to:
```toml
config = { version = "0.15.25", features = ["toml"] }
```

- [ ] **Step 2: Run `cargo check -p cms-config` to verify compilation**

Run: `cargo check -p cms-config`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "build: enable toml feature for config dependency"
```

---

### Task 2: Implement Cascading TOML Loader & Tests in `cms-config`

**Files:**
- Modify: `crates/cms-config/src/lib.rs`
- Test: `crates/cms-config/src/lib.rs` (`mod tests`)

**Interfaces:**
- Consumes: `config::Config`, `config::File`, `config::Environment`
- Produces: `Config::load() -> Result<Self, anyhow::Error>`, `Config::load_from_path(path: &str) -> Result<Self, anyhow::Error>`

- [ ] **Step 1: Write tests for TOML loading and environment overrides in `crates/cms-config/src/lib.rs`**

Add unit tests in `mod tests`:
- `test_load_from_toml_content`: Deserializes custom TOML string using `config::FileFormat::Toml` or temporary config file.
- `test_default_config`: Asserts default port is 3000, database url is postgres default, rate limiting is enabled.
- `test_load_from_path`: Asserts custom file path loading.

- [ ] **Step 2: Run `cargo test -p cms-config` to observe test failures/success**

Run: `cargo test -p cms-config`

- [ ] **Step 3: Implement `Config::load` and `Config::load_from_path` with cascading hierarchy in `crates/cms-config/src/lib.rs`**

Cascade resolution:
1. Base `config.toml` or `config/config.toml` (if exists).
2. Environment-specific `config/{CMS_ENV}.toml` or `config/{CMS_ENV}.env` (defaults to `dev`).
3. Explicit `CMS_CONFIG_PATH` if set.
4. Environment variables starting with `CMS_` (using `_` prefix and `__` separator).

- [ ] **Step 4: Run `cargo test -p cms-config` to verify all tests pass**

Run: `cargo test -p cms-config`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cms-config/src/lib.rs
git commit -m "feat(config): add cascading TOML and environment profile loading"
```

---

### Task 3: Create TOML Configuration Files

**Files:**
- Create: `config.toml`
- Create: `config.example.toml`
- Create: `config/dev.toml`
- Create: `config/deploy.toml`

**Interfaces:**
- Consumes: Config schema types from `cms-config`
- Produces: Standard configuration templates for local dev and AWS deployment

- [ ] **Step 1: Create `config.toml` with detailed comments for all sections**

Define sections: `[server]`, `[database]`, `[storage]`, `[search]`, `[queue]`, `[analytics]`, `[auth]`, `[auth.oauth.github]`, `[auth.oauth.google]`, `[site]`, `[mcp]`, `[mailer]`, `[rate_limit]`, `[security_headers]`, `[admin_origin]`.

- [ ] **Step 2: Create `config.example.toml` mirroring all parameters with placeholders/defaults**

- [ ] **Step 3: Create `config/dev.toml` and `config/deploy.toml` with environment specific presets**

`config/dev.toml` with local development presets, and `config/deploy.toml` with production recommendations (SSL enabled, S3 storage, Redis queue, secure cookies/session secrets placeholders).

- [ ] **Step 4: Verify `Config::load()` correctly loads `config.toml`**

Run: `cargo test -p cms-config`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add config.toml config.example.toml config/dev.toml config/deploy.toml
git commit -m "feat(config): add root config.toml, config.example.toml, and environment profiles"
```

---

### Task 4: Workspace Verification

**Files:**
- Test: `Cargo.lock`, `src/main.rs`, all crates

- [ ] **Step 1: Run full workspace test suite and lint checks**

Run: `cargo test --workspace`
Run: `cargo clippy --workspace --all-targets -- -D warnings`

- [ ] **Step 2: Commit any final workspace adjustments**

```bash
git add -A
git commit -m "chore: verify workspace builds and passes tests with toml config"
```
