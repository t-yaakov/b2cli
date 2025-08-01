ALTER TABLE backup_jobs
DROP COLUMN source_paths,
DROP COLUMN destination_path;

ALTER TABLE backup_jobs
ADD COLUMN mappings JSONB NOT NULL DEFAULT '{}'::jsonb;