-- Keep the fields needed for listings and sitemaps outside the JSON document
-- body so host routes do not fetch or decompress Markdown just to build links.
ALTER TABLE "DeploymentSnapshotPageIndex"
    ADD COLUMN IF NOT EXISTS title TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS slug TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS parent_id TEXT,
    ADD COLUMN IF NOT EXISTS is_published BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS page_updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Backfill older immutable releases from their stored page JSON. Preserve the
-- release timestamp as a safe last-modified fallback for legacy rows without a
-- per-page updated_at value.
UPDATE "DeploymentSnapshotPageIndex" AS page_index
SET kind = UPPER(COALESCE(NULLIF(page_index.kind, ''), 'PAGE')),
    title = COALESCE(page_index.page_content->>'title', ''),
    slug = COALESCE(page_index.page_content->>'slug', ''),
    parent_id = NULLIF(page_index.page_content->>'parent_id', ''),
    is_published = COALESCE((page_index.page_content->>'is_published')::BOOLEAN, FALSE),
    page_updated_at = COALESCE(
        NULLIF(page_index.page_content->>'updated_at', '')::TIMESTAMPTZ,
        snapshot.created_at
    )
FROM "DeploymentSnapshot" AS snapshot
WHERE snapshot.deployment_id = page_index.deployment_id;

CREATE INDEX IF NOT EXISTS "DeploymentSnapshotPageIndex_published_listing_idx"
    ON "DeploymentSnapshotPageIndex" (deployment_id, language_id, path)
    INCLUDE (page_id, title, slug, parent_id, page_updated_at)
    WHERE kind = 'PAGE' AND is_published = TRUE;
