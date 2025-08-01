-- Step 1: Add the new column as nullable
ALTER TABLE backup_jobs ADD COLUMN source_paths JSONB;

-- Step 2: Update existing rows to set a default value (empty array)
UPDATE backup_jobs SET source_paths = '[]'::jsonb;

-- Step 3: Make the new column NOT NULL
ALTER TABLE backup_jobs ALTER COLUMN source_paths SET NOT NULL;

-- Step 4: Drop the old column
ALTER TABLE backup_jobs DROP COLUMN source_path;