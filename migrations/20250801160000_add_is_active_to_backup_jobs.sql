-- Add is_active column to backup_jobs table for soft delete
ALTER TABLE backup_jobs 
ADD COLUMN is_active BOOLEAN NOT NULL DEFAULT true;

-- Create index for active jobs queries
CREATE INDEX idx_backup_jobs_is_active ON backup_jobs(is_active);

-- Update existing deleted jobs to be inactive
UPDATE backup_jobs 
SET is_active = false 
WHERE deleted_at IS NOT NULL;