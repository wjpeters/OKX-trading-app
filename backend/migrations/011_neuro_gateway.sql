ALTER TABLE training.releases ADD COLUMN policy_revision bigint NOT NULL DEFAULT 0;
ALTER TABLE training.releases ADD COLUMN code_version text NOT NULL DEFAULT '';
CREATE TRIGGER immutable_release BEFORE UPDATE OF artifact_hash,account_uid,mode,instrument,bar,limits,qualification,policy_revision,code_version,created_ms ON training.releases FOR EACH ROW EXECUTE FUNCTION training.immutable();
CREATE TABLE training.native_runs (
 release_id text PRIMARY KEY REFERENCES training.releases(id),
 status text NOT NULL, state jsonb NOT NULL DEFAULT '{}', report jsonb NOT NULL DEFAULT '{}',
 last_ms bigint NOT NULL DEFAULT 0, updated_ms bigint NOT NULL
);
CREATE TABLE training.native_decisions (
 release_id text NOT NULL REFERENCES training.releases(id), candle_ms bigint NOT NULL,
 kind text NOT NULL, client_id text, detail jsonb NOT NULL, created_ms bigint NOT NULL,
 PRIMARY KEY(release_id,candle_ms,kind)
);
CREATE TABLE training.demo_acceptance (
 id text PRIMARY KEY, release_id text NOT NULL UNIQUE REFERENCES training.releases(id),
 artifact_hash text NOT NULL REFERENCES training.artifacts(hash), code_version text NOT NULL,
 evidence jsonb NOT NULL, confirmed_ms bigint NOT NULL
);
CREATE TRIGGER immutable_demo BEFORE UPDATE OR DELETE ON training.demo_acceptance FOR EACH ROW EXECUTE FUNCTION training.immutable();
ALTER TABLE execution.orders ADD COLUMN bot_release_id text REFERENCES training.releases(id);
ALTER TABLE execution.orders ADD COLUMN reduction_parent_id text;
ALTER TABLE execution.orders ADD COLUMN close_requested boolean NOT NULL DEFAULT false;
ALTER TABLE execution.orders ADD CONSTRAINT owned_reduction FOREIGN KEY(mode,reduction_parent_id) REFERENCES execution.orders(mode,client_id);
ALTER TABLE execution.protection ADD COLUMN cancel_requested boolean NOT NULL DEFAULT false;
CREATE INDEX bot_order_owner ON execution.orders(bot_release_id,created_ms);
CREATE INDEX bot_owned_fills ON execution.fills(mode,parent_id,fill_ms);
