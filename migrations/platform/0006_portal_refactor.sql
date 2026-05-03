-- ══════════════════════════════════════════════════════════════════════════════
-- 0006_identity_redesign.sql
--
-- Replaces the flat agency_id/role columns on users with a proper membership
-- table so one person can belong to multiple agencies in different roles.
--
-- Changes
-- ───────
-- 1.  Drop old users table (was (id, agency_id, email, password_hash, role)).
-- 2.  Create users          — identity only, no agency/role coupling.
-- 3.  Create user_agency_roles — membership junction.
-- 4.  Create portal_user_index — fast login lookup supporting email OR phone.
-- 5.  Create invite_tokens  — single-use links / temp passwords for both
--                             email and phone-only invitees.
-- 6.  Create refresh_tokens — JWT revocation.
-- ══════════════════════════════════════════════════════════════════════════════

-- ── 0. Safety: rename old table instead of hard-dropping ──────────────────────
ALTER TABLE IF EXISTS users RENAME TO _users_v1_backup;

-- ── 1. users — pure identity, no agency coupling ──────────────────────────────
CREATE TABLE users (
                       id                  UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    -- At least one of email/phone must be non-null (enforced by CHECK below).
                       email               TEXT        UNIQUE,
    -- E.164 format, e.g. +254712345678
                       phone               TEXT        UNIQUE,
                       password_hash       TEXT        NOT NULL DEFAULT '',
                       is_active           BOOLEAN     NOT NULL DEFAULT true,
    -- True for phone-only invites that received a generated temp password.
    -- Login handler returns HTTP 202 with must_change_password=true when set.
                       must_change_password BOOLEAN    NOT NULL DEFAULT false,
                       last_login_at       TIMESTAMPTZ,
                       created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),

                       CONSTRAINT users_must_have_contact
                           CHECK (email IS NOT NULL OR phone IS NOT NULL)
);

CREATE INDEX idx_users_email ON users(email) WHERE email IS NOT NULL;
CREATE INDEX idx_users_phone ON users(phone) WHERE phone IS NOT NULL;

-- ── 2. user_agency_roles — one row per (person × agency × role) ───────────────
-- Allows: same person as 'resident' in agency A and 'manager' in agency B.
-- Also allows: a manager who owns a property in the same agency  →  two rows,
-- one with role='manager', one with role='owner'.
CREATE TABLE user_agency_roles (
                                   id         UUID  PRIMARY KEY DEFAULT uuid_generate_v4(),
                                   user_id    UUID  NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
                                   agency_id  UUID  NOT NULL REFERENCES agencies(id) ON DELETE CASCADE,
                                   role       TEXT  NOT NULL
                                       CHECK (role IN ('admin','manager','agent','resident','owner','vendor','platform_admin')),
                                   is_active  BOOLEAN NOT NULL DEFAULT true,
                                   created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

                                   UNIQUE (user_id, agency_id, role)
);

CREATE INDEX idx_uar_user   ON user_agency_roles(user_id);
CREATE INDEX idx_uar_agency ON user_agency_roles(agency_id);

-- ── 3. portal_user_index ──────────────────────────────────────────────────────
-- Fast O(1) lookup at login time: "which agency does this email/phone belong to
-- on this portal?"
--
-- contact_type: 'email' | 'phone'
-- portal:       'resident' | 'owner' | 'vendor'
-- (Staff always supply agency_slug so they skip this table.)
--
-- PRIMARY KEY allows the same contact to appear on different portals/agencies,
-- but not the SAME portal at two different agencies — i.e. you cannot be a
-- resident at two agencies under the same phone number.  That is intentional:
-- if multi-agency ever becomes a real requirement, remove the PK and add a
-- disambiguation flow at login.
CREATE TABLE portal_user_index (
                                   contact         TEXT NOT NULL,          -- email address or E.164 phone
                                   contact_type    TEXT NOT NULL CHECK (contact_type IN ('email','phone')),
                                   portal          TEXT NOT NULL CHECK (portal IN ('resident','owner','vendor')),
                                   agency_id       UUID NOT NULL REFERENCES agencies(id)  ON DELETE CASCADE,
                                   user_id         UUID NOT NULL REFERENCES users(id)     ON DELETE CASCADE,
                                   membership_id   UUID NOT NULL REFERENCES user_agency_roles(id) ON DELETE CASCADE,

                                   PRIMARY KEY (contact, contact_type, portal)
);

CREATE INDEX idx_pui_contact ON portal_user_index(contact, portal);

-- ── 4. invite_tokens ──────────────────────────────────────────────────────────
-- Generated for every newly invited resident / owner / vendor / staff user.
--
-- Email invites:  token is embedded in a link  →  {portal}.emakao.co.ke/invite/{token}
-- Phone invites:  a random temp_password is generated and sent via SMS, and
--                 is also stored here so accept_invite can verify it without
--                 issuing a full login token until the user sets a real password.
CREATE TABLE invite_tokens (
                               token           TEXT        PRIMARY KEY,         -- 32-char hex (UUID v4 stripped)
                               user_id         UUID        NOT NULL REFERENCES users(id)     ON DELETE CASCADE,
                               agency_id       UUID        NOT NULL REFERENCES agencies(id)  ON DELETE CASCADE,
                               role            TEXT        NOT NULL,
                               portal          TEXT        NOT NULL,
    -- Which contact method was used so the accept page can pre-fill the form.
                               contact         TEXT        NOT NULL,
                               contact_type    TEXT        NOT NULL CHECK (contact_type IN ('email','phone')),
    -- For phone invites: the plain-text temp password (hashed in users.password_hash).
    -- Cleared (set to NULL) after the invite is accepted.
                               temp_password   TEXT,
                               expires_at      TIMESTAMPTZ NOT NULL DEFAULT now() + INTERVAL '48 hours',
                               created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_invite_user ON invite_tokens(user_id);

-- ── 5. refresh_tokens ─────────────────────────────────────────────────────────
-- Access tokens are short-lived (15 min).  Refresh tokens are 30 days.
-- Revoke a session by setting revoked_at; the JTI is also cached in Redis
-- with a TTL matching the access token lifetime for O(1) hot-path checking.
CREATE TABLE refresh_tokens (
                                jti         TEXT        PRIMARY KEY,          -- UUID v4, no dashes
                                user_id     UUID        NOT NULL REFERENCES users(id)     ON DELETE CASCADE,
                                agency_id   UUID        NOT NULL REFERENCES agencies(id)  ON DELETE CASCADE,
                                role        TEXT        NOT NULL,
                                portal      TEXT        NOT NULL,
                                expires_at  TIMESTAMPTZ NOT NULL,
                                revoked_at  TIMESTAMPTZ,                      -- NULL = still valid
                                created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_rt_user    ON refresh_tokens(user_id);
CREATE INDEX idx_rt_expires ON refresh_tokens(expires_at);