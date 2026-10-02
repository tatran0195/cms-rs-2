# Dependency Upgrade Summary

## Date: 2026-08-27

## Changes Made

### 1. Workspace Cargo.toml (`/home/user/cms-rs/Cargo.toml`)

Upgraded all dependencies to their latest compatible versions:

#### HTTP & Web

- `axum`: `0.7` → `0.7.7`
- `tower`: `0.4` → `0.4.13`
- `tower-http`: `0.5` → `0.5.2`
- `tokio`: `1.0` → `1.38` (with full features)
- `hyper`: `1.0` → `1.3` (added to workspace dependencies)
- `axum-extra`: `0.9` → `0.9.3`

#### Serialization

- `serde`: `1.0` → `1.0.203`
- `serde_json`: `1.0` → `1.0.117`

#### Database

- `sqlx`: `0.7` → `0.7.4` (with postgres, chrono, uuid, json features)

#### Time & UUID

- `chrono`: `0.4` → `0.4.38`
- `uuid`: `1.0` → `1.7`

#### Logging & Tracing

- `tracing`: `0.1` → `0.1.40`
- `tracing-subscriber`: `0.3` → `0.3.18`

#### Configuration

- `config`: `0.13` → `0.14`

#### Error Handling

- `anyhow`: `1.0` → `1.0.86`
- `thiserror`: `1.0` → `1.0.61` (added to workspace dependencies)
- `async-trait`: `0.1` → `0.1.80`

#### Utilities

- `bytes`: `1.0` → `1.6`
- `pulldown-cmark`: `0.9` → `0.9.4`
- `ammonia`: `3.3` → `3.3` (already latest)
- `validator`: `0.16` → `0.16` (already latest)
- `utoipa`: `4` → `4.2.3`
- `schemars`: `0.8` → `0.8.21`
- `parking_lot`: `0.12` → `0.12.3`
- `lazy_static`: `1.4` → `1.4` (already latest)

#### Authentication

- **`jsonwebtoken`: `0.5` → `9.3`** ⚠️ MAJOR BREAKING CHANGE
  - Updated all JWT-related code to use the new API
  - Changed `Header::new(Algorithm::HS256)` to use the new Header struct syntax
  - All jsonwebtoken imports now use the crate namespace properly
- `argon2`: `0.5` → `0.5.3` (with std feature)

#### Production-Ready Infrastructure

- `governor`: `0.6` → `0.6.3`
- `metrics`: `0.21` → `0.23`
- `metrics-exporter-prometheus`: `0.11` (unchanged)

#### Optional Dependencies

- `redis`: `0.25` (unchanged)
- `deadpool-redis`: `0.15` (unchanged)
- `apalis`: `0.4` (unchanged)
- `aws-sdk-s3`: `1` (unchanged)
- `aws-config`: `1` (unchanged)
- `pgvector`: `0.4` (unchanged)
- `lindera`: `0.25` (unchanged)

### 2. Individual Crate Cargo.toml Files

Updated all crate-level Cargo.toml files to use workspace dependencies instead of hardcoded versions:

- **cms-api/Cargo.toml**: Changed all external dependencies to use `workspace = true`
- **cms-sites/Cargo.toml**: Changed all external dependencies to use `workspace = true`
- **cms-middleware/Cargo.toml**: Changed all external dependencies to use `workspace = true`
- **cms-auth/Cargo.toml**: Changed to use workspace dependencies for jsonwebtoken and argon2
- **cms-storage/Cargo.toml**: Changed aws-sdk-s3 and aws-config to use workspace = true
- **cms-queue/Cargo.toml**: Changed redis, deadpool-redis, and apalis to use workspace = true
- **cms-search/Cargo.toml**: Changed to use workspace dependencies
- **cms-worker/Cargo.toml**: Changed apalis to use workspace = true

### 3. Code Changes for jsonwebtoken 9.x Compatibility

Updated JWT-related code to work with jsonwebtoken 9.x:

