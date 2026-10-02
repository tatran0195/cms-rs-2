-- Persist the exact content tree used by each successful release. Public reads
-- and rollback use this snapshot instead of mutable editor tables.
CREATE TABLE IF NOT EXISTS "DeploymentSnapshot" (
    id TEXT PRIMARY KEY DEFAULT uuid_generate_v4()::TEXT,
    deployment_id TEXT NOT NULL UNIQUE REFERENCES "Deployment"(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    branch_id TEXT NOT NULL,
    version BIGINT NOT NULL CHECK (version > 0),
    snapshot JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, branch_id, version)
);

CREATE INDEX IF NOT EXISTS "DeploymentSnapshot_project_branch_version_idx"
    ON "DeploymentSnapshot" (project_id, branch_id, version DESC);

-- A compact lookup index lets page-ID routes find retained release content
-- without scanning/GIN-indexing full Markdown bodies in the snapshot JSON.
CREATE TABLE IF NOT EXISTS "DeploymentSnapshotPageIndex" (
    deployment_id TEXT NOT NULL REFERENCES "DeploymentSnapshot"(deployment_id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES "Project"(id) ON DELETE CASCADE,
    branch_id TEXT NOT NULL,
    page_id TEXT NOT NULL,
    language_id TEXT,
    path TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'PAGE',
    PRIMARY KEY (deployment_id, page_id)
);

CREATE INDEX IF NOT EXISTS "DeploymentSnapshotPageIndex_page_project_idx"
    ON "DeploymentSnapshotPageIndex" (page_id, project_id, deployment_id);

-- Backfill active releases so an upgrade does not make an already-published
-- project disappear. These records capture the current database state at the
-- migration cutover; subsequent releases are captured atomically by the app.
WITH active AS (
    SELECT deployment.id,
           deployment.project_id,
           deployment.branch_id,
           ROW_NUMBER() OVER (
               PARTITION BY deployment.project_id, deployment.branch_id
               ORDER BY deployment.deployed_at NULLS FIRST,
                        deployment.created_at ASC,
                        deployment.id ASC
           ) AS version
    FROM "Deployment" AS deployment
    WHERE deployment.status = 'ACTIVE'
      AND deployment.branch_id IS NOT NULL
)
INSERT INTO "DeploymentSnapshot" (
    id, deployment_id, project_id, branch_id, version, snapshot, created_at
)
SELECT uuid_generate_v4()::TEXT,
       active.id,
       active.project_id,
       active.branch_id,
       active.version,
       jsonb_build_object(
           'project', to_jsonb(project),
           'branch', to_jsonb(branch),
           'branches', COALESCE((
               SELECT jsonb_agg(to_jsonb(all_branches) ORDER BY all_branches.is_default DESC, all_branches.created_at)
               FROM "Branch" AS all_branches
               WHERE all_branches.project_id = active.project_id
           ), '[]'::JSONB),
           'languages', COALESCE((
               SELECT jsonb_agg(to_jsonb(language) ORDER BY language.position, language.created_at)
               FROM "Language" AS language
               WHERE language.project_id = active.project_id AND language.enabled = TRUE
           ), '[]'::JSONB),
           'translations', COALESCE((
               SELECT jsonb_agg(to_jsonb(translation) ORDER BY translation.created_at)
               FROM "ProjectTranslation" AS translation
               JOIN "Language" AS language ON language.id = translation.language_id
               WHERE translation.project_id = active.project_id AND language.enabled = TRUE
           ), '[]'::JSONB),
           'pages', COALESCE((
               SELECT jsonb_agg(to_jsonb(page) ORDER BY page.language_id, page.path)
               FROM "Page" AS page
               JOIN "Language" AS language ON language.id = page.language_id
               WHERE page.project_id = active.project_id
                 AND page.branch_id = active.branch_id
                 AND page.is_published = TRUE
                 AND language.enabled = TRUE
           ), '[]'::JSONB)
       ),
       COALESCE(active_timestamp.deployed_at, NOW())
FROM active
JOIN "Project" AS project ON project.id = active.project_id
JOIN "Branch" AS branch ON branch.id = active.branch_id
JOIN "Deployment" AS active_timestamp ON active_timestamp.id = active.id
ON CONFLICT (deployment_id) DO NOTHING;
