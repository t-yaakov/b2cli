ALTER TABLE backup_jobs RENAME COLUMN destination_path TO old_destination_path;

ALTER TABLE backup_jobs ADD COLUMN destination_path JSONB;

UPDATE backup_jobs SET destination_path = jsonb_build_array(old_destination_path);

ALTER TABLE backup_jobs DROP COLUMN old_destination_path;