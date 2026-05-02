
CREATE TABLE IF NOT EXISTS pending_mpesa_subscription_requests (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    agency_id           UUID        NOT NULL REFERENCES agencies(id) ON DELETE CASCADE,
    plan_slug           TEXT        NOT NULL,
    amount_kes          INTEGER     NOT NULL,
    checkout_request_id TEXT        NOT NULL UNIQUE,
    merchant_request_id TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at          TIMESTAMPTZ NOT NULL DEFAULT now() + INTERVAL '10 minutes'
);

CREATE INDEX IF NOT EXISTS idx_pending_mpesa_sub_checkout
    ON pending_mpesa_subscription_requests (checkout_request_id);

CREATE INDEX IF NOT EXISTS idx_pending_mpesa_sub_agency
    ON pending_mpesa_subscription_requests (agency_id);