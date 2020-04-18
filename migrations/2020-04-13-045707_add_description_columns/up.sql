ALTER TABLE device_types
ADD COLUMN description VARCHAR(255);

ALTER TABLE modules
ADD COLUMN description VARCHAR(255);

ALTER TABLE components
ADD COLUMN description VARCHAR(255);
