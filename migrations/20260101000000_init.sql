-- Migration: Initial schema
-- This migration creates the foundational tables for CMS
-- Based on the existing Prisma schema from the TypeScript monorepo

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";


-- ============================================
-- Identity & Authentication
-- ============================================

CREATE TABLE IF NOT EXISTS "User" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    email TEXT NOT NULL UNIQUE,
    name TEXT,
    image TEXT,
    email_verified BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "Session" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id TEXT NOT NULL REFERENCES "User"(id) ON DELETE CASCADE,
    session_token TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "Account" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id TEXT NOT NULL REFERENCES "User"(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    provider_account_id TEXT NOT NULL,
    access_token TEXT,
    refresh_token TEXT,
    expires_at TIMESTAMPTZ,
    token_type TEXT,
    scope TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(provider, provider_account_id)
);

CREATE TABLE IF NOT EXISTS "VerificationToken" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    identifier TEXT NOT NULL,
    token TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "ApiKey" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id TEXT NOT NULL REFERENCES "User"(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    key TEXT NOT NULL, -- Hashed key
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

-- ============================================
-- Tenancy (Organizations & Members)
-- ============================================

CREATE TABLE IF NOT EXISTS "Organization" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT,
    logo TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TYPE "MemberRole" AS ENUM ('OWNER', 'ADMIN', 'MEMBER', 'GUEST');

CREATE TABLE IF NOT EXISTS "Member" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id TEXT NOT NULL REFERENCES "User"(id) ON DELETE CASCADE,
    organization_id TEXT NOT NULL REFERENCES "Organization"(id) ON DELETE CASCADE,
    role "MemberRole" NOT NULL DEFAULT 'MEMBER',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, organization_id)
);

CREATE TABLE IF NOT EXISTS "Invitation" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id TEXT NOT NULL REFERENCES "Organization"(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role "MemberRole" NOT NULL DEFAULT 'MEMBER',
    token TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Core Product (Projects)
-- ============================================

CREATE TABLE IF NOT EXISTS "Project" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id TEXT NOT NULL REFERENCES "Organization"(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    description TEXT,
    icon TEXT,
    is_public BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(organization_id, slug)
);

CREATE TABLE IF NOT EXISTS "ProjectAddon" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    addon_type TEXT NOT NULL,
    config JSONB NOT NULL DEFAULT '{}',
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "ProjectAuditEvent" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    user_id TEXT REFERENCES "User"(id) ON DELETE SET NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Branches
-- ============================================

CREATE TABLE IF NOT EXISTS "Branch" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    description TEXT,
    is_default BOOLEAN NOT NULL DEFAULT false,
    is_protected BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, slug)
);

-- ============================================
-- Languages (retargeted from Arabic to Japanese)
-- ============================================

CREATE TABLE IF NOT EXISTS "Language" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    code TEXT NOT NULL, -- ISO 639-1 code (e.g., "en", "ja")
    name TEXT NOT NULL, -- Human-readable name
    is_default BOOLEAN NOT NULL DEFAULT false,
    is_rtl BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, code)
);

CREATE TABLE IF NOT EXISTS "ProjectTranslation" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    language_id TEXT NOT NULL REFERENCES "Language"(id) ON DELETE CASCADE,
    name TEXT,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, language_id)
);

-- ============================================
-- Pages (Page Tree)
-- ============================================

CREATE TABLE IF NOT EXISTS "Page" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    branch_id TEXT NOT NULL REFERENCES "Branch"(id) ON DELETE CASCADE,
    parent_id TEXT REFERENCES "Page"(id) ON DELETE SET NULL,
    path TEXT NOT NULL, -- Materialized path
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    content TEXT NOT NULL DEFAULT '',
    position INTEGER NOT NULL DEFAULT 0,
    is_published BOOLEAN NOT NULL DEFAULT true,
    is_indexed BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, branch_id, slug),
    UNIQUE(project_id, branch_id, path)
);

-- ============================================
-- Publishing & Deployments
-- ============================================

CREATE TYPE "DeploymentStatus" AS ENUM ('PENDING', 'BUILDING', 'DEPLOYING', 'ACTIVE', 'FAILED', 'DELETED');

