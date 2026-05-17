-- migrations/platform/20250116_password_reset_tokens.sql
--
-- Stores short-lived password-reset tokens.
-- Created in the PLATFORM database (user accounts live there).

CREATE TABLE IF NOT EXISTS password_reset_tokens (
    id         UUID        NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash TEXT        NOT NULL UNIQUE,   -- SHA-256 hex of the raw token
    expires_at TIMESTAMPTZ NOT NULL,
    used_at    TIMESTAMPTZ,                   -- set when consumed; NULL = unused
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_prt_token_hash ON password_reset_tokens (token_hash);
CREATE INDEX IF NOT EXISTS idx_prt_user_id    ON password_reset_tokens (user_id);

-- Purge expired tokens automatically (requires pg_cron or a scheduler)
-- If you use pg_cron:
-- SELECT cron.schedule('purge-reset-tokens', '0 4 * * *',
--   $$DELETE FROM password_reset_tokens WHERE expires_at < now() - interval '1 day'$$);