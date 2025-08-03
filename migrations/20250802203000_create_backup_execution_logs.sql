-- Migration: Create backup_execution_logs table
-- This table stores detailed logs from rclone executions

CREATE TABLE backup_execution_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    backup_job_id UUID NOT NULL REFERENCES backup_jobs(id) ON DELETE CASCADE,
    schedule_id UUID REFERENCES backup_schedules(id) ON DELETE SET NULL,
    
    -- Execution details
    started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE,
    status VARCHAR(20) NOT NULL DEFAULT 'running', -- running, completed, failed, cancelled
    
    -- Rclone command and config
    rclone_command TEXT NOT NULL,
    source_path TEXT NOT NULL,
    destination_path TEXT NOT NULL,
    rclone_config JSONB, -- rclone flags and options used
    
    -- Transfer statistics
    files_transferred INTEGER DEFAULT 0,
    files_checked INTEGER DEFAULT 0,
    files_deleted INTEGER DEFAULT 0,
    bytes_transferred BIGINT DEFAULT 0,
    transfer_rate_mbps DECIMAL(10,2), -- MB/s
    duration_seconds INTEGER,
    
    -- Error tracking
    error_count INTEGER DEFAULT 0,
    retry_count INTEGER DEFAULT 0,
    error_message TEXT,
    
    -- Raw logs for debugging
    rclone_stdout TEXT,
    rclone_stderr TEXT,
    rclone_log_file_path TEXT,
    
    -- Metadata
    triggered_by VARCHAR(20) DEFAULT 'schedule', -- schedule, manual, api
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_backup_execution_logs_job_id ON backup_execution_logs(backup_job_id);
CREATE INDEX idx_backup_execution_logs_schedule_id ON backup_execution_logs(schedule_id);
CREATE INDEX idx_backup_execution_logs_status ON backup_execution_logs(status);
CREATE INDEX idx_backup_execution_logs_started_at ON backup_execution_logs(started_at DESC);
CREATE INDEX idx_backup_execution_logs_completed_at ON backup_execution_logs(completed_at DESC) WHERE completed_at IS NOT NULL;

-- Index for Prometheus queries (last 24h, success rate, etc)
CREATE INDEX idx_backup_execution_logs_metrics ON backup_execution_logs(started_at, status, backup_job_id);

-- Trigger to update updated_at
CREATE OR REPLACE FUNCTION update_backup_execution_logs_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER trigger_backup_execution_logs_updated_at
    BEFORE UPDATE ON backup_execution_logs
    FOR EACH ROW
    EXECUTE FUNCTION update_backup_execution_logs_updated_at();