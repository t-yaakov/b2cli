-- Create backup_schedules table for scheduling backup jobs
CREATE TABLE backup_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    backup_job_id UUID NOT NULL REFERENCES backup_jobs(id) ON DELETE CASCADE,
    name VARCHAR NOT NULL,
    cron_expression VARCHAR NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    next_run TIMESTAMP,
    last_run TIMESTAMP,
    last_status VARCHAR DEFAULT 'pending', -- 'pending', 'running', 'completed', 'failed'
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    
    -- Ensure only one active schedule per backup job
    UNIQUE(backup_job_id)
);

-- Index for querying enabled schedules
CREATE INDEX idx_backup_schedules_enabled ON backup_schedules(enabled);

-- Index for querying next runs
CREATE INDEX idx_backup_schedules_next_run ON backup_schedules(next_run) WHERE enabled = true;