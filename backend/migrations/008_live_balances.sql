CREATE TABLE execution.wallets (
  mode text PRIMARY KEY REFERENCES execution.accounts(mode),
  snapshot jsonb, status text NOT NULL DEFAULT 'OFFLINE',
  reason text NOT NULL DEFAULT '', updated_ms bigint NOT NULL DEFAULT 0
);
INSERT INTO execution.wallets(mode) VALUES ('OKX_DEMO'),('LIVE');
ALTER TABLE execution.accounts ADD COLUMN live_release jsonb;
