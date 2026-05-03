-- ── Tenant schema portal refactor ─────────────────────────────────────────────
--
-- 1. Add user_id to vendors  (same pattern as owners — links to platform users)
-- 2. Make residents.user_id nullable  (profile can exist before portal invite)
-- 3. Make owners.user_id   nullable  (already nullable, no-op but explicit)

-- 1. vendors.user_id ───────────────────────────────────────────────────────────
ALTER TABLE vendors
    ADD COLUMN IF NOT EXISTS user_id UUID;           -- no FK — cross-schema ref

CREATE INDEX IF NOT EXISTS idx_vendors_user_id ON vendors(user_id);

-- 2. residents.user_id — currently NOT NULL which breaks invite flow ───────────
-- A resident profile can be created before they accept a portal invite.
-- user_id is set when the invite is accepted (activate_user in auth repo).
ALTER TABLE residents
    ALTER COLUMN user_id DROP NOT NULL;

-- 3. owners.user_id — already nullable, make it explicit ─────────────────────
-- (No-op if already nullable — safe to run.)
ALTER TABLE owners
    ALTER COLUMN user_id DROP NOT NULL;


    