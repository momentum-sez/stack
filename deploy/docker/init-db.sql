-- MSEZ Database Initialization
-- Creates all required databases and schemas

-- Create additional databases
CREATE DATABASE msez_corridor;
CREATE DATABASE msez_watcher;
CREATE DATABASE msez_settlement;

-- Grant permissions
GRANT ALL PRIVILEGES ON DATABASE msez_corridor TO msez;
GRANT ALL PRIVILEGES ON DATABASE msez_watcher TO msez;
GRANT ALL PRIVILEGES ON DATABASE msez_settlement TO msez;

-- Connect to main database and create schemas
\c msez

-- Entity registry schema
CREATE SCHEMA IF NOT EXISTS entity;
CREATE SCHEMA IF NOT EXISTS licensing;
CREATE SCHEMA IF NOT EXISTS identity;
CREATE SCHEMA IF NOT EXISTS compliance;
CREATE SCHEMA IF NOT EXISTS tax;
CREATE SCHEMA IF NOT EXISTS audit;

-- Core tables

-- Entities table
CREATE TABLE IF NOT EXISTS entity.entities (
    entity_id VARCHAR(255) PRIMARY KEY,
    entity_type VARCHAR(50) NOT NULL,
    legal_name VARCHAR(500) NOT NULL,
    registration_number VARCHAR(100),
    jurisdiction VARCHAR(50) NOT NULL,
    status VARCHAR(50) DEFAULT 'active',
    formed_date DATE,
    did VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_entities_jurisdiction ON entity.entities(jurisdiction);
CREATE INDEX idx_entities_status ON entity.entities(status);
CREATE INDEX idx_entities_did ON entity.entities(did);

-- Licenses table
CREATE TABLE IF NOT EXISTS licensing.licenses (
    license_id VARCHAR(255) PRIMARY KEY,
    license_type VARCHAR(100) NOT NULL,
    holder_entity_id VARCHAR(255) REFERENCES entity.entities(entity_id),
    holder_did VARCHAR(255),
    status VARCHAR(50) DEFAULT 'active',
    issued_date DATE NOT NULL,
    effective_date DATE,
    expiry_date DATE,
    regulator_id VARCHAR(100),
    conditions JSONB,
    permissions JSONB,
    restrictions JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_licenses_holder ON licensing.licenses(holder_entity_id);
CREATE INDEX idx_licenses_status ON licensing.licenses(status);
CREATE INDEX idx_licenses_type ON licensing.licenses(license_type);

-- Identities table
CREATE TABLE IF NOT EXISTS identity.identities (
    did VARCHAR(255) PRIMARY KEY,
    identity_tier INTEGER DEFAULT 0,
    entity_id VARCHAR(255),
    did_document JSONB,
    credentials JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_identities_tier ON identity.identities(identity_tier);
CREATE INDEX idx_identities_entity ON identity.identities(entity_id);

-- Compliance tensor snapshots
CREATE TABLE IF NOT EXISTS compliance.tensor_snapshots (
    snapshot_id VARCHAR(255) PRIMARY KEY,
    asset_id VARCHAR(255) NOT NULL,
    jurisdiction_id VARCHAR(50) NOT NULL,
    tensor_state JSONB NOT NULL,
    commitment VARCHAR(64),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_tensor_asset ON compliance.tensor_snapshots(asset_id);
CREATE INDEX idx_tensor_jurisdiction ON compliance.tensor_snapshots(jurisdiction_id);

-- Audit log
CREATE TABLE IF NOT EXISTS audit.events (
    event_id VARCHAR(255) PRIMARY KEY,
    event_type VARCHAR(100) NOT NULL,
    actor_did VARCHAR(255),
    resource_type VARCHAR(100),
    resource_id VARCHAR(255),
    action VARCHAR(100),
    metadata JSONB,
    previous_hash VARCHAR(64),
    event_hash VARCHAR(64),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_audit_actor ON audit.events(actor_did);
CREATE INDEX idx_audit_resource ON audit.events(resource_type, resource_id);
CREATE INDEX idx_audit_time ON audit.events(created_at);

-- Connect to corridor database
\c msez_corridor

CREATE SCHEMA IF NOT EXISTS corridor;
CREATE SCHEMA IF NOT EXISTS receipts;

-- Corridors table
CREATE TABLE IF NOT EXISTS corridor.corridors (
    corridor_id VARCHAR(255) PRIMARY KEY,
    source_jurisdiction VARCHAR(50) NOT NULL,
    target_jurisdiction VARCHAR(50) NOT NULL,
    status VARCHAR(50) DEFAULT 'active',
    corridor_manifest JSONB,
    trust_anchors JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Receipts table
CREATE TABLE IF NOT EXISTS receipts.receipts (
    receipt_id VARCHAR(255) PRIMARY KEY,
    corridor_id VARCHAR(255) NOT NULL,
    receipt_type VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    previous_receipt_id VARCHAR(255),
    receipt_hash VARCHAR(64),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_receipts_corridor ON receipts.receipts(corridor_id);
CREATE INDEX idx_receipts_time ON receipts.receipts(created_at);

-- Connect to watcher database
\c msez_watcher

CREATE SCHEMA IF NOT EXISTS watcher;

-- Watchers table
CREATE TABLE IF NOT EXISTS watcher.watchers (
    watcher_id VARCHAR(255) PRIMARY KEY,
    watcher_did VARCHAR(255) NOT NULL UNIQUE,
    bond_amount DECIMAL(20,2),
    bond_currency VARCHAR(10) DEFAULT 'USD',
    status VARCHAR(50) DEFAULT 'pending',
    reputation_score DECIMAL(5,2) DEFAULT 100.00,
    corridors_authorized JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Attestations table
CREATE TABLE IF NOT EXISTS watcher.attestations (
    attestation_id VARCHAR(255) PRIMARY KEY,
    watcher_id VARCHAR(255) REFERENCES watcher.watchers(watcher_id),
    attestation_type VARCHAR(100) NOT NULL,
    asset_id VARCHAR(255),
    jurisdiction_id VARCHAR(50),
    domain VARCHAR(50),
    attestation_data JSONB,
    signature VARCHAR(512),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_attestations_watcher ON watcher.attestations(watcher_id);
CREATE INDEX idx_attestations_asset ON watcher.attestations(asset_id);

-- Connect to settlement database
\c msez_settlement

CREATE SCHEMA IF NOT EXISTS settlement;

-- Settlement plans table
CREATE TABLE IF NOT EXISTS settlement.plans (
    plan_id VARCHAR(255) PRIMARY KEY,
    plan_type VARCHAR(50) NOT NULL,
    status VARCHAR(50) DEFAULT 'pending',
    legs JSONB NOT NULL,
    net_positions JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    settled_at TIMESTAMP WITH TIME ZONE
);

-- Transactions table
CREATE TABLE IF NOT EXISTS settlement.transactions (
    transaction_id VARCHAR(255) PRIMARY KEY,
    plan_id VARCHAR(255) REFERENCES settlement.plans(plan_id),
    from_entity VARCHAR(255) NOT NULL,
    to_entity VARCHAR(255) NOT NULL,
    amount DECIMAL(30,8) NOT NULL,
    currency VARCHAR(10) NOT NULL,
    status VARCHAR(50) DEFAULT 'pending',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX idx_transactions_plan ON settlement.transactions(plan_id);
CREATE INDEX idx_transactions_status ON settlement.transactions(status);
