-- MEZ Database Initialization
-- Schema matching SQLx migrations for the Rust mez-api binary.
--
-- This script runs once on first PostgreSQL startup. Subsequent schema
-- changes are managed by SQLx migrations embedded in the mez-api binary
-- (sqlx::migrate!("./migrations")).
--
-- Database layout:
--   mez (default)  — entities, licensing, identity, compliance, audit, consent
--   mez_corridor   — corridor state, receipts, MMR chain
--   mez_settlement — settlement plans, transactions, netting

-- ============================================
-- Create additional databases
-- ============================================

CREATE DATABASE mez_corridor;
CREATE DATABASE mez_settlement;

GRANT ALL PRIVILEGES ON DATABASE mez_corridor TO mez;
GRANT ALL PRIVILEGES ON DATABASE mez_settlement TO mez;

-- ============================================
-- Main database: mez
-- ============================================

\c mez

-- Schemas
CREATE SCHEMA IF NOT EXISTS entity;
CREATE SCHEMA IF NOT EXISTS licensing;
CREATE SCHEMA IF NOT EXISTS identity;
CREATE SCHEMA IF NOT EXISTS compliance;
CREATE SCHEMA IF NOT EXISTS tax;
CREATE SCHEMA IF NOT EXISTS consent;
CREATE SCHEMA IF NOT EXISTS audit;

-- -------------------------------------------
-- Entities (Five Primitives: ENTITIES)
-- -------------------------------------------

