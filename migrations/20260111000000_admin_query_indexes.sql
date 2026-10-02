-- Admin dashboards filter and order event rows by their event type, tenant,
-- actor, and timestamp. These indexes keep bounded list/funnel queries from
-- degenerating into full scans as the audit and analytics tables grow.
CREATE INDEX IF NOT EXISTS "PlatformEvent_type_created_at_idx"
    ON "PlatformEvent" (event_type, created_at DESC);

CREATE INDEX IF NOT EXISTS "PlatformEvent_user_created_at_idx"
    ON "PlatformEvent" (user_id, created_at DESC)
    WHERE user_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS "PlatformEvent_org_created_at_idx"
    ON "PlatformEvent" (organization_id, created_at DESC)
    WHERE organization_id IS NOT NULL;
