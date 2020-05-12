ALTER TABLE webhook_topics
ADD CONSTRAINT unique_webhook_id_topic_id
UNIQUE (webhook_id, topic_id);
