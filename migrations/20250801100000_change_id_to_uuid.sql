ALTER TABLE backup_jobs DROP COLUMN id;

ALTER TABLE backup_jobs ADD COLUMN id UUID PRIMARY KEY DEFAULT gen_random_uuid();