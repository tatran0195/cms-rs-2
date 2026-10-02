-- Security and durability follow-up.
-- Domain challenge values are generated per hostname and never inferred from a
-- publicly guessable domain id. The rate-limit table prevents OTP resend loops.
ALTER TABLE "Domain"
    ADD COLUMN IF NOT EXISTS verification_token TEXT;

UPDATE "Domain"
SET verification_token = 'cmsrs-v1-' || replace(uuid_generate_v4()::TEXT, '-', '')
WHERE verification_token IS NULL OR verification_token = '';

ALTER TABLE "Domain"
    ALTER COLUMN verification_token SET NOT NULL;

CREATE TABLE IF NOT EXISTS "EmailOtpThrottle" (
    identifier TEXT PRIMARY KEY,
    last_sent_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS "EmailOtpThrottle_last_sent_at_idx"
    ON "EmailOtpThrottle" (last_sent_at);

-- A durable Postgres queue is available even when Redis is not installed.
CREATE TABLE IF NOT EXISTS "CmsJob" (
    id TEXT PRIMARY KEY,
    job_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    locked_by TEXT,
    locked_at TIMESTAMPTZ,
    CONSTRAINT "CmsJob_status_check" CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'retrying'))
);

CREATE INDEX IF NOT EXISTS "CmsJob_ready_idx"
    ON "CmsJob" (available_at, created_at)
    WHERE status IN ('pending', 'retrying');

CREATE INDEX IF NOT EXISTS "CmsJob_processing_idx"
    ON "CmsJob" (locked_at)
    WHERE status = 'processing';
