CREATE SCHEMA IF NOT EXISTS training;
CREATE TABLE training.datasets (
 id text PRIMARY KEY, instrument text NOT NULL, start_ms bigint NOT NULL, end_ms bigint NOT NULL,
 status text NOT NULL, manifest jsonb NOT NULL DEFAULT '{}', updated_ms bigint NOT NULL
);
CREATE TABLE training.campaigns (
 id text PRIMARY KEY, config jsonb NOT NULL, config_hash text NOT NULL, desired text NOT NULL,
 status text NOT NULL, progress jsonb NOT NULL DEFAULT '{}', created_ms bigint NOT NULL, updated_ms bigint NOT NULL
);
CREATE TABLE training.artifacts (
 hash text PRIMARY KEY, campaign_id text NOT NULL REFERENCES training.campaigns(id),
 instrument text NOT NULL, bar text NOT NULL, artifact jsonb NOT NULL, summary jsonb NOT NULL,
 created_ms bigint NOT NULL
);
CREATE TABLE training.holdouts (
 instrument text NOT NULL, start_ms bigint NOT NULL, end_ms bigint NOT NULL,
 artifact_hash text NOT NULL REFERENCES training.artifacts(hash), campaign_id text NOT NULL,
 status text NOT NULL, report jsonb, created_ms bigint NOT NULL,
 PRIMARY KEY(instrument,start_ms,end_ms,artifact_hash)
);
CREATE TABLE training.forward_runs (
 id text PRIMARY KEY, artifact_hash text NOT NULL REFERENCES training.artifacts(hash),
 status text NOT NULL, state jsonb NOT NULL, report jsonb NOT NULL DEFAULT '{}',
 started_ms bigint NOT NULL, updated_ms bigint NOT NULL
);
CREATE TABLE training.releases (
 id text PRIMARY KEY, artifact_hash text NOT NULL REFERENCES training.artifacts(hash),
 account_uid text NOT NULL, mode text NOT NULL, instrument text NOT NULL, bar text NOT NULL,
 limits jsonb NOT NULL, qualification jsonb NOT NULL, status text NOT NULL,
 created_ms bigint NOT NULL
);
CREATE TABLE training.audit (
 id bigserial PRIMARY KEY, kind text NOT NULL, subject text NOT NULL, detail jsonb NOT NULL, at_ms bigint NOT NULL
);
CREATE TABLE training.live_candles (instrument text NOT NULL,ts bigint NOT NULL,payload jsonb NOT NULL,received_ms bigint NOT NULL,PRIMARY KEY(instrument,ts));
CREATE OR REPLACE FUNCTION training.immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'Training artifact/configuration is immutable'; END $$;
CREATE TRIGGER immutable_artifact BEFORE UPDATE OR DELETE ON training.artifacts FOR EACH ROW EXECUTE FUNCTION training.immutable();
CREATE TRIGGER immutable_campaign_config BEFORE UPDATE OF config,config_hash ON training.campaigns FOR EACH ROW EXECUTE FUNCTION training.immutable();
