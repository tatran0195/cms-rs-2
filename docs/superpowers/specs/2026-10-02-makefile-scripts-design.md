# Design Spec: Developer Makefile for CMS

## 1. Overview & Goal
Add a comprehensive, self-documenting root `Makefile` to the `cms-rs` repository to streamline common full-stack development, build, test, lint, and database operations.

The repository is a full-stack monorepo featuring:
- Rust backend workspace (`apps/api` for `cms-server`, `crates/cms-worker` for `cms-worker`, domain crates, and `xtask` for E2E tests).
- Frontend web application in `apps/app` (Vite, React, TanStack) managed via `pnpm`.
- PostgreSQL database schemas managed with SQLx migrations in `migrations/`.

The Makefile unifies these disparate tools into intuitive, memorable commands with formatted `--help` output.

---

## 2. Target Design & Specifications

### 2.1 Help & Default Target
- **`default` / `help`**: Scans the Makefile for `##` comments and displays a clean, colorized reference table categorized by workflow phase. Running `make` without arguments displays help.

### 2.2 Development Commands
- **`dev`**: Starts the development environment (runs backend API server by default).
- **`dev-backend`**: Runs the API server binary via `cargo run -p cms-server`.
- **`dev-worker`**: Runs the background worker binary via `cargo run -p cms-worker`.
- **`dev-frontend`**: Starts the Vite development server via `pnpm --filter @cms/app dev`.
- **`install`**: Installs frontend workspace dependencies via `pnpm install`.

### 2.3 Build Commands
- **`build`**: Builds both backend and frontend release artifacts (`build-backend` + `build-frontend`).
- **`build-backend`**: Builds optimized release binaries for `cms-server` and `cms-worker` via `cargo build --release --bin cms-server --bin cms-worker`.
- **`build-backend-dev`**: Builds workspace debug binaries via `cargo build --workspace`.
- **`build-frontend`**: Builds the frontend distribution bundle via `pnpm --filter @cms/app build`.

### 2.4 Testing Commands
- **`test`**: Runs both Rust workspace tests and frontend tests (`test-backend` + `test-frontend`).
- **`test-backend`**: Executes all Rust workspace unit and integration tests via `cargo test --workspace`.
- **`test-frontend`**: Runs frontend test suite via `pnpm --filter @cms/app test`.
- **`test-e2e`**: Runs the disposable-database integration E2E test via `cargo xtask e2e`.

### 2.5 Code Quality & Formatting
- **`check`**: Fast workspace compile check via `cargo check --workspace --all-targets`.
- **`lint`**: Runs Rust clippy (`cargo clippy --workspace --all-targets --all-features -- -D warnings`) and frontend TypeScript type checking (`pnpm --filter @cms/app typecheck`).
- **`fmt`**: Formats Rust source files (`cargo fmt --all`).
- **`fmt-check`**: Validates Rust code formatting without mutating files (`cargo fmt --all -- --check`).

### 2.6 Database Management (SQLx)
- **`db-create`**: Creates the database using `sqlx database create`.
- **`db-drop`**: Drops the database using `sqlx database drop -y`.
- **`db-migrate`**: Applies all pending migrations using `sqlx migrate run`.
- **`db-revert`**: Reverts the most recent migration using `sqlx migrate revert`.
- **`db-reset`**: Drops, recreates, and re-applies migrations (`db-drop db-create db-migrate`).
- **`db-status`**: Shows applied migration status with `sqlx migrate info`.

### 2.7 Clean
- **`clean`**: Cleans Rust cargo target artifacts and frontend build outputs (`cargo clean`, removing `apps/app/dist` and caches).

---

## 3. Platform & Shell Compatibility
- **Host System**: Windows with GNU Make 4.4.1 (using built-in `sh.exe` shell runner).
- **Declaration**: All non-file targets declared in `.PHONY` to avoid collision with existing directories like `dist/` or `tests/`.
- **Variables**:
  - `CARGO ?= cargo`
  - `PNPM ?= pnpm`
  - `SQLX ?= sqlx`
  - `DATABASE_URL ?= postgres://postgres:postgres@localhost:5432/cms_dev`
- **Output Styling**: ANSI colors for header sections and target names with safe fallback.

---

## 4. Verification & Testing Strategy
1. Verify `make help` displays formatted target documentation cleanly.
2. Verify `make check` executes `cargo check --workspace --all-targets`.
3. Verify `make fmt-check` executes without errors.
4. Verify non-destructive targets (`make dev-backend --dry-run`, `make build-frontend --dry-run` or direct target invocation) parse and run accurately.
