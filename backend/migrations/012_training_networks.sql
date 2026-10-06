CREATE TABLE training.network_imports (
 id text PRIMARY KEY,
 artifact_sha256 text NOT NULL UNIQUE,
 name text NOT NULL,
 bundle jsonb NOT NULL,
 created_ms bigint NOT NULL
);
CREATE TRIGGER immutable_network_import BEFORE UPDATE OR DELETE ON training.network_imports
 FOR EACH ROW EXECUTE FUNCTION training.immutable();