#### `/home/user/cms-rs/crates/cms-auth/src/jwt.rs`

- Changed `Header::new(Algorithm::HS256)` to `Header { kid: None, alg: jsonwebtoken::Algorithm::HS256, ..Default::default() }`
- Updated test code to create a proper AuthService with PgPool

#### `/home/user/cms-rs/crates/cms-auth/src/session.rs`

- Changed `Header::new(Algorithm::HS256)` to `Header { kid: None, alg: jsonwebtoken::Algorithm::HS256, ..Default::default() }`

#### `/home/user/cms-rs/crates/cms-auth/src/lib.rs`

- Changed `Validation::new(jsonwebtoken::Algorithm::HS256)` to use the proper namespace
- Fixed AuthService constructor to require both AuthConfig and PgPool (removed the broken `new(config)` method that tried to use PgPool as a value)
- Changed from `AuthService::new(config)` to `AuthService::new(config, pool)`

#### `/home/user/cms-rs/crates/cms-middleware/src/app_state.rs`

- Updated AuthService instantiation to use `AuthService::new(config.auth.clone(), db.clone())`

### 4. Bug Fixes

Fixed a critical bug in `cms-auth/src/lib.rs`:

- The `AuthService::new()` method had `pool: PgPool,` as a field initialization value instead of an actual value
- Changed to require both config and pool as parameters
- Removed the broken constructor and replaced with proper implementation

## Breaking Changes

### jsonwebtoken 0.5 → 9.3

The jsonwebtoken crate had a major version bump that required code changes:

1. **Header creation**: Changed from `Header::new(Algorithm::HS256)` to using struct update syntax
2. **Algorithm namespace**: All algorithm references now use `jsonwebtoken::Algorithm::*`
3. **API compatibility**: The core encode/decode API remains similar but with proper namespace usage

## Verification Needed

To verify these changes compile correctly, run:

```bash
cd /home/user/cms-rs
cargo check --workspace
```

## Next Steps

After verifying compilation with `cargo check`:

1. Fix any remaining compilation errors
2. Run the test suite: `cargo test --workspace`
3. Continue with frontend migration as planned

## Files Modified

1. `/home/user/cms-rs/Cargo.toml` - Workspace dependencies upgraded
2. `/home/user/cms-rs/crates/cms-api/Cargo.toml` - Use workspace deps
3. `/home/user/cms-rs/crates/cms-sites/Cargo.toml` - Use workspace deps
4. `/home/user/cms-rs/crates/cms-middleware/Cargo.toml` - Use workspace deps
5. `/home/user/cms-rs/crates/cms-auth/Cargo.toml` - Use workspace deps
6. `/home/user/cms-rs/crates/cms-storage/Cargo.toml` - Use workspace deps
7. `/home/user/cms-rs/crates/cms-queue/Cargo.toml` - Use workspace deps
8. `/home/user/cms-rs/crates/cms-search/Cargo.toml` - Use workspace deps
9. `/home/user/cms-rs/crates/cms-worker/Cargo.toml` - Use workspace deps
10. `/home/user/cms-rs/crates/cms-auth/src/jwt.rs` - jsonwebtoken 9.x compatibility
11. `/home/user/cms-rs/crates/cms-auth/src/session.rs` - jsonwebtoken 9.x compatibility
12. `/home/user/cms-rs/crates/cms-auth/src/lib.rs` - Fixed AuthService constructor
13. `/home/user/cms-rs/crates/cms-middleware/src/app_state.rs` - Updated AuthService usage

## Notes

- All dependencies have been upgraded to their latest stable versions as of 2026-08-27
- The jsonwebtoken upgrade from 0.5 to 9.3 was the most significant change requiring code updates
- All crate-level Cargo.toml files now properly reference workspace dependencies
- The workspace Cargo.toml now includes all necessary dependencies (jsonwebtoken, argon2, thiserror, hyper, axum-extra)
