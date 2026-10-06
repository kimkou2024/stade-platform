-- Additional requirements: password reset [M §7.1.2], consent [M §10.1],
-- notifications log [M §7.6], access alerts [M §8.5].

-- Consent (Loi 18-07) [M §10.1]
ALTER TABLE user_account ADD COLUMN IF NOT EXISTS consent_at timestamptz;

CREATE TABLE IF NOT EXISTS password_reset (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    uuid NOT NULL REFERENCES user_account(id) ON DELETE CASCADE,
    token      text NOT NULL UNIQUE,
    expires_at timestamptz NOT NULL,
    used       boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now()
);

-- Notifications sent [M §7.6] (channel: email|sms|push)
CREATE TABLE IF NOT EXISTS notification_log (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    uuid REFERENCES user_account(id) ON DELETE SET NULL,
    channel    text NOT NULL,
    subject    text NOT NULL,
    body       text NOT NULL,
    status     text NOT NULL DEFAULT 'queued',  -- queued|sent|failed
    created_at timestamptz NOT NULL DEFAULT now()
);

-- Access alerts [M §8.5]
CREATE TABLE IF NOT EXISTS access_alert (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id   uuid NOT NULL REFERENCES event(id) ON DELETE CASCADE,
    type       text NOT NULL,                    -- capacity_exceeded | high_invalid_rate
    detail     jsonb,
    created_at timestamptz NOT NULL DEFAULT now()
);
