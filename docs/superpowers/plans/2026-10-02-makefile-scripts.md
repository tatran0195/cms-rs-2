# Developer Makefile Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a comprehensive, self-documenting root `Makefile` that simplifies common developer workflows across the Rust backend workspace, frontend web application, and database operations.

**Architecture:** A single top-level `Makefile` organized into logical sections (`help`, `dev`, `build`, `test`, `lint & format`, `database`, `clean`). Recipes invoke underlying tooling (`cargo`, `pnpm`, `sqlx`) with portable flags and cross-platform compatibility for Windows GNU Make.

**Tech Stack:** GNU Make 4.4.1, Cargo / Rust 1.96+, pnpm 11+, SQLx CLI.

## Global Constraints
- Target platform: Windows with GNU Make 4.4.1 (standard in developer environment at `C:\Users\Admin\.cargo\bin\make.exe`).
- All phony targets must be declared explicitly in `.PHONY` to avoid collision with files/directories like `dist/` or `tests/`.
- Must support default invocation (`make` or `make help`) showing colorized target categories and descriptions extracted from `##` doc comments.
- Must respect configurable environment overrides (`CARGO`, `PNPM`, `SQLX`, `DATABASE_URL`, `CMS_ENV`).

---

### Task 1: Create Root Makefile

**Files:**
- Create: `d:/Workspace/Software/Cloned-Repos/cms-rs/cms-rs/Makefile`

**Interfaces:**
- Consumes: `cargo`, `pnpm`, `sqlx` CLIs.
- Produces: Makefile targets: `help`, `dev`, `dev-backend`, `dev-worker`, `dev-frontend`, `install`, `build`, `build-backend`, `build-backend-dev`, `build-frontend`, `test`, `test-backend`, `test-frontend`, `test-e2e`, `check`, `lint`, `fmt`, `fmt-check`, `db-create`, `db-drop`, `db-migrate`, `db-revert`, `db-reset`, `db-status`, `clean`.

- [ ] **Step 1: Write root Makefile**

Write `Makefile` with clean recipe headers and self-documenting comments:

