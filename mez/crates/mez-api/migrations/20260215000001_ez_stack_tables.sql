-- EZ Stack Persistence Layer — Initial Migration
--
-- These tables store EZ-Stack-owned state: corridor lifecycle, smart assets,
-- compliance attestations, tensor snapshots, and the immutable audit log.
--
-- Mass primitive data (entities, ownership, fiscal, identity, consent) is NOT
-- stored here. That data lives in the Mass APIs and is accessed via
-- mez-mass-client. See CLAUDE.md Section II.

-- ────────────────────────────────────────────────────────────────
-- Corridors — cross-border corridor lifecycle (EZ Stack domain)
-- State machine: DRAFT → PENDING → ACTIVE → HALTED/SUSPENDED → DEPRECATED
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS corridors (
    id                  UUID PRIMARY KEY,
    jurisdiction_a      VARCHAR(255) NOT NULL,
    jurisdiction_b      VARCHAR(255) NOT NULL,
    status              VARCHAR(50) NOT NULL DEFAULT 'DRAFT',
    transition_log      JSONB NOT NULL DEFAULT '[]',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_corridor_status CHECK (
        status IN ('DRAFT', 'PENDING', 'ACTIVE', 'HALTED', 'SUSPENDED', 'DEPRECATED')
    ),
    CONSTRAINT different_jurisdictions CHECK (
        jurisdiction_a <> jurisdiction_b
    )
);

CREATE INDEX IF NOT EXISTS idx_corridors_status ON corridors(status);
CREATE INDEX IF NOT EXISTS idx_corridors_jurisdictions ON corridors(jurisdiction_a, jurisdiction_b);

-- ────────────────────────────────────────────────────────────────
-- Corridor Receipts — MMR chain entries per corridor
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS corridor_receipts (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    corridor_id         UUID NOT NULL REFERENCES corridors(id) ON DELETE RESTRICT,
    sequence_number     BIGINT NOT NULL,
    payload             JSONB NOT NULL,
    receipt_digest      VARCHAR(64) NOT NULL,
    prev_root           VARCHAR(64) NOT NULL,
    next_root           VARCHAR(64) NOT NULL,
    mmr_root            VARCHAR(64) NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_corridor_sequence UNIQUE (corridor_id, sequence_number)
);

CREATE INDEX IF NOT EXISTS idx_receipts_corridor ON corridor_receipts(corridor_id);
CREATE INDEX IF NOT EXISTS idx_receipts_sequence ON corridor_receipts(corridor_id, sequence_number);

-- ────────────────────────────────────────────────────────────────
-- Smart Assets — smart asset lifecycle (EZ Stack domain)
-- Status: GENESIS → REGISTERED → ACTIVE → PENDING/SUSPENDED → RETIRED
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS smart_assets (
    id                  UUID PRIMARY KEY,
    asset_type          VARCHAR(255) NOT NULL,
    jurisdiction_id     VARCHAR(255) NOT NULL,
    status              VARCHAR(50) NOT NULL DEFAULT 'GENESIS',
    genesis_digest      VARCHAR(64),
    compliance_status   VARCHAR(50) NOT NULL DEFAULT 'unevaluated',
    metadata            JSONB NOT NULL DEFAULT '{}',
    owner_entity_id     UUID,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_asset_status CHECK (
        status IN ('GENESIS', 'REGISTERED', 'ACTIVE', 'PENDING', 'SUSPENDED', 'RETIRED')
    ),
    CONSTRAINT valid_compliance_status CHECK (
        compliance_status IN ('compliant', 'pending', 'non_compliant', 'partially_compliant', 'unevaluated')
    )
);

CREATE INDEX IF NOT EXISTS idx_smart_assets_jurisdiction ON smart_assets(jurisdiction_id);
CREATE INDEX IF NOT EXISTS idx_smart_assets_status ON smart_assets(status);
CREATE INDEX IF NOT EXISTS idx_smart_assets_owner ON smart_assets(owner_entity_id) WHERE owner_entity_id IS NOT NULL;

-- ────────────────────────────────────────────────────────────────
-- Attestations — compliance attestations for regulator queries
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS attestations (
    id                  UUID PRIMARY KEY,
    entity_id           UUID NOT NULL,
    attestation_type    VARCHAR(255) NOT NULL,
    issuer              VARCHAR(255) NOT NULL,
    status              VARCHAR(50) NOT NULL DEFAULT 'PENDING',
    jurisdiction_id     VARCHAR(255) NOT NULL,
    issued_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at          TIMESTAMPTZ,
    details             JSONB NOT NULL DEFAULT '{}',
    CONSTRAINT valid_attestation_status CHECK (
        status IN ('ACTIVE', 'PENDING', 'REVOKED', 'EXPIRED')
    )
);

CREATE INDEX IF NOT EXISTS idx_attestations_entity ON attestations(entity_id);
CREATE INDEX IF NOT EXISTS idx_attestations_jurisdiction ON attestations(jurisdiction_id);
CREATE INDEX IF NOT EXISTS idx_attestations_status ON attestations(status);

-- ────────────────────────────────────────────────────────────────
-- Tensor Snapshots — compliance tensor evaluation results
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS tensor_snapshots (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id            UUID NOT NULL,
    jurisdiction_id     VARCHAR(255) NOT NULL,
    tensor_state        JSONB NOT NULL,
    overall_status      VARCHAR(50) NOT NULL,
    domain_count        INTEGER NOT NULL,
    commitment          VARCHAR(64),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tensor_asset ON tensor_snapshots(asset_id);
CREATE INDEX IF NOT EXISTS idx_tensor_jurisdiction ON tensor_snapshots(jurisdiction_id);

-- ────────────────────────────────────────────────────────────────
-- Audit Events — immutable hash chain for all state mutations
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS audit_events (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type          VARCHAR(100) NOT NULL,
    actor_did           VARCHAR(255),
    resource_type       VARCHAR(100) NOT NULL,
    resource_id         UUID NOT NULL,
    action              VARCHAR(100) NOT NULL,
    metadata            JSONB NOT NULL DEFAULT '{}',
    previous_hash       VARCHAR(64),
    event_hash          VARCHAR(64) NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_events(actor_did) WHERE actor_did IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_audit_resource ON audit_events(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_audit_time ON audit_events(created_at);

-- ────────────────────────────────────────────────────────────────
-- Agentic Policy State — persisted policy configurations
-- ────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS policy_snapshots (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_name         VARCHAR(255) NOT NULL,
    policy_config       JSONB NOT NULL,
    active              BOOLEAN NOT NULL DEFAULT true,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_policy_active ON policy_snapshots(active) WHERE active = true;