CREATE TABLE IF NOT EXISTS entity.entities (
    entity_id       VARCHAR(255) PRIMARY KEY,
    entity_type     VARCHAR(50) NOT NULL,
    legal_name      VARCHAR(500) NOT NULL,
    registration_number VARCHAR(100),
    jurisdiction    VARCHAR(50) NOT NULL,
    status          VARCHAR(50) NOT NULL DEFAULT 'draft',
    formed_date     DATE,
    did             VARCHAR(255),
    -- Pakistan GovOS: National Tax Number for FBR IRIS integration
    ntn             VARCHAR(20),
    -- Pakistan GovOS: CNIC for NADRA cross-referencing
    cnic            VARCHAR(15),
    metadata        JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_entities_jurisdiction ON entity.entities(jurisdiction);
CREATE INDEX idx_entities_status ON entity.entities(status);
CREATE INDEX idx_entities_did ON entity.entities(did);
CREATE INDEX idx_entities_ntn ON entity.entities(ntn) WHERE ntn IS NOT NULL;

-- Beneficial ownership registry
CREATE TABLE IF NOT EXISTS entity.beneficial_owners (
    id              BIGSERIAL PRIMARY KEY,
    entity_id       VARCHAR(255) NOT NULL REFERENCES entity.entities(entity_id),
    owner_did       VARCHAR(255) NOT NULL,
    ownership_pct   DECIMAL(5,2) NOT NULL CHECK (ownership_pct > 0 AND ownership_pct <= 100),
    owner_type      VARCHAR(50) NOT NULL,  -- 'natural_person', 'legal_entity'
    effective_date  DATE NOT NULL,
    end_date        DATE,
    metadata        JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_beneficial_owners_entity ON entity.beneficial_owners(entity_id);

-- Dissolution tracking (10-stage state machine per spec)
CREATE TABLE IF NOT EXISTS entity.dissolution_stages (
    id              BIGSERIAL PRIMARY KEY,
    entity_id       VARCHAR(255) NOT NULL REFERENCES entity.entities(entity_id),
    stage           VARCHAR(50) NOT NULL,
    status          VARCHAR(50) NOT NULL DEFAULT 'pending',
    evidence_hash   VARCHAR(64),
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at    TIMESTAMPTZ
);

CREATE INDEX idx_dissolution_entity ON entity.dissolution_stages(entity_id);

-- -------------------------------------------
-- Licensing
-- -------------------------------------------

CREATE TABLE IF NOT EXISTS licensing.licenses (
    license_id      VARCHAR(255) PRIMARY KEY,
    license_type    VARCHAR(100) NOT NULL,
    holder_entity_id VARCHAR(255) REFERENCES entity.entities(entity_id),
    holder_did      VARCHAR(255),
    status          VARCHAR(50) NOT NULL DEFAULT 'active',
    issued_date     DATE NOT NULL,
    effective_date  DATE,
    expiry_date     DATE,
    regulator_id    VARCHAR(100),
    conditions      JSONB DEFAULT '{}',
    permissions     JSONB DEFAULT '{}',
    restrictions    JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_licenses_holder ON licensing.licenses(holder_entity_id);
CREATE INDEX idx_licenses_status ON licensing.licenses(status);
CREATE INDEX idx_licenses_type ON licensing.licenses(license_type);
CREATE INDEX idx_licenses_expiry ON licensing.licenses(expiry_date) WHERE expiry_date IS NOT NULL;

-- -------------------------------------------
-- Identity (Five Primitives: IDENTITY)
-- -------------------------------------------

CREATE TABLE IF NOT EXISTS identity.identities (
    did             VARCHAR(255) PRIMARY KEY,
    identity_tier   INTEGER NOT NULL DEFAULT 0,
    entity_id       VARCHAR(255),
    did_document    JSONB NOT NULL DEFAULT '{}',
    credentials     JSONB DEFAULT '[]',
    -- External ID linkages (CNIC, NTN, passport)
    linked_ids      JSONB DEFAULT '[]',
    verification_status VARCHAR(50) NOT NULL DEFAULT 'unverified',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_identities_tier ON identity.identities(identity_tier);
CREATE INDEX idx_identities_entity ON identity.identities(entity_id);
CREATE INDEX idx_identities_verification ON identity.identities(verification_status);

-- Identity attestations
CREATE TABLE IF NOT EXISTS identity.attestations (
    attestation_id  VARCHAR(255) PRIMARY KEY,
    subject_did     VARCHAR(255) NOT NULL REFERENCES identity.identities(did),
    issuer_did      VARCHAR(255) NOT NULL,
    attestation_type VARCHAR(100) NOT NULL,
    claims          JSONB NOT NULL,
    proof           JSONB NOT NULL,
    issued_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ
);

CREATE INDEX idx_identity_attestations_subject ON identity.attestations(subject_did);

-- -------------------------------------------
-- Compliance (tensor snapshots)
-- -------------------------------------------

CREATE TABLE IF NOT EXISTS compliance.tensor_snapshots (
    snapshot_id     VARCHAR(255) PRIMARY KEY,
    asset_id        VARCHAR(255) NOT NULL,
    jurisdiction_id VARCHAR(50) NOT NULL,
    -- 9 compliance domains: aml, kyc, sanctions, tax, securities, corporate, custody, data_privacy, licensing
    tensor_state    JSONB NOT NULL,
    commitment      VARCHAR(64),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tensor_asset ON compliance.tensor_snapshots(asset_id);
CREATE INDEX idx_tensor_jurisdiction ON compliance.tensor_snapshots(jurisdiction_id);

-- -------------------------------------------
-- Tax / Fiscal (Five Primitives: FISCAL)
-- -------------------------------------------

CREATE TABLE IF NOT EXISTS tax.events (
    event_id        VARCHAR(255) PRIMARY KEY,
    entity_id       VARCHAR(255) NOT NULL,
    event_type      VARCHAR(100) NOT NULL,  -- 'capital_gain', 'withholding', 'dividend', 'fee'
    amount          DECIMAL(30,8) NOT NULL,
    currency        VARCHAR(10) NOT NULL,
    tax_year        INTEGER NOT NULL,
    ntn             VARCHAR(20),  -- FBR National Tax Number
    jurisdiction    VARCHAR(50) NOT NULL,
    details         JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tax_events_entity ON tax.events(entity_id);
CREATE INDEX idx_tax_events_year ON tax.events(tax_year);
CREATE INDEX idx_tax_events_ntn ON tax.events(ntn) WHERE ntn IS NOT NULL;

-- -------------------------------------------
-- Consent (Five Primitives: CONSENT)
-- -------------------------------------------

CREATE TABLE IF NOT EXISTS consent.requests (
    consent_id      VARCHAR(255) PRIMARY KEY,
    requester_did   VARCHAR(255) NOT NULL,
    consent_type    VARCHAR(100) NOT NULL,
    subject         JSONB NOT NULL,
    required_signers JSONB NOT NULL DEFAULT '[]',
    status          VARCHAR(50) NOT NULL DEFAULT 'pending',
    expires_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_consent_status ON consent.requests(status);
CREATE INDEX idx_consent_requester ON consent.requests(requester_did);

CREATE TABLE IF NOT EXISTS consent.signatures (
    id              BIGSERIAL PRIMARY KEY,
    consent_id      VARCHAR(255) NOT NULL REFERENCES consent.requests(consent_id),
    signer_did      VARCHAR(255) NOT NULL,
    signature       VARCHAR(512) NOT NULL,
    signed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(consent_id, signer_did)
);

CREATE INDEX idx_consent_signatures_consent ON consent.signatures(consent_id);

-- -------------------------------------------
-- Ownership (Five Primitives: OWNERSHIP)
-- -------------------------------------------

CREATE TABLE IF NOT EXISTS entity.cap_tables (
    cap_table_id    VARCHAR(255) PRIMARY KEY,
    entity_id       VARCHAR(255) NOT NULL REFERENCES entity.entities(entity_id),
    version         INTEGER NOT NULL DEFAULT 1,
    share_classes   JSONB NOT NULL DEFAULT '[]',
    total_authorized DECIMAL(30,8),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(entity_id)
);

CREATE TABLE IF NOT EXISTS entity.ownership_transfers (
    transfer_id     VARCHAR(255) PRIMARY KEY,
    cap_table_id    VARCHAR(255) NOT NULL REFERENCES entity.cap_tables(cap_table_id),
    from_did        VARCHAR(255) NOT NULL,
    to_did          VARCHAR(255) NOT NULL,
    share_class     VARCHAR(100) NOT NULL,
    quantity        DECIMAL(30,8) NOT NULL,
    price_per_share DECIMAL(30,8),
    currency        VARCHAR(10),
    -- Triggers tax event on insert
    tax_event_id    VARCHAR(255),
    status          VARCHAR(50) NOT NULL DEFAULT 'pending',
    transfer_date   DATE NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_transfers_cap_table ON entity.ownership_transfers(cap_table_id);
CREATE INDEX idx_transfers_status ON entity.ownership_transfers(status);

-- -------------------------------------------
-- Audit log (immutable hash chain)
-- -------------------------------------------

CREATE TABLE IF NOT EXISTS audit.events (
    event_id        VARCHAR(255) PRIMARY KEY,
    event_type      VARCHAR(100) NOT NULL,
    actor_did       VARCHAR(255),
    resource_type   VARCHAR(100),
    resource_id     VARCHAR(255),
    action          VARCHAR(100) NOT NULL,
    metadata        JSONB DEFAULT '{}',
    previous_hash   VARCHAR(64),
    event_hash      VARCHAR(64) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_actor ON audit.events(actor_did);
CREATE INDEX idx_audit_resource ON audit.events(resource_type, resource_id);
CREATE INDEX idx_audit_time ON audit.events(created_at);

-- ============================================
-- Corridor database: mez_corridor
-- ============================================

\c mez_corridor

CREATE SCHEMA IF NOT EXISTS corridor;
CREATE SCHEMA IF NOT EXISTS receipts;

-- Corridors — state machine: DRAFT → PENDING → ACTIVE → HALTED/SUSPENDED → DEPRECATED
CREATE TABLE IF NOT EXISTS corridor.corridors (
    corridor_id         VARCHAR(255) PRIMARY KEY,
    source_jurisdiction VARCHAR(50) NOT NULL,
    target_jurisdiction VARCHAR(50) NOT NULL,
    status              VARCHAR(50) NOT NULL DEFAULT 'draft',
    corridor_manifest   JSONB,
    trust_anchors       JSONB DEFAULT '[]',
    watcher_count       INTEGER NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_corridor_status CHECK (
        status IN ('draft', 'pending', 'active', 'halted', 'suspended', 'deprecated')
    )
);

CREATE INDEX idx_corridors_status ON corridor.corridors(status);
CREATE INDEX idx_corridors_jurisdictions ON corridor.corridors(source_jurisdiction, target_jurisdiction);

-- Receipts — MMR chain
CREATE TABLE IF NOT EXISTS receipts.receipts (
    receipt_id          VARCHAR(255) PRIMARY KEY,
    corridor_id         VARCHAR(255) NOT NULL,
    receipt_type        VARCHAR(50) NOT NULL,
    sequence_number     BIGINT NOT NULL,
    payload             JSONB NOT NULL,
    previous_receipt_id VARCHAR(255),
    receipt_hash        VARCHAR(64) NOT NULL,
    mmr_position        BIGINT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_receipts_corridor ON receipts.receipts(corridor_id);
CREATE INDEX idx_receipts_sequence ON receipts.receipts(corridor_id, sequence_number);
CREATE INDEX idx_receipts_time ON receipts.receipts(created_at);

-- Watchers
CREATE TABLE IF NOT EXISTS corridor.watchers (
    watcher_id          VARCHAR(255) PRIMARY KEY,
    watcher_did         VARCHAR(255) NOT NULL UNIQUE,
    bond_amount         DECIMAL(20,2) NOT NULL,
    bond_currency       VARCHAR(10) NOT NULL DEFAULT 'USD',
    status              VARCHAR(50) NOT NULL DEFAULT 'pending',
    reputation_score    DECIMAL(5,2) NOT NULL DEFAULT 100.00,
    corridors_authorized JSONB DEFAULT '[]',
    slashing_history    JSONB DEFAULT '[]',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_watchers_status ON corridor.watchers(status);

-- Watcher attestations
CREATE TABLE IF NOT EXISTS corridor.attestations (
    attestation_id      VARCHAR(255) PRIMARY KEY,
    watcher_id          VARCHAR(255) NOT NULL REFERENCES corridor.watchers(watcher_id),
    corridor_id         VARCHAR(255) NOT NULL,
    attestation_type    VARCHAR(100) NOT NULL,
    asset_id            VARCHAR(255),
    jurisdiction_id     VARCHAR(50),
    domain              VARCHAR(50),
    attestation_data    JSONB NOT NULL,
    signature           VARCHAR(512) NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_attestations_watcher ON corridor.attestations(watcher_id);
CREATE INDEX idx_attestations_corridor ON corridor.attestations(corridor_id);
CREATE INDEX idx_attestations_asset ON corridor.attestations(asset_id);

-- ============================================
-- Settlement database: mez_settlement
-- ============================================

\c mez_settlement

CREATE SCHEMA IF NOT EXISTS settlement;

-- Settlement plans
CREATE TABLE IF NOT EXISTS settlement.plans (
    plan_id         VARCHAR(255) PRIMARY KEY,
    plan_type       VARCHAR(50) NOT NULL,
    corridor_id     VARCHAR(255),
    status          VARCHAR(50) NOT NULL DEFAULT 'pending',
    legs            JSONB NOT NULL,
    net_positions   JSONB,
    total_amount    DECIMAL(30,8),
    currency        VARCHAR(10),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    settled_at      TIMESTAMPTZ
);

CREATE INDEX idx_plans_status ON settlement.plans(status);
CREATE INDEX idx_plans_corridor ON settlement.plans(corridor_id);

-- Settlement transactions
CREATE TABLE IF NOT EXISTS settlement.transactions (
    transaction_id  VARCHAR(255) PRIMARY KEY,
    plan_id         VARCHAR(255) NOT NULL REFERENCES settlement.plans(plan_id),
    from_entity     VARCHAR(255) NOT NULL,
    to_entity       VARCHAR(255) NOT NULL,
    amount          DECIMAL(30,8) NOT NULL,
    currency        VARCHAR(10) NOT NULL,
    status          VARCHAR(50) NOT NULL DEFAULT 'pending',
    payment_rail    VARCHAR(50),  -- 'raast', 'swift', 'internal'
    reference       VARCHAR(255),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at    TIMESTAMPTZ
);

CREATE INDEX idx_transactions_plan ON settlement.transactions(plan_id);
CREATE INDEX idx_transactions_status ON settlement.transactions(status);
