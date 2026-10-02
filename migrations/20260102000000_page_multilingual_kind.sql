-- ============================================
-- Page Multilingual & Kind Support Migration
-- ============================================

-- 1. Add missing columns to Page table
ALTER TABLE "Page" ADD COLUMN IF NOT EXISTS language_id TEXT REFERENCES "Language"(id) ON DELETE CASCADE;
ALTER TABLE "Page" ADD COLUMN IF NOT EXISTS kind TEXT NOT NULL DEFAULT 'PAGE';
ALTER TABLE "Page" ADD COLUMN IF NOT EXISTS icon TEXT;
ALTER TABLE "Page" ADD COLUMN IF NOT EXISTS config JSONB;
ALTER TABLE "Page" ADD COLUMN IF NOT EXISTS translation_key TEXT;

-- 2. Backfill language_id for existing pages using project's default language
UPDATE "Page" p
SET language_id = l.id
FROM "Language" l
WHERE p.project_id = l.project_id AND l.is_default = true AND p.language_id IS NULL;

-- Fallback for any remaining pages without language_id
UPDATE "Page" p
SET language_id = (SELECT l.id FROM "Language" l WHERE l.project_id = p.project_id LIMIT 1)
WHERE p.language_id IS NULL;

-- 3. Update unique constraints: drop global branch constraints and replace with language-scoped constraints
ALTER TABLE "Page" DROP CONSTRAINT IF EXISTS "Page_project_id_branch_id_slug_key";
ALTER TABLE "Page" DROP CONSTRAINT IF EXISTS "Page_project_id_branch_id_path_key";

-- Add unique constraint scoped by (project_id, branch_id, language_id, slug) and (project_id, branch_id, language_id, path)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'Page_project_id_branch_id_language_id_slug_key'
    ) THEN
        ALTER TABLE "Page" ADD CONSTRAINT "Page_project_id_branch_id_language_id_slug_key" UNIQUE (project_id, branch_id, language_id, slug);
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'Page_project_id_branch_id_language_id_path_key'
    ) THEN
        ALTER TABLE "Page" ADD CONSTRAINT "Page_project_id_branch_id_language_id_path_key" UNIQUE (project_id, branch_id, language_id, path);
    END IF;
END $$;

-- 4. Create indexes for performance
CREATE INDEX IF NOT EXISTS "Page_language_id_idx" ON "Page"(language_id);
CREATE INDEX IF NOT EXISTS "Page_kind_idx" ON "Page"(kind);
CREATE INDEX IF NOT EXISTS "Page_translation_key_idx" ON "Page"(translation_key);
