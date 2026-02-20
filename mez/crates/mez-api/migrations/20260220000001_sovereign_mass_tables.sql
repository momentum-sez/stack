-- Sovereign Mass Persistence Layer — ADR-007
--
-- These tables store Mass primitive data when a zone operates in sovereign mode
-- (SOVEREIGN_MASS=true). Each zone persists its own entity, fiscal, consent,
-- ownership, identity, and templating data in Postgres alongside the EZ Stack
-- corridor/asset/attestation tables.
--
-- This migration exists only for sovereign deployments. In proxy mode
-- (SOVEREIGN_MASS=false), these tables exist but remain empty.

-- ────────────────────────────────────────────────────────────────
-- Organizations — Mass entity primitive
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS mass_organizations (
    id              UUID PRIMARY KEY,
    name            TEXT NOT NULL DEFAULT '',
    jurisdiction    TEXT,
    status          TEXT NOT NULL DEFAULT 'ACTIVE',
    tags            JSONB NOT NULL DEFAULT '[]',
    address         JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mass_organizations_status ON mass_organizations(status);

-- ────────────────────────────────────────────────────────────────
-- Treasuries — Mass fiscal primitive (treasury)
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS mass_treasuries (
    id              UUID PRIMARY KEY,
    entity_id       TEXT NOT NULL DEFAULT '',
    name            TEXT,
    status          TEXT NOT NULL DEFAULT 'ACTIVE',
    context         TEXT,
    reference_id    TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mass_treasuries_entity_id ON mass_treasuries(entity_id);
CREATE INDEX IF NOT EXISTS idx_mass_treasuries_status ON mass_treasuries(status);

-- ────────────────────────────────────────────────────────────────
-- Accounts — Mass fiscal primitive (account)
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS mass_accounts (
    id              UUID PRIMARY KEY,
    entity_id       TEXT,
    treasury_id     UUID,
    name            TEXT NOT NULL DEFAULT 'Default Account',
    currency        TEXT NOT NULL DEFAULT 'PKR',
    balance         TEXT NOT NULL DEFAULT '0.00',
    available       TEXT NOT NULL DEFAULT '0.00',
    status          TEXT NOT NULL DEFAULT 'ACTIVE',
    funding_details JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mass_accounts_entity_id ON mass_accounts(entity_id);
CREATE INDEX IF NOT EXISTS idx_mass_accounts_treasury_id ON mass_accounts(treasury_id);
CREATE INDEX IF NOT EXISTS idx_mass_accounts_status ON mass_accounts(status);

-- ────────────────────────────────────────────────────────────────
-- Transactions — Mass fiscal primitive (payment/transaction)
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS mass_transactions (
    id                  UUID PRIMARY KEY,
    account_id          TEXT,
    entity_id           TEXT,
    transaction_type    TEXT NOT NULL DEFAULT 'PAYMENT',
    status              TEXT NOT NULL DEFAULT 'PENDING',
    direction           TEXT NOT NULL DEFAULT 'OUTBOUND',
    currency            TEXT NOT NULL DEFAULT 'PKR',
    amount              TEXT,
    reference           TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mass_transactions_entity_id ON mass_transactions(entity_id);
CREATE INDEX IF NOT EXISTS idx_mass_transactions_status ON mass_transactions(status);

-- ────────────────────────────────────────────────────────────────
-- Tax Events (sovereign) — Mass fiscal tax events
-- Named mass_tax_events_sovereign to distinguish from EZ Stack tax_events.
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS mass_tax_events_sovereign (
    id              UUID PRIMARY KEY,
    entity_id       TEXT NOT NULL DEFAULT '',
    event_type      TEXT NOT NULL DEFAULT 'UNKNOWN',
    amount          TEXT NOT NULL DEFAULT '0',
    currency        TEXT NOT NULL DEFAULT 'PKR',
    tax_year        TEXT,
    details         JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mass_tax_events_sov_entity_id ON mass_tax_events_sovereign(entity_id);

-- ────────────────────────────────────────────────────────────────
-- Consents — Mass consent primitive
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS mass_consents (
    id                      UUID PRIMARY KEY,
    organization_id         TEXT NOT NULL DEFAULT '',
    operation_id            TEXT,
    operation_type          TEXT,
    status                  TEXT NOT NULL DEFAULT 'PENDING',
    votes                   JSONB NOT NULL DEFAULT '[]',
    num_votes_required      INT,
    approval_count          INT NOT NULL DEFAULT 0,
    rejection_count         INT NOT NULL DEFAULT 0,
    requested_by            TEXT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mass_consents_organization_id ON mass_consents(organization_id);
CREATE INDEX IF NOT EXISTS idx_mass_consents_status ON mass_consents(status);

-- ────────────────────────────────────────────────────────────────
-- Cap Tables — Mass ownership primitive
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS mass_cap_tables (
    id                      UUID PRIMARY KEY,
    organization_id         TEXT NOT NULL DEFAULT '',
    authorized_shares       BIGINT NOT NULL DEFAULT 0,
    outstanding_shares      BIGINT NOT NULL DEFAULT 0,
    fully_diluted_shares    BIGINT NOT NULL DEFAULT 0,
    reserved_shares         BIGINT NOT NULL DEFAULT 0,
    unreserved_shares       BIGINT NOT NULL DEFAULT 0,
    share_classes           JSONB NOT NULL DEFAULT '[]',
    shareholders            JSONB NOT NULL DEFAULT '[]',
    options_pools           JSONB NOT NULL DEFAULT '[]',
    par_value               TEXT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mass_cap_tables_organization_id ON mass_cap_tables(organization_id);

-- ────────────────────────────────────────────────────────────────
-- Investments — Mass ownership primitive
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS mass_investments (
    id              UUID PRIMARY KEY,
    payload         JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ────────────────────────────────────────────────────────────────
-- Templates — Mass templating primitive (string-keyed)
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS mass_templates (
    id              TEXT PRIMARY KEY,
    name            TEXT,
    context         TEXT,
    entity_id       TEXT,
    version         TEXT,
    type_field      TEXT,
    grouping        TEXT,
    status          TEXT NOT NULL DEFAULT 'ACTIVE'
);

-- ────────────────────────────────────────────────────────────────
-- Submissions — Mass templating primitive (string-keyed)
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS mass_submissions (
    id              TEXT PRIMARY KEY,
    entity_id       TEXT,
    context         TEXT,
    status          TEXT NOT NULL DEFAULT 'PENDING',
    signing_order   TEXT,
    signers         JSONB NOT NULL DEFAULT '[]',
    document_uri    TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ────────────────────────────────────────────────────────────────
-- Members — Mass identity primitive (org-keyed)
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS mass_members (
    id              UUID PRIMARY KEY,
    org_id          TEXT NOT NULL,
    payload         JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mass_members_org_id ON mass_members(org_id);

-- ────────────────────────────────────────────────────────────────
-- Board Members — Mass identity primitive (org-keyed)
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS mass_board_members (
    id              UUID PRIMARY KEY,
    org_id          TEXT NOT NULL,
    payload         JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mass_board_members_org_id ON mass_board_members(org_id);

-- ────────────────────────────────────────────────────────────────
-- Shareholders — Mass identity primitive (org-keyed)
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS mass_shareholders (
    id              UUID PRIMARY KEY,
    org_id          TEXT NOT NULL,
    payload         JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mass_shareholders_org_id ON mass_shareholders(org_id);
