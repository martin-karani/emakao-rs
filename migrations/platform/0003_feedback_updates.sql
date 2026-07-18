
-- =============================================================================
-- Feedback Updates — adds status column and feedback_replies table
-- =============================================================================

CREATE TYPE feedback_status AS ENUM (
    'open',
    'in-progress',
    'resolved',
    'closed'
);

ALTER TABLE feedback ADD COLUMN status feedback_status NOT NULL DEFAULT 'open';
ALTER TABLE feedback ADD CONSTRAINT chk_feedback_status CHECK (status IN ('open', 'in-progress', 'resolved', 'closed'));

CREATE TABLE feedback_replies (
    id              UUID                 NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    feedback_id     UUID                 NOT NULL REFERENCES feedback (id) ON DELETE CASCADE,
    user_id         UUID                 NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    message         TEXT                 NOT NULL CHECK (char_length(trim(message)) >= 1),
    created_at      TIMESTAMPTZ          NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ          NOT NULL DEFAULT now()
);

CREATE INDEX idx_feedback_replies_feedback_id ON feedback_replies (feedback_id);
CREATE INDEX idx_feedback_replies_user_id ON feedback_replies (user_id);
CREATE INDEX idx_feedback_replies_created_at ON feedback_replies (created_at DESC);

-- Add updated_at trigger for feedback_replies
DO $$
DECLARE
    tbl TEXT := 'feedback_replies';
BEGIN
    EXECUTE format(
        'CREATE TRIGGER trg_%s_updated_at
         BEFORE UPDATE ON %s
         FOR EACH ROW EXECUTE FUNCTION set_updated_at();',
        tbl, tbl
    );
END;
$$;
