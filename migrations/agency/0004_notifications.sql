-- Migration: 0004_notifications.sql
-- In-app notifications for staff members within an agency.

CREATE TABLE IF NOT EXISTS notifications (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL,
    title      TEXT        NOT NULL,
    body       TEXT        NOT NULL,
    is_read    BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Fast lookup of unread notifications for a specific user
CREATE INDEX IF NOT EXISTS notifications_user_unread_idx
    ON notifications (user_id, is_read)
    WHERE is_read = FALSE;

-- Fast paged listing for a user, newest first
CREATE INDEX IF NOT EXISTS notifications_user_created_idx
    ON notifications (user_id, created_at DESC);
