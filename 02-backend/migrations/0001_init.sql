-- Stade Ali Ammar — initial schema
-- Source tags: [M]=master spec, [C1]=CDC1, [C2]=CDC2, [X]=capacity sheet.
-- Seat model: ZONE-QUOTA counting (blueprint §2 #4 / open question). Numbered
-- seats can be added later via a seat table referencing tribune_section.

CREATE EXTENSION IF NOT EXISTS "pgcrypto";  -- gen_random_uuid()

-- ─────────────────────────────────────────────────────────────────────────
-- Reference / seating  [M §7.3][X]
-- ─────────────────────────────────────────────────────────────────────────
CREATE TYPE zone_kind AS ENUM
  ('public','vip','vvip','official','visitor','buffer','press');

CREATE TABLE zone (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    code          text NOT NULL UNIQUE,
    name          text NOT NULL,
    kind          zone_kind NOT NULL,
    sellable      boolean NOT NULL DEFAULT true,
    display_order int NOT NULL DEFAULT 0
);

CREATE TABLE tribune_section (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    zone_id    uuid NOT NULL REFERENCES zone(id),
    label      text NOT NULL,
    level      text,                 -- 'superieur' | 'inferieur' | NULL [X]
    gate_range text,                 -- e.g. 'V19-V25' [X]
    capacity   int  NOT NULL CHECK (capacity >= 0),
    sellable   boolean NOT NULL DEFAULT true
);

-- ─────────────────────────────────────────────────────────────────────────
-- Users, agents, RBAC  [M §7.1.2, §7.1.3, §9.1]
-- ─────────────────────────────────────────────────────────────────────────
CREATE TYPE user_status AS ENUM ('active','locked','blacklisted');
CREATE TYPE locale_t    AS ENUM ('ar','fr','en');   -- Arabic primary [M §13]

CREATE TABLE user_account (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email          text NOT NULL UNIQUE,
    password_hash  text NOT NULL,                      -- argon2id [M §10.2]
    full_name      text NOT NULL,
    personal_info  jsonb NOT NULL DEFAULT '{}',
    id_document_ref text,                              -- optional upload [M §7.1.2]
    locale         locale_t NOT NULL DEFAULT 'ar',
    status         user_status NOT NULL DEFAULT 'active',
    twofa_enabled  boolean NOT NULL DEFAULT false,     -- [M §7.1.3]
    twofa_channel  text,                               -- 'sms' | 'totp'
    totp_secret    text,
    created_at     timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE role (
    id   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    code text NOT NULL UNIQUE,                         -- 'admin','operator','gate_agent',...
    name text NOT NULL
);

CREATE TABLE permission (
    id   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    code text NOT NULL UNIQUE                          -- 'event.create','ticket.export',...
);

CREATE TABLE role_permission (
    role_id       uuid NOT NULL REFERENCES role(id) ON DELETE CASCADE,
    permission_id uuid NOT NULL REFERENCES permission(id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE agent (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      uuid NOT NULL REFERENCES user_account(id),
    employee_ref text
);

CREATE TABLE agent_role (
    agent_id uuid NOT NULL REFERENCES agent(id) ON DELETE CASCADE,
    role_id  uuid NOT NULL REFERENCES role(id)  ON DELETE CASCADE,
    PRIMARY KEY (agent_id, role_id)
);

-- Tokenised payment method — NEVER store PAN/CVV [M §7.4, §10.4]
CREATE TABLE payment_method_token (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       uuid NOT NULL REFERENCES user_account(id) ON DELETE CASCADE,
    scheme        text NOT NULL,                       -- 'cib' | 'dahabia'
    gateway_token text NOT NULL,
    last4         text,
    exp           text,
    created_at    timestamptz NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────────────────
-- Events & per-event zone config  [M §7.2, §7.3]
-- ─────────────────────────────────────────────────────────────────────────
CREATE TYPE event_status AS ENUM
  ('draft','published','sales_open','sales_closed','closed','cancelled','postponed');
CREATE TYPE ticket_category AS ENUM ('local','visitor','vip','vvip','official');

CREATE TABLE event (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    title           text NOT NULL,
    kind            text NOT NULL DEFAULT 'match',
    opponent_home   text,
    opponent_away   text,
    venue           text NOT NULL DEFAULT 'Stade Ali Ammar, Douira',
    starts_at       timestamptz NOT NULL,
    capacity        int,
    status          event_status NOT NULL DEFAULT 'draft',
    entry_conditions text,
    created_by      uuid REFERENCES user_account(id),
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE event_zone_config (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id        uuid NOT NULL REFERENCES event(id) ON DELETE CASCADE,
    zone_id         uuid NOT NULL REFERENCES zone(id),
    active          boolean NOT NULL DEFAULT true,
    quota           int NOT NULL CHECK (quota >= 0),
    price_dzd       numeric(12,2) NOT NULL DEFAULT 0,
    ticket_category ticket_category NOT NULL,
    UNIQUE (event_id, zone_id)
);

-- ─────────────────────────────────────────────────────────────────────────
-- Seat locking, orders, tickets, payments  [M §7.1.4, §7.1.5, §7.1.6, §7.4]
-- ─────────────────────────────────────────────────────────────────────────
CREATE TYPE lock_status AS ENUM ('held','converted','expired');

-- Durable mirror of the Redis TTL lock [M §7.1.4]
CREATE TABLE seat_lock (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id     uuid NOT NULL REFERENCES event(id),
    zone_id      uuid NOT NULL REFERENCES zone(id),
    user_id      uuid NOT NULL REFERENCES user_account(id),
    qty          int  NOT NULL CHECK (qty BETWEEN 1 AND 5),  -- [M §7.1.4]
    locked_until timestamptz NOT NULL,
    status       lock_status NOT NULL DEFAULT 'held'
);

CREATE TYPE order_status AS ENUM ('pending','paid','failed','refunded','cancelled');

CREATE TABLE "order" (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    uuid NOT NULL REFERENCES user_account(id),
    event_id   uuid NOT NULL REFERENCES event(id),
    total_dzd  numeric(12,2) NOT NULL DEFAULT 0,
    status     order_status NOT NULL DEFAULT 'pending',
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TYPE ticket_type   AS ENUM ('local','visitor','vip','cashier_pdf','season');
CREATE TYPE ticket_status AS ENUM
  ('valid','invalid','used','cancelled','blocked','out_of_zone');  -- [M §8.3]

CREATE TABLE ticket (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id    uuid REFERENCES "order"(id),
    event_id    uuid NOT NULL REFERENCES event(id),
    zone_id     uuid NOT NULL REFERENCES zone(id),
    section_id  uuid REFERENCES tribune_section(id),
    holder_name text NOT NULL,
    type        ticket_type NOT NULL,
    qr_payload  text NOT NULL,                          -- signed payload [M §7.1.6]
    qr_signature text NOT NULL,
    status      ticket_status NOT NULL DEFAULT 'valid',
    issued_at   timestamptz NOT NULL DEFAULT now(),
    used_at     timestamptz,
    used_gate   text
);

CREATE TYPE txn_status AS ENUM ('accepted','refused','pending','failed');

CREATE TABLE payment_transaction (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id        uuid NOT NULL REFERENCES "order"(id),
    gateway         text NOT NULL,                      -- 'cib'|'dahabia'|'baridimob'
    gateway_txn_ref text,
    amount_dzd      numeric(12,2) NOT NULL,
    status          txn_status NOT NULL DEFAULT 'pending',
    raw_callback    jsonb,
    created_at      timestamptz NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────────────────
-- Subscriptions & VIP invitations  [M §7.5, §8.4]
-- ─────────────────────────────────────────────────────────────────────────
CREATE TABLE subscription (
    id        uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id   uuid NOT NULL REFERENCES user_account(id),
    season    text NOT NULL,
    card_ref  text NOT NULL UNIQUE,
    zone_id   uuid REFERENCES zone(id),
    status    text NOT NULL DEFAULT 'active',
    issued_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE vip_invitation (
    id                   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id             uuid NOT NULL REFERENCES event(id),
    tribune              text NOT NULL CHECK (tribune IN ('A','B','C')),  -- [M §8.4]
    holder_name          text NOT NULL,
    qr_payload           text NOT NULL,
    qr_signature         text NOT NULL,
    status               ticket_status NOT NULL DEFAULT 'valid',
    second_check_required boolean NOT NULL DEFAULT false
);

-- ─────────────────────────────────────────────────────────────────────────
-- Access logs, audit, fraud  [M §8.1, §8.2, §8.5, §9.1, §10.5]
-- ─────────────────────────────────────────────────────────────────────────
CREATE TABLE access_log (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    ticket_id  uuid REFERENCES ticket(id),              -- null for invalid scan
    event_id   uuid NOT NULL REFERENCES event(id),
    gate_code  text NOT NULL,
    device_id  text NOT NULL,
    result     ticket_status NOT NULL,                  -- scan outcome [M §8.3]
    scanned_at timestamptz NOT NULL,
    synced     boolean NOT NULL DEFAULT true,           -- offline→online [M §8.2]
    source     text NOT NULL DEFAULT 'online'           -- 'online'|'offline'
);
-- Enforces the double-scan rule at reconciliation [M §8.2]: one accepted entry
-- per ticket; later duplicates are rejected and logged as fraud_event.
CREATE UNIQUE INDEX one_valid_entry_per_ticket
    ON access_log (ticket_id)
    WHERE result = 'valid' AND ticket_id IS NOT NULL;

CREATE TABLE audit_log (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id   uuid REFERENCES agent(id),
    action     text NOT NULL,
    entity     text NOT NULL,
    entity_id  uuid,
    before     jsonb,
    after      jsonb,
    ip         inet,
    occurred_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE fraud_event (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    type       text NOT NULL,                           -- multi_purchase|ip_abuse|scan_fraud|duplicate
    user_id    uuid REFERENCES user_account(id),
    ip         inet,
    detail     jsonb,
    occurred_at timestamptz NOT NULL DEFAULT now()
);

-- Helpful indexes
CREATE INDEX idx_ticket_event   ON ticket(event_id);
CREATE INDEX idx_ticket_status  ON ticket(status);
CREATE INDEX idx_order_user     ON "order"(user_id);
CREATE INDEX idx_access_event   ON access_log(event_id);
CREATE INDEX idx_ezc_event      ON event_zone_config(event_id);
