CREATE TABLE backed_up_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    backup_job_id UUID NOT NULL REFERENCES backup_jobs(id) ON DELETE CASCADE,
    original_path TEXT NOT NULL,
    backed_up_path TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_extension TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    checksum TEXT NOT NULL,
    backed_up_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);