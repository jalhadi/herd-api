CREATE TABLE topics (
    id VARCHAR PRIMARY KEY,
    account_id VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    description VARCHAR,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE (account_id, name)
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON topics
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();