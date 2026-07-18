
-- =============================================================================
-- Feedback & Support — adds feedback table
-- =============================================================================

CREATE TYPE feedback_type AS ENUM (
    'bug',
    'feature',
    'improvement',
    'other'
);

CREATE TYPE satisfaction_rating AS ENUM (
    'very-dissatisfied',
    'dissatisfied',
    'neutral',
    'satisfied',
    'very-satisfied'
);

CREATE TABLE feedback (
    id              UUID                 NOT NULL PRIMARY KEY DEFAULT uuidv7(),
    agency_id       UUID                 NOT NULL REFERENCES agencies (id) ON DELETE CASCADE,
    user_id         UUID                 NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    feedback_type   feedback_type        NOT NULL,
    satisfaction    satisfaction_rating,
    subject         TEXT                 NOT NULL CHECK (char_length(trim(subject)) >= 5),
    description     TEXT                 NOT NULL CHECK (char_length(trim(description)) >= 20),
    email           TEXT,
    created_at      TIMESTAMPTZ          NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ          NOT NULL DEFAULT now()
);

CREATE INDEX idx_feedback_agency_id ON feedback (agency_id);
CREATE INDEX idx_feedback_user_id ON feedback (user_id);
CREATE INDEX idx_feedback_created_at ON feedback (created_at DESC);

-- Add updated_at trigger
DO $$
DECLARE
    tbl TEXT := 'feedback';
BEGIN
    EXECUTE format(
        'CREATE TRIGGER trg_%s_updated_at
         BEFORE UPDATE ON %s
         FOR EACH ROW EXECUTE FUNCTION set_updated_at();',
        tbl, tbl
    );
END;
$$;
