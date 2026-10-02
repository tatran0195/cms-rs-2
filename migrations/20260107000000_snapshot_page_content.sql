-- Normalize large page bodies out of the release metadata document. This keeps
-- shell/version lookups small and lets page-ID lookups use a compact index.
ALTER TABLE "DeploymentSnapshotPageIndex"
    ADD COLUMN IF NOT EXISTS page_content JSONB;

-- Populate the page table for snapshots backfilled by migration 060 and refresh
-- rows written by the earlier app version. Conflict updates also make this
-- migration safe to resume after a transaction rollback.
INSERT INTO "DeploymentSnapshotPageIndex" (
    deployment_id, project_id, branch_id, page_id, language_id, path, kind, page_content
)
SELECT snapshot.deployment_id,
       snapshot.project_id,
       snapshot.branch_id,
       page->>'id',
       NULLIF(page->>'language_id', ''),
       page->>'path',
       COALESCE(page->>'kind', 'PAGE'),
       page
FROM "DeploymentSnapshot" AS snapshot
CROSS JOIN LATERAL jsonb_array_elements(COALESCE(snapshot.snapshot->'pages', '[]'::JSONB)) AS page
WHERE page ? 'id' AND page ? 'path'
ON CONFLICT (deployment_id, page_id) DO UPDATE
SET project_id = EXCLUDED.project_id,
    branch_id = EXCLUDED.branch_id,
    language_id = EXCLUDED.language_id,
    path = EXCLUDED.path,
    kind = EXCLUDED.kind,
    page_content = EXCLUDED.page_content;

ALTER TABLE "DeploymentSnapshotPageIndex"
    ALTER COLUMN page_content SET NOT NULL;

-- Freeze OpenAPI content alongside other release metadata for existing active
-- releases, then remove page bodies from the metadata JSON. The page copies
-- above preserve the exact existing snapshot bytes.
UPDATE "DeploymentSnapshot" AS snapshot
SET snapshot = jsonb_set(
    snapshot.snapshot || jsonb_build_object(
        'openapi', (
            SELECT document.content
            FROM "OpenApiDocument" AS document
            WHERE document.project_id = snapshot.project_id
            ORDER BY document.updated_at DESC, document.created_at DESC
            LIMIT 1
        )
    ),
    '{pages}',
    '[]'::JSONB,
    TRUE
);
