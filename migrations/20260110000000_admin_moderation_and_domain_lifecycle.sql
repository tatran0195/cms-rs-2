-- Platform operator controls and moderation state.
ALTER TABLE "User"
    ADD COLUMN IF NOT EXISTS role TEXT NOT NULL DEFAULT 'user',
    ADD COLUMN IF NOT EXISTS suspended_at TIMESTAMPTZ;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'User_role_check'
          AND conrelid = '"User"'::regclass
    ) THEN
        ALTER TABLE "User"
            ADD CONSTRAINT "User_role_check" CHECK (role IN ('user', 'admin'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS "User_role_created_at_idx"
    ON "User" (role, created_at DESC);

CREATE INDEX IF NOT EXISTS "User_suspended_at_idx"
    ON "User" (suspended_at) WHERE suspended_at IS NOT NULL;

ALTER TABLE "Project"
    ADD COLUMN IF NOT EXISTS takedown_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS takedown_reason TEXT;

CREATE INDEX IF NOT EXISTS "Project_takedown_at_idx"
    ON "Project" (takedown_at) WHERE takedown_at IS NOT NULL;

-- TLS lifecycle bookkeeping is separate from ownership verification. A cert is
-- not considered active merely because a domain passed its DNS challenge.
ALTER TABLE "Domain"
    ADD COLUMN IF NOT EXISTS ssl_status TEXT NOT NULL DEFAULT 'PENDING',
    ADD COLUMN IF NOT EXISTS ssl_last_error TEXT,
    ADD COLUMN IF NOT EXISTS ssl_checked_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS acme_order_url TEXT;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'Domain_ssl_status_check'
          AND conrelid = '"Domain"'::regclass
    ) THEN
        ALTER TABLE "Domain"
            ADD CONSTRAINT "Domain_ssl_status_check"
            CHECK (ssl_status IN ('PENDING', 'PROVISIONING', 'ACTIVE', 'ERROR', 'EXPIRED'));
    END IF;
END $$;

UPDATE "Domain"
SET ssl_status = CASE
    WHEN ssl_certificate IS NOT NULL
      AND ssl_certificate_expires_at > NOW() THEN 'ACTIVE'
    WHEN ssl_certificate_expires_at IS NOT NULL
      AND ssl_certificate_expires_at <= NOW() THEN 'EXPIRED'
    ELSE 'PENDING'
END;

CREATE INDEX IF NOT EXISTS "Domain_ssl_lifecycle_idx"
    ON "Domain" (ssl_status, ssl_certificate_expires_at);
