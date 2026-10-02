-- Make language metadata match the product contract and keep the language
-- switcher/reader surfaces backed by persisted data rather than fabricated
-- response defaults.
ALTER TABLE "Language"
    ADD COLUMN IF NOT EXISTS enabled BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS position INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS config JSONB;

-- Older deployments could contain no default or more than one default because
-- the first implementation updated the flag outside a transaction. Repair those
-- rows deterministically before installing the invariant index.
WITH winners AS (
    SELECT DISTINCT ON (project_id) id, project_id
    FROM "Language"
    ORDER BY project_id, is_default DESC, created_at ASC, id ASC
)
UPDATE "Language" AS language
SET is_default = (language.id = winners.id)
FROM winners
WHERE language.project_id = winners.project_id;

-- Give existing rows a stable display order. Preserve the default-first order,
-- then the historical creation order; new languages append after the max.
WITH ordered AS (
    SELECT id,
           ROW_NUMBER() OVER (
               PARTITION BY project_id
               ORDER BY is_default DESC, created_at ASC, id ASC
           ) - 1 AS next_position
    FROM "Language"
)
UPDATE "Language" AS language
SET position = ordered.next_position
FROM ordered
WHERE language.id = ordered.id;

-- A project may have zero languages if an earlier partial project-create flow
-- swallowed the default-language insert error. Repair it with an English default
-- so language-scoped page creation/publication has a safe fallback.
INSERT INTO "Language" (
    id, project_id, code, name, is_default, is_rtl, enabled, position, created_at, updated_at
)
SELECT uuid_generate_v4()::TEXT, project.id, 'en', 'English', TRUE, FALSE, TRUE, 0, NOW(), NOW()
FROM "Project" AS project
WHERE NOT EXISTS (
    SELECT 1 FROM "Language" AS language WHERE language.project_id = project.id
);

-- Backfill any pages whose old project had no language row before the repair.
UPDATE "Page" AS page
SET language_id = language.id
FROM "Language" AS language
WHERE page.project_id = language.project_id
  AND language.is_default = TRUE
  AND page.language_id IS NULL;

-- Exactly one default at most. Application writes serialize on the owning
-- project row and create the first default in the same transaction.
CREATE UNIQUE INDEX IF NOT EXISTS "Language_one_default_per_project_idx"
    ON "Language" (project_id)
    WHERE is_default = TRUE;

-- Language tags are case-insensitive per BCP-47. This also protects concurrent
-- creates that race past the business-layer existence check.
CREATE UNIQUE INDEX IF NOT EXISTS "Language_project_code_ci_idx"
    ON "Language" (project_id, lower(code));

CREATE INDEX IF NOT EXISTS "Language_project_position_idx"
    ON "Language" (project_id, position, created_at);

CREATE INDEX IF NOT EXISTS "Page_project_branch_language_position_idx"
    ON "Page" (project_id, branch_id, language_id, position, created_at);

-- Public reads and branch deployments only enumerate published document pages.
-- Keep this partial index small on large documentation trees.
CREATE INDEX IF NOT EXISTS "Page_published_documents_scope_idx"
    ON "Page" (project_id, branch_id, language_id, path)
    WHERE is_published = TRUE AND kind = 'PAGE';
