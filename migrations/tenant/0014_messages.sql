CREATE TABLE IF NOT EXISTS conversations (
    id                   UUID        NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    agency_id            UUID        NOT NULL,
    participant_ids      UUID[]      NOT NULL DEFAULT '{}',
    entity_type          TEXT,
    entity_id            UUID,
    subject              TEXT,
    last_message_preview TEXT,
    last_message_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    is_archived          BOOLEAN     NOT NULL DEFAULT false,
    created_by           UUID        NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_conversations_agency
    ON conversations (agency_id, last_message_at DESC);
CREATE INDEX IF NOT EXISTS idx_conversations_participants
    ON conversations USING GIN (participant_ids);

CREATE TABLE IF NOT EXISTS messages (
    id              UUID        NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    conversation_id UUID        NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    sender_id       UUID        NOT NULL,
    sender_type     TEXT        NOT NULL DEFAULT 'staff'
                                CHECK (sender_type IN ('staff','resident','owner','system')),
    body            TEXT        NOT NULL,
    attachments     JSONB       NOT NULL DEFAULT '[]',
    read_by         JSONB       NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation
    ON messages (conversation_id, created_at ASC);