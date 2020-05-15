CREATE TABLE logs (
    id SERIAL PRIMARY KEY,
    account_id VARCHAR NOT NULL,
    level VARCHAR NOT NULL,
    data JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX account_id_logs_index ON logs(account_id);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON logs
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();
