CREATE TABLE IF NOT EXISTS market_cache (
 instrument TEXT NOT NULL, timeframe TEXT NOT NULL,
 received_at_ms BIGINT NOT NULL, payload JSONB NOT NULL,
 PRIMARY KEY (instrument, timeframe)
);
CREATE TABLE IF NOT EXISTS audit_events (
 id BIGSERIAL PRIMARY KEY, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 kind TEXT NOT NULL, detail JSONB NOT NULL
);
-- Trading tables are intentionally absent until atomic reservation/OMS phase 2.
