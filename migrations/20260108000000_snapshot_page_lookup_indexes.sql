-- Public page reads select one language's pages from a release; keep this
-- lookup ordered and avoid scanning every language/page for ordinary requests.
CREATE INDEX IF NOT EXISTS "DeploymentSnapshotPageIndex_release_language_path_idx"
    ON "DeploymentSnapshotPageIndex" (deployment_id, language_id, path);
