# Production Maturity Audit & Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Conduct an exhaustive implementation maturity audit and remediate naive, stubbed, simplified, and unsafe implementations across the CMS Rust workspace.

**Architecture:** Replace stubs and naive implementations with mature production solutions: cryptographic SHA-256 for API keys, Argon2 password verification, proper session claims management, path traversal guards in storage, real OpenAPI fetching/parsing with reqwest, full-text search with Lindera Japanese tokenization, tree hierarchy preservation on branch duplicate, live database health pings, and batch query optimizations.

**Tech Stack:** Rust 2021, Tokio, Axum 0.8, SQLx 0.9, Argon2, Ring/SHA-2, Lindera 6.0, Reqwest 0.12, Governor, Metrics.

## Global Constraints
- Layering discipline: api → biz → db (no downward or circular dependencies).
- Zero placeholders or dummy mock responses in production code.
- All workspace tests must pass cleanly.

---

### Task 1: Cryptographic API Key Hashing & Session Authentication Fixes

**Files:**
- Modify: `crates/cms-auth/src/api_key.rs`
- Modify: `crates/cms-auth/src/lib.rs`
- Modify: `crates/cms-auth/src/session.rs`
- Modify: `crates/cms-biz/src/auth.rs`
- Modify: `crates/cms-api/src/auth/handlers.rs`
- Modify: `crates/cms-api/src/auth/middleware.rs`

- [ ] **Step 1: Write failing tests for API key hashing, session validation, and password verification**
- [ ] **Step 2: Implement cryptographic SHA-256 key hashing in `cms-auth` and fix `ApiKeyPrincipal` extractor**
- [ ] **Step 3: Fix `SessionService::create` and `validate` token/ID consistency in `cms-auth`**
- [ ] **Step 4: Implement Argon2 password hashing on register and password verification on login in `cms-biz/src/auth.rs`**
- [ ] **Step 5: Run tests and verify all auth tests pass**

---

### Task 2: Storage Path Traversal Guard

**Files:**
- Modify: `crates/cms-storage/src/lib.rs`

- [ ] **Step 1: Write tests attempting path traversal (`../evil.txt`, `/etc/passwd`)**
- [ ] **Step 2: Implement canonical root path containment check in `LocalFsStorage::get_path`**
- [ ] **Step 3: Run storage tests and verify traversal attacks are rejected**

---

### Task 3: Access Control & Authorization Hardening

**Files:**
- Modify: `crates/cms-access-control/src/lib.rs`
- Modify: `crates/cms-api/src/admin/handlers.rs`
- Modify: `crates/cms-biz/src/mcp.rs`
- Modify: `crates/cms-biz/src/entitlement.rs`

- [ ] **Step 1: Add system admin authorization check in `AccessControl` and enforce in admin handlers**
- [ ] **Step 2: Enforce public project check on unauthenticated MCP tool calls**
- [ ] **Step 3: Run access control and API tests**

---

### Task 4: Search Engine Japanese Morphological Tokenization & Postgres FTS

**Files:**
- Modify: `crates/cms-search/src/lib.rs`
- Modify: `crates/cms-biz/src/search.rs`

- [ ] **Step 1: Integrate `lindera` morphological analysis into `JapaneseTokenizer`**
- [ ] **Step 2: Implement `PgVectorSearchEngine::hybrid_query`, `index_page`, and `remove_page`**
- [ ] **Step 3: Run search engine tests and verify tokenization & query execution**

---

### Task 5: Production OpenAPI Document Fetching & Parsing

**Files:**
- Modify: `crates/cms-biz/src/openapi.rs`

- [ ] **Step 1: Implement HTTP client fetching with timeout, size limit, and error handling**
- [ ] **Step 2: Parse OpenAPI JSON/YAML specs and accurately count endpoints**
- [ ] **Step 3: Implement real content retrieval in `get_document_content`**
- [ ] **Step 4: Run OpenAPI tests**

---

### Task 6: Hierarchy Preservation & Public Documentation Resolution

**Files:**
- Modify: `crates/cms-biz/src/branch.rs`
- Modify: `crates/cms-biz/src/page.rs`
- Modify: `crates/cms-biz/src/theme.rs`
- Modify: `crates/cms-biz/src/org.rs`
- Modify: `crates/cms-biz/src/notification.rs`
- Modify: `crates/cms-db/src/notification.rs`

- [ ] **Step 1: Fix `duplicate_branch` page tree hierarchy preservation with ID mapping**
- [ ] **Step 2: Implement public page resolution (`get_public_page`, `list_public_pages`, `get_project_sitemap`)**
- [ ] **Step 3: Implement `set_project_theme` and fix `list_all_organizations` pagination total count**
- [ ] **Step 4: Implement batch notification status update and count aggregation by type**

---

### Task 7: Live Health Check Connectivity & Worker Graceful Shutdown

**Files:**
- Modify: `crates/cms-biz/src/platform_event.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Implement live database ping and latency reporting in `get_system_health`**
- [ ] **Step 2: Wire cancellation tokens for queue worker tasks in `src/main.rs`**
- [ ] **Step 3: Run full workspace test suite and verify clean shutdown**