CREATE TABLE IF NOT EXISTS "Deployment" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    branch_id TEXT REFERENCES "Branch"(id) ON DELETE SET NULL,
    status "DeploymentStatus" NOT NULL DEFAULT 'PENDING',
    build_logs TEXT,
    error_message TEXT,
    deployed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "Domain" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    deployment_id TEXT NOT NULL REFERENCES "Deployment"(id) ON DELETE CASCADE,
    hostname TEXT NOT NULL UNIQUE,
    is_primary BOOLEAN NOT NULL DEFAULT false,
    ssl_certificate TEXT,
    ssl_certificate_expires_at TIMESTAMPTZ,
    verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Git Integration
-- ============================================

CREATE TYPE "GitProvider" AS ENUM ('GITHUB', 'GITLAB', 'BITBUCKET', 'AZURE_DEVOPS');

CREATE TABLE IF NOT EXISTS "GitConnection" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    provider "GitProvider" NOT NULL,
    repository TEXT NOT NULL,
    branch TEXT NOT NULL DEFAULT 'main',
    access_token TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TYPE "GitSyncOperationStatus" AS ENUM ('PENDING', 'PROCESSING', 'COMPLETED', 'FAILED', 'CONFLICT');
CREATE TYPE "GitSyncOperationType" AS ENUM ('FULL', 'INCREMENTAL', 'MANUAL');

CREATE TABLE IF NOT EXISTS "GitSyncOperation" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    connection_id TEXT NOT NULL REFERENCES "GitConnection"(id) ON DELETE CASCADE,
    operation_type "GitSyncOperationType" NOT NULL,
    status "GitSyncOperationStatus" NOT NULL DEFAULT 'PENDING',
    commit_hash TEXT,
    error_message TEXT,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "GitFileState" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    git_path TEXT NOT NULL,
    last_commit_hash TEXT,
    last_sync_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, path)
);

CREATE TABLE IF NOT EXISTS "GitConflict" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    conflict_type TEXT NOT NULL,
    our_content TEXT,
    their_content TEXT,
    resolved_content TEXT,
    resolved_at TIMESTAMPTZ,
    resolved_by TEXT REFERENCES "User"(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "GitPullRequest" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    connection_id TEXT NOT NULL REFERENCES "GitConnection"(id) ON DELETE CASCADE,
    pr_number INTEGER NOT NULL,
    title TEXT NOT NULL,
    state TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(connection_id, pr_number)
);

