-- Add config JSONB column to Project table
ALTER TABLE "Project" ADD COLUMN IF NOT EXISTS config JSONB NOT NULL DEFAULT '{}';
