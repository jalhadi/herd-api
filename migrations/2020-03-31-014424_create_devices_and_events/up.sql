-- TRIGGER SET TIMESTAMP NOT NULL FUNCTION
CREATE OR REPLACE FUNCTION trigger_set_timestamp()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- DEVICE TYPES
CREATE TABLE device_types (
    id VARCHAR PRIMARY KEY,
    account_id VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE (account_id, name)
);

CREATE INDEX account_id_index ON device_types (account_id);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON device_types
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

-- DEVICES
CREATE TABLE devices (
    id VARCHAR PRIMARY KEY,
    device_type_id VARCHAR NOT NULL REFERENCES device_types(id),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON devices
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

-- MODULE TYPES
CREATE TABLE modules (
    id VARCHAR PRIMARY KEY,
    device_type_id VARCHAR NOT NULL REFERENCES device_types(id),
    name VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE (device_type_id, name)
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON modules
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

-- COMPONENTS
CREATE TABLE components (
    id VARCHAR PRIMARY KEY,
    module_type_id VARCHAR NOT NULL REFERENCES modules(id),
    name VARCHAR NOT NULL,
    -- Ideally, some json schema is adhered to
    -- but removing for now due to complexity
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE (module_type_id, name)
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON components
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();


-- COMPONENT EVENTS
CREATE TABLE component_events (
    id SERIAL PRIMARY KEY,
    component_id VARCHAR NOT NULL REFERENCES components(id),
    device_id VARCHAR NOT NULL REFERENCES devices(id),
    data JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON component_events
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();
