ALTER TABLE modules
ADD COLUMN parent_id VARCHAR REFERENCES modules(id);
