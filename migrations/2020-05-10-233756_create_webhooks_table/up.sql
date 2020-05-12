CREATE TABLE webhooks (
    id SERIAL PRIMARY KEY,
    account_id VARCHAR NOT NULL,
    url VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON webhooks
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TABLE webhook_topics (
    id SERIAL PRIMARY KEY,
    webhook_id INTEGER NOT NULL REFERENCES webhooks(id),
    topic_id VARCHAR NOT NULL REFERENCES topics(id),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON webhook_topics
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();
