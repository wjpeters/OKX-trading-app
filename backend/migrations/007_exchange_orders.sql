CREATE SCHEMA IF NOT EXISTS execution;
CREATE TABLE execution.accounts (
  mode text PRIMARY KEY CHECK(mode IN ('OKX_DEMO','LIVE')),
  snapshot jsonb, policy jsonb, policy_revision bigint NOT NULL DEFAULT 0,
  status text NOT NULL DEFAULT 'OFFLINE', reason text NOT NULL DEFAULT '',
  updated_ms bigint NOT NULL DEFAULT 0, risk_memory jsonb NOT NULL DEFAULT '{}'
);
INSERT INTO execution.accounts(mode) VALUES ('OKX_DEMO'),('LIVE');
CREATE TABLE execution.previews (
  id text PRIMARY KEY, mode text NOT NULL REFERENCES execution.accounts(mode),
  policy_revision bigint NOT NULL, ticket jsonb NOT NULL, compiled jsonb NOT NULL,
  expires_ms bigint NOT NULL, client_id text NOT NULL UNIQUE, used boolean NOT NULL DEFAULT false
);
CREATE TABLE execution.orders (
  mode text NOT NULL REFERENCES execution.accounts(mode), client_id text NOT NULL,
  account_uid text NOT NULL, ticket jsonb NOT NULL, compiled jsonb NOT NULL,
  state text NOT NULL, command text NOT NULL DEFAULT 'SUBMIT',
  exchange_id text, result jsonb, reason text NOT NULL DEFAULT '',
  reserve_currency text NOT NULL, reserve numeric NOT NULL CHECK(reserve>=0),
  quantity numeric NOT NULL CHECK(quantity>0), filled numeric NOT NULL DEFAULT 0 CHECK(filled>=0),
  created_ms bigint NOT NULL, updated_ms bigint NOT NULL,
  PRIMARY KEY(mode,client_id)
);
CREATE INDEX execution_open_orders ON execution.orders(mode,state,created_ms);
CREATE TABLE execution.events (
  id bigserial PRIMARY KEY, mode text NOT NULL, client_id text,
  kind text NOT NULL, detail jsonb NOT NULL, created_ms bigint NOT NULL
);
CREATE TABLE execution.protection (
  mode text NOT NULL, parent_id text NOT NULL, client_id text NOT NULL,
  quantity numeric NOT NULL CHECK(quantity>0), state text NOT NULL,
  exchange_id text, result jsonb, created_ms bigint NOT NULL,
  PRIMARY KEY(mode,client_id), FOREIGN KEY(mode,parent_id) REFERENCES execution.orders(mode,client_id)
);
CREATE TABLE execution.children (
  mode text NOT NULL, parent_id text NOT NULL, exchange_id text NOT NULL,
  PRIMARY KEY(mode,exchange_id)
);
CREATE TABLE execution.fills (
  mode text NOT NULL, bill_id text NOT NULL, parent_id text NOT NULL,
  exchange_id text NOT NULL, instrument text NOT NULL, side text NOT NULL,
  quantity numeric NOT NULL CHECK(quantity>0), price numeric NOT NULL CHECK(price>0),
  fee numeric NOT NULL, fee_currency text NOT NULL, fill_ms bigint NOT NULL,
  detail jsonb NOT NULL, PRIMARY KEY(mode,bill_id)
);
CREATE TABLE execution.bills (
  mode text NOT NULL, bill_id text NOT NULL, currency text NOT NULL,
  balance_change numeric NOT NULL, kind text NOT NULL, bill_ms bigint NOT NULL,
  detail jsonb NOT NULL, risk_applied boolean NOT NULL DEFAULT false, PRIMARY KEY(mode,bill_id)
);