```makefile
# ==============================================================================
# CMS Monorepo Makefile
# ==============================================================================

# Variables (can be overridden via environment or command-line args)
CARGO ?= cargo
PNPM ?= pnpm
SQLX ?= sqlx
CMS_ENV ?= dev
DATABASE_URL ?= postgres://postgres:postgres@localhost:5432/cms_dev

# Terminal colors for help output
CYAN   := \033[36m
GREEN  := \033[32m
YELLOW := \033[33m
RESET  := \033[0m

.DEFAULT_GOAL := help

# ==============================================================================
# Help
# ==============================================================================

.PHONY: help
help: ## Display this help message
	@printf "\n$(CYAN)Available targets in CMS:$(RESET)\n\n"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(GREEN)%-20s$(RESET) %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@printf "\n"

# ==============================================================================
# Development
# ==============================================================================

.PHONY: dev dev-backend dev-worker dev-frontend install
install: ## Install frontend and workspace node dependencies
	$(PNPM) install

dev: dev-backend ## Run backend API server (alias to dev-backend)

dev-backend: ## Run backend API server in debug mode
	CMS_ENV=$(CMS_ENV) $(CARGO) run -p cms-server

dev-worker: ## Run background worker in debug mode
	CMS_ENV=$(CMS_ENV) $(CARGO) run -p cms-worker

dev-frontend: ## Run frontend web app dev server (port 4310)
	$(PNPM) --filter @cms/app dev

# ==============================================================================
# Build
# ==============================================================================

.PHONY: build build-backend build-backend-dev build-frontend
build: build-backend build-frontend ## Build backend release binaries and frontend bundle

build-backend: ## Build optimized release binaries for server and worker
	$(CARGO) build --release --bin cms-server --bin cms-worker

build-backend-dev: ## Build backend debug binaries across workspace
	$(CARGO) build --workspace

build-frontend: ## Build frontend web application bundle
	$(PNPM) --filter @cms/app build

# ==============================================================================
# Testing
# ==============================================================================

.PHONY: test test-backend test-frontend test-e2e
test: test-backend test-frontend ## Run backend workspace tests and frontend tests

test-backend: ## Run all Rust unit and integration tests across workspace
	$(CARGO) test --workspace

test-frontend: ## Run frontend test suite
	$(PNPM) --filter @cms/app test

test-e2e: ## Run PostgreSQL integration test (requires CMS_E2E_DATABASE_URL)
	$(CARGO) xtask e2e

# ==============================================================================
# Linting & Formatting
# ==============================================================================

.PHONY: check lint fmt fmt-check
check: ## Quick compiler check across all Rust workspace targets
	$(CARGO) check --workspace --all-targets

lint: ## Run Clippy on Rust code and typecheck frontend
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	$(PNPM) --filter @cms/app typecheck

fmt: ## Format Rust code
	$(CARGO) fmt --all

fmt-check: ## Check Rust formatting without modifying files
	$(CARGO) fmt --all -- --check

# ==============================================================================
# Database (SQLx)
# ==============================================================================

.PHONY: db-create db-drop db-migrate db-revert db-reset db-status
db-create: ## Create PostgreSQL database if not exists
	DATABASE_URL="$(DATABASE_URL)" $(SQLX) database create

db-drop: ## Drop PostgreSQL database (with confirmation prompt bypass)
	DATABASE_URL="$(DATABASE_URL)" $(SQLX) database drop -y

db-migrate: ## Run pending SQLx migrations
	DATABASE_URL="$(DATABASE_URL)" $(SQLX) migrate run

db-revert: ## Revert latest SQLx migration
	DATABASE_URL="$(DATABASE_URL)" $(SQLX) migrate revert

db-reset: db-drop db-create db-migrate ## Drop, recreate, and migrate database

db-status: ## Show status of applied and pending migrations
	DATABASE_URL="$(DATABASE_URL)" $(SQLX) migrate info

# ==============================================================================
# Cleanup
# ==============================================================================

.PHONY: clean
clean: ## Clean Rust target artifacts and frontend build outputs
	$(CARGO) clean
	$(PNPM) --filter @cms/app clean
```

- [ ] **Step 2: Commit Makefile**

```bash
git add Makefile
git commit -m "feat: add root Makefile for full-stack developer workflows"
```

---

### Task 2: Verify Makefile Targets

**Files:**
- Test target execution from workspace root `d:/Workspace/Software/Cloned-Repos/cms-rs/cms-rs`

- [ ] **Step 1: Test `make help`**

Run: `make help`
Expected: Nicely formatted table listing all targets categorized with green names and clear descriptions.

- [ ] **Step 2: Test `make fmt-check`**

Run: `make fmt-check`
Expected: Rust formatting check completes (exits 0 or reports formatted status).

- [ ] **Step 3: Test `make check`**

Run: `make check`
Expected: `cargo check --workspace --all-targets` runs without fatal errors.

- [ ] **Step 4: Test target dry-runs**

Run: `make -n build`, `make -n dev-backend`, `make -n db-migrate`
Expected: Makefile prints the exact expected commands without executing them.

---

### Task 3: Update Documentation & Quick Start

**Files:**
- Modify: `d:/Workspace/Software/Cloned-Repos/cms-rs/cms-rs/README.md`

- [ ] **Step 1: Add Makefile section to README.md**

Update the "Build and run" and quick start section to highlight the convenient `make` commands:
```markdown
### Using Makefile

A root `Makefile` is provided for common development and maintenance tasks:

```bash
make help          # View all available targets and descriptions
make dev           # Start backend server in debug mode
make dev-frontend  # Start frontend development server
make build         # Build release backend binaries and frontend bundle
make test          # Run all Rust and frontend tests
make check         # Fast compiler check across the workspace
make lint          # Run clippy and TypeScript typechecking
make fmt           # Format codebase
make db-migrate    # Run pending SQLx migrations
```
```

- [ ] **Step 2: Commit documentation updates**

```bash
git add README.md
git commit -m "docs: add Makefile quick reference to README"
```
