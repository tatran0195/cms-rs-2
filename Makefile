# ==============================================================================
# CMS Monorepo Makefile
# ==============================================================================

# Variables (can be overridden via environment or command-line args)
CARGO ?= cargo
PNPM ?= pnpm
SQLX ?= sqlx
DATABASE_URL ?= postgres://postgres:postgres@localhost:5432/cms_dev

.DEFAULT_GOAL := help

# ==============================================================================
# Help
# ==============================================================================

.PHONY: help
help:
	@echo.
	@echo ==============================================================================
	@echo   CMS Monorepo - Available Make Commands
	@echo ==============================================================================
	@echo.
	@echo   Development:
	@echo     make dev                Run backend API server (alias to dev-backend)
	@echo     make dev-backend        Run backend API server in debug mode
	@echo     make dev-worker         Run background worker in debug mode
	@echo     make dev-frontend       Run frontend web app dev server (port 4310)
	@echo     make install            Install frontend and workspace dependencies
	@echo.
	@echo   Build:
	@echo     make build              Build backend release binaries and frontend bundle
	@echo     make build-backend      Build optimized release binaries for server and worker
	@echo     make build-backend-dev  Build backend debug binaries across workspace
	@echo     make build-frontend     Build frontend web application bundle
	@echo.
	@echo   Testing:
	@echo     make test               Run backend workspace tests and frontend tests
	@echo     make test-backend       Run all Rust unit and integration tests
	@echo     make test-frontend      Run frontend vitest suite
	@echo     make test-e2e           Run PostgreSQL integration test (xtask e2e)
	@echo.
	@echo   Linting and Formatting:
	@echo     make check              Quick compiler check across all Rust workspace targets
	@echo     make lint               Run Rust clippy and frontend TypeScript typecheck
	@echo     make fmt                Format Rust code across workspace
	@echo     make fmt-check          Check Rust formatting without modifying files
	@echo.
	@echo   Database (SQLx):
	@echo     make db-create          Create PostgreSQL database
	@echo     make db-drop            Drop PostgreSQL database (with confirmation bypass)
	@echo     make db-migrate         Run pending SQLx migrations
	@echo     make db-revert          Revert latest SQLx migration
	@echo     make db-reset           Drop, recreate, and migrate database
	@echo     make db-status          Show status of applied and pending migrations
	@echo.
	@echo   Cleanup:
	@echo     make clean              Clean Rust target artifacts and frontend build outputs
	@echo.

# ==============================================================================
# Development
# ==============================================================================

.PHONY: dev dev-backend dev-worker dev-frontend install
install:
	$(PNPM) install

dev: dev-backend

dev-backend:
	$(CARGO) run -p cms-server

dev-worker:
	$(CARGO) run -p cms-worker

dev-frontend:
	$(PNPM) --filter @cms/app run dev

# ==============================================================================
# Build
# ==============================================================================

.PHONY: build build-backend build-backend-dev build-frontend
build: build-backend build-frontend

build-backend:
	$(CARGO) build --release --bin cms-server --bin cms-worker

build-backend-dev:
	$(CARGO) build --workspace

build-frontend:
	$(PNPM) --filter @cms/app run build

# ==============================================================================
# Testing
# ==============================================================================

.PHONY: test test-backend test-frontend test-e2e
test: test-backend test-frontend

test-backend:
	$(CARGO) test --workspace

test-frontend:
	$(PNPM) --filter @cms/app run test

test-e2e:
	$(CARGO) xtask e2e

# ==============================================================================
# Linting & Formatting
# ==============================================================================

.PHONY: check lint fmt fmt-check
check:
	$(CARGO) check --workspace --all-targets

lint:
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	$(PNPM) --filter @cms/app run typecheck

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

# ==============================================================================
# Database (SQLx)
# ==============================================================================

.PHONY: db-create db-drop db-migrate db-revert db-reset db-status
db-create:
	$(SQLX) database create --database-url $(DATABASE_URL)

db-drop:
	$(SQLX) database drop -y --database-url $(DATABASE_URL)

db-migrate:
	$(SQLX) migrate run --database-url $(DATABASE_URL)

db-revert:
	$(SQLX) migrate revert --database-url $(DATABASE_URL)

db-reset: db-drop db-create db-migrate

db-status:
	$(SQLX) migrate info --database-url $(DATABASE_URL)

# ==============================================================================
# Cleanup
# ==============================================================================

.PHONY: clean
clean:
	$(CARGO) clean
	$(PNPM) --filter @cms/app run clean
