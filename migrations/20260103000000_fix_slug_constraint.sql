-- Fix slug uniqueness: slugs are only unique per path, not globally per language.
-- Drop the overly strict slug unique constraint; the path constraint is sufficient.
ALTER TABLE "Page" DROP CONSTRAINT IF EXISTS "Page_project_id_branch_id_language_id_slug_key";
ALTER TABLE "Page" DROP CONSTRAINT IF EXISTS "Page_project_id_branch_id_slug_key";
