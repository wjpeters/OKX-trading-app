CREATE SCHEMA paper_v2;
CREATE TABLE paper_v2.account (
 id integer PRIMARY KEY CHECK (id=1), revision bigint NOT NULL,
 payload jsonb NOT NULL CHECK (payload->>'mode' = 'PAPER_LOCAL'),
 updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE paper_v2.events (
 seq bigint PRIMARY KEY, ts bigint NOT NULL, kind text NOT NULL,
 subject text NOT NULL, detail text NOT NULL
);
CREATE TABLE paper_v2.previews (
 id bigserial PRIMARY KEY, revision bigint NOT NULL, expires_ms bigint NOT NULL,
 payload jsonb NOT NULL, used boolean NOT NULL DEFAULT false
);
CREATE TABLE paper_v2.replays (
 id bigserial PRIMARY KEY, created_at timestamptz NOT NULL DEFAULT now(), report jsonb NOT NULL
);