CREATE TABLE IF NOT EXISTS "GitPreview" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    pull_request_id TEXT NOT NULL REFERENCES "GitPullRequest"(id) ON DELETE CASCADE,
    deployment_id TEXT REFERENCES "Deployment"(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Integrations
-- ============================================

CREATE TYPE "IntegrationProvider" AS ENUM ('SLACK', 'DISCORD', 'MICROSOFT_TEAMS', 'ZAPIER', 'MAKE', 'WEBHOOK');

CREATE TABLE IF NOT EXISTS "ProjectIntegration" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    provider "IntegrationProvider" NOT NULL,
    name TEXT NOT NULL,
    config JSONB NOT NULL DEFAULT '{}',
    webhook_url TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "IntegrationAuditEvent" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    integration_id TEXT NOT NULL REFERENCES "ProjectIntegration"(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'PENDING',
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "IntegrationConfirmation" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    integration_id TEXT NOT NULL REFERENCES "ProjectIntegration"(id) ON DELETE CASCADE,
    confirmation_token TEXT NOT NULL UNIQUE,
    confirmed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "IntegrationWebhookDelivery" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    integration_id TEXT NOT NULL REFERENCES "ProjectIntegration"(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'PENDING',
    response_status INTEGER,
    error_message TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "IntegrationIdempotencyRecord" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    integration_id TEXT NOT NULL REFERENCES "ProjectIntegration"(id) ON DELETE CASCADE,
    request_id TEXT NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(integration_id, request_id)
);

-- ============================================
-- Reader Access (Private Documentation)
-- ============================================

CREATE TABLE IF NOT EXISTS "Reader" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    email TEXT NOT NULL,
    name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(email)
);

CREATE TABLE IF NOT EXISTS "Audience" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "ReaderAudience" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    reader_id TEXT NOT NULL REFERENCES "Reader"(id) ON DELETE CASCADE,
    audience_id TEXT NOT NULL REFERENCES "Audience"(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(reader_id, audience_id)
);

CREATE TABLE IF NOT EXISTS "AudienceGrant" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    audience_id TEXT NOT NULL REFERENCES "Audience"(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    branch_id TEXT REFERENCES "Branch"(id) ON DELETE CASCADE,
    language_id TEXT REFERENCES "Language"(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "ReaderInvitation" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    audience_id TEXT NOT NULL REFERENCES "Audience"(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    token TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "ReaderSession" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    reader_id TEXT NOT NULL REFERENCES "Reader"(id) ON DELETE CASCADE,
    session_token TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "JwtAccessProvider" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    issuer TEXT NOT NULL,
    audience TEXT NOT NULL,
    secret TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "JwtReplay" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    jwt_id TEXT NOT NULL,
    provider_id TEXT NOT NULL REFERENCES "JwtAccessProvider"(id) ON DELETE CASCADE,
    used_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(jwt_id, provider_id)
);

CREATE TABLE IF NOT EXISTS "ReaderAuditLog" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    reader_id TEXT NOT NULL REFERENCES "Reader"(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Comments
-- ============================================

CREATE TABLE IF NOT EXISTS "Comment" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    page_id TEXT NOT NULL REFERENCES "Page"(id) ON DELETE CASCADE,
    user_id TEXT REFERENCES "User"(id) ON DELETE SET NULL,
    reader_id TEXT REFERENCES "Reader"(id) ON DELETE SET NULL,
    parent_id TEXT REFERENCES "Comment"(id) ON DELETE SET NULL,
    content TEXT NOT NULL,
    resolved BOOLEAN NOT NULL DEFAULT false,
    resolved_at TIMESTAMPTZ,
    resolved_by TEXT REFERENCES "User"(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Search
-- ============================================

CREATE TYPE "SearchIndexRunStatus" AS ENUM ('PENDING', 'PROCESSING', 'COMPLETED', 'FAILED');

CREATE TABLE IF NOT EXISTS "SearchIndexRun" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    branch_id TEXT REFERENCES "Branch"(id) ON DELETE CASCADE,
    language_id TEXT REFERENCES "Language"(id) ON DELETE CASCADE,
    status "SearchIndexRunStatus" NOT NULL DEFAULT 'PENDING',
    pages_indexed INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Vector store for pgvector / embeddings backend
-- ============================================

CREATE TABLE IF NOT EXISTS "PageEmbedding" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    page_id TEXT NOT NULL REFERENCES "Page"(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    embedding REAL[] NOT NULL, -- Dimension for text-embedding-ada-002 (1536 float32 values)
    chunk_text TEXT NOT NULL,
    chunk_index INTEGER NOT NULL DEFAULT 0,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indices for embedding lookups
CREATE INDEX IF NOT EXISTS idx_page_embedding_project_page ON "PageEmbedding"(project_id, page_id);
CREATE INDEX IF NOT EXISTS idx_page_embedding_page ON "PageEmbedding"(page_id);

-- ============================================
-- Exports
-- ============================================

CREATE TYPE "ExportStatus" AS ENUM ('PENDING', 'PROCESSING', 'COMPLETED', 'FAILED');
CREATE TYPE "ExportFormat" AS ENUM ('HTML', 'PDF', 'MARKDOWN', 'EPUB');

CREATE TABLE IF NOT EXISTS "ExportSnapshot" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    branch_id TEXT REFERENCES "Branch"(id) ON DELETE CASCADE,
    language_id TEXT REFERENCES "Language"(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "ExportJob" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    snapshot_id TEXT NOT NULL REFERENCES "ExportSnapshot"(id) ON DELETE CASCADE,
    format "ExportFormat" NOT NULL,
    status "ExportStatus" NOT NULL DEFAULT 'PENDING',
    output_path TEXT,
    error_message TEXT,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "ExportArtifact" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_id TEXT NOT NULL REFERENCES "ExportJob"(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    file_size INTEGER NOT NULL DEFAULT 0,
    storage_path TEXT NOT NULL,
    download_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "ExportSchedule" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    format "ExportFormat" NOT NULL,
    frequency TEXT NOT NULL, -- 'DAILY', 'WEEKLY', 'MONTHLY'
    day_of_week INTEGER,
    day_of_month INTEGER,
    time_of_day TIME NOT NULL DEFAULT '00:00:00',
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_run_at TIMESTAMPTZ,
    next_run_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- OpenAPI Documents
-- ============================================

CREATE TABLE IF NOT EXISTS "OpenApiDocument" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    content TEXT,
    parsed_at TIMESTAMPTZ,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Assets
-- ============================================

CREATE TABLE IF NOT EXISTS "Asset" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    page_id TEXT REFERENCES "Page"(id) ON DELETE CASCADE,
    storage_key TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    content_type TEXT NOT NULL,
    file_size INTEGER NOT NULL DEFAULT 0,
    width INTEGER,
    height INTEGER,
    alt_text TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Notifications
-- ============================================

CREATE TYPE "NotificationType" AS ENUM ('COMMENT', 'INVITATION', 'MENTION', 'SYSTEM', 'EXPORT_COMPLETE', 'DEPLOYMENT_COMPLETE');
CREATE TYPE "NotificationStatus" AS ENUM ('UNREAD', 'READ', 'ARCHIVED');

CREATE TABLE IF NOT EXISTS "Notification" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id TEXT NOT NULL REFERENCES "User"(id) ON DELETE CASCADE,
    notification_type "NotificationType" NOT NULL,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    data JSONB NOT NULL DEFAULT '{}',
    status "NotificationStatus" NOT NULL DEFAULT 'UNREAD',
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Platform Events
-- ============================================

CREATE TABLE IF NOT EXISTS "PlatformEvent" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id TEXT REFERENCES "Organization"(id) ON DELETE CASCADE,
    user_id TEXT REFERENCES "User"(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Usage & Billing
-- ============================================

CREATE TABLE IF NOT EXISTS "UsagePlan" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    description TEXT,
    price INTEGER NOT NULL DEFAULT 0,
    billing_period TEXT NOT NULL, -- 'MONTHLY', 'YEARLY'
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "UsageMeter" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT,
    unit TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "UsagePlanMeter" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    usage_plan_id TEXT NOT NULL REFERENCES "UsagePlan"(id) ON DELETE CASCADE,
    usage_meter_id TEXT NOT NULL REFERENCES "UsageMeter"(id) ON DELETE CASCADE,
    "limit" INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "UsageEntitlement" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    usage_meter_id TEXT NOT NULL REFERENCES "UsageMeter"(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "OrganizationUsagePlan" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id TEXT NOT NULL REFERENCES "Organization"(id) ON DELETE CASCADE,
    usage_plan_id TEXT NOT NULL REFERENCES "UsagePlan"(id) ON DELETE CASCADE,
    starts_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ends_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'ACTIVE',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "AnalyticsEvent" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id TEXT REFERENCES "Organization"(id) ON DELETE CASCADE,
    project_id TEXT REFERENCES "Project"(id) ON DELETE CASCADE,
    user_id TEXT REFERENCES "User"(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    ip_address TEXT,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Checkpoint table for idempotent usage ingestion
CREATE TABLE IF NOT EXISTS "UsageCheckpoint" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    event_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    period_start TIMESTAMPTZ NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(event_type, entity_id, period_start)
);

-- ============================================
-- MCP Audit Events
-- ============================================

CREATE TABLE IF NOT EXISTS "McpAuditEvent" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id TEXT REFERENCES "Organization"(id) ON DELETE CASCADE,
    project_id TEXT REFERENCES "Project"(id) ON DELETE CASCADE,
    user_id TEXT REFERENCES "User"(id) ON DELETE CASCADE,
    operation TEXT NOT NULL,
    request_id TEXT,
    response_status INTEGER,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Project Settings
-- ============================================

CREATE TABLE IF NOT EXISTS "ProjectSettings" (
    project_id TEXT PRIMARY KEY REFERENCES "Project"(id) ON DELETE CASCADE,
    theme TEXT,
    default_language TEXT,
    custom_domain TEXT,
    search_enabled BOOLEAN NOT NULL DEFAULT true,
    comments_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Indices for performance
-- ============================================

-- User indexes
CREATE INDEX IF NOT EXISTS idx_user_email ON "User"(email);
CREATE INDEX IF NOT EXISTS idx_user_created_at ON "User"(created_at);

-- Session indexes
CREATE INDEX IF NOT EXISTS idx_session_user_id ON "Session"(user_id);
CREATE INDEX IF NOT EXISTS idx_session_token ON "Session"(session_token);
CREATE INDEX IF NOT EXISTS idx_session_expires_at ON "Session"(expires_at);

-- Account indexes
CREATE INDEX IF NOT EXISTS idx_account_user_id ON "Account"(user_id);
CREATE INDEX IF NOT EXISTS idx_account_provider ON "Account"(provider, provider_account_id);

-- Organization indexes
CREATE INDEX IF NOT EXISTS idx_organization_slug ON "Organization"(slug);

-- Member indexes
CREATE INDEX IF NOT EXISTS idx_member_user_id ON "Member"(user_id);
CREATE INDEX IF NOT EXISTS idx_member_org_id ON "Member"(organization_id);
CREATE INDEX IF NOT EXISTS idx_member_user_org ON "Member"(user_id, organization_id);

-- Project indexes
CREATE INDEX IF NOT EXISTS idx_project_org_id ON "Project"(organization_id);
CREATE INDEX IF NOT EXISTS idx_project_org_slug ON "Project"(organization_id, slug);

-- Branch indexes
CREATE INDEX IF NOT EXISTS idx_branch_project_id ON "Branch"(project_id);
CREATE INDEX IF NOT EXISTS idx_branch_project_slug ON "Branch"(project_id, slug);

-- Page indexes
CREATE INDEX IF NOT EXISTS idx_page_project_id ON "Page"(project_id);
CREATE INDEX IF NOT EXISTS idx_page_branch_id ON "Page"(branch_id);
CREATE INDEX IF NOT EXISTS idx_page_parent_id ON "Page"(parent_id);
CREATE INDEX IF NOT EXISTS idx_page_project_branch ON "Page"(project_id, branch_id);
CREATE INDEX IF NOT EXISTS idx_page_path ON "Page"(path);
CREATE INDEX IF NOT EXISTS idx_page_slug ON "Page"(slug);
CREATE INDEX IF NOT EXISTS idx_page_position ON "Page"(position);
CREATE INDEX IF NOT EXISTS idx_page_project_branch_slug ON "Page"(project_id, branch_id, slug);

-- Git connection indexes
CREATE INDEX IF NOT EXISTS idx_git_connection_project_id ON "GitConnection"(project_id);

-- Language indexes
CREATE INDEX IF NOT EXISTS idx_language_project_id ON "Language"(project_id);
CREATE INDEX IF NOT EXISTS idx_language_code ON "Language"(code);

-- Reader access indexes
CREATE INDEX IF NOT EXISTS idx_reader_email ON "Reader"(email);
CREATE INDEX IF NOT EXISTS idx_audience_project_id ON "Audience"(project_id);
CREATE INDEX IF NOT EXISTS idx_reader_audience ON "ReaderAudience"(reader_id, audience_id);
CREATE INDEX IF NOT EXISTS idx_audience_grant ON "AudienceGrant"(audience_id, project_id);

-- JwtReplay indexes
CREATE INDEX IF NOT EXISTS idx_jwt_replay ON "JwtReplay"(jwt_id, provider_id);

-- Comment indexes
CREATE INDEX IF NOT EXISTS idx_comment_page_id ON "Comment"(page_id);
CREATE INDEX IF NOT EXISTS idx_comment_parent_id ON "Comment"(parent_id);

-- Search indexes
CREATE INDEX IF NOT EXISTS idx_search_index_project ON "SearchIndexRun"(project_id);

-- Asset indexes
CREATE INDEX IF NOT EXISTS idx_asset_project_id ON "Asset"(project_id);
CREATE INDEX IF NOT EXISTS idx_asset_storage_key ON "Asset"(storage_key);

-- Notification indexes
CREATE INDEX IF NOT EXISTS idx_notification_user_id ON "Notification"(user_id);
CREATE INDEX IF NOT EXISTS idx_notification_status ON "Notification"(status);

-- Analytics indexes
CREATE INDEX IF NOT EXISTS idx_analytics_org_id ON "AnalyticsEvent"(organization_id);
CREATE INDEX IF NOT EXISTS idx_analytics_project_id ON "AnalyticsEvent"(project_id);
CREATE INDEX IF NOT EXISTS idx_analytics_user_id ON "AnalyticsEvent"(user_id);
CREATE INDEX IF NOT EXISTS idx_analytics_created_at ON "AnalyticsEvent"(created_at);

-- Usage checkpoint indexes
CREATE INDEX IF NOT EXISTS idx_usage_checkpoint ON "UsageCheckpoint"(event_type, entity_id, period_start);

-- MCP audit indexes
CREATE INDEX IF NOT EXISTS idx_mcp_audit_org_id ON "McpAuditEvent"(organization_id);
CREATE INDEX IF NOT EXISTS idx_mcp_audit_created_at ON "McpAuditEvent"(created_at);
