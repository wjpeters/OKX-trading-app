CREATE TABLE execution.market_interest (mode text NOT NULL REFERENCES execution.accounts(mode), instrument text NOT NULL, updated_ms bigint NOT NULL, PRIMARY KEY(mode,instrument));
