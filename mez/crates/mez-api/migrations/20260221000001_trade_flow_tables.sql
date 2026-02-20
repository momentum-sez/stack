-- Trade flow tables for the Trade Corridor Instruments runtime.
-- Follows the established pattern from sovereign_mass_tables.sql.

CREATE TABLE IF NOT EXISTS trade_flows (
    flow_id UUID PRIMARY KEY,
    corridor_id UUID,
    flow_type TEXT NOT NULL,
    state TEXT NOT NULL,
    seller JSONB NOT NULL,
    buyer JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS trade_transitions (
    transition_id UUID PRIMARY KEY,
    flow_id UUID NOT NULL REFERENCES trade_flows(flow_id),
    kind TEXT NOT NULL,
    from_state TEXT NOT NULL,
    to_state TEXT NOT NULL,
    payload JSONB NOT NULL,
    document_digests JSONB NOT NULL DEFAULT '[]',
    receipt_digest TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_trade_transitions_flow_id ON trade_transitions(flow_id);
