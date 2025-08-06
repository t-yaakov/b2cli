-- Tabela para histórico de mudanças dos arquivos (1 para N)
CREATE TABLE IF NOT EXISTS file_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_catalog_id UUID NOT NULL REFERENCES file_catalog(id) ON DELETE CASCADE,
    scan_job_id UUID REFERENCES scan_jobs(id),
    
    -- Snapshot dos dados no momento do scan
    file_size BIGINT NOT NULL,
    content_hash VARCHAR(64),
    modified_at TIMESTAMP,
    accessed_at TIMESTAMP,
    
    -- O que mudou desde o último scan
    size_changed BOOLEAN DEFAULT FALSE,
    hash_changed BOOLEAN DEFAULT FALSE,
    modified_changed BOOLEAN DEFAULT FALSE,
    accessed_changed BOOLEAN DEFAULT FALSE,
    
    -- Delta de mudanças
    size_delta BIGINT,  -- Diferença em bytes (positivo = cresceu)
    days_since_last_access INTEGER,
    days_since_last_modification INTEGER,
    
    -- Contexto
    scan_type VARCHAR(50), -- 'scheduled', 'manual', 'backup_pre', 'backup_post'
    backup_job_id UUID REFERENCES backup_jobs(id),
    
    -- Timestamp
    scanned_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Índices para consultas eficientes
CREATE INDEX idx_file_history_file_id ON file_history(file_catalog_id);
CREATE INDEX idx_file_history_scan_job ON file_history(scan_job_id);
CREATE INDEX idx_file_history_backup_job ON file_history(backup_job_id);
CREATE INDEX idx_file_history_scanned_at ON file_history(scanned_at DESC);
CREATE INDEX idx_file_history_changes ON file_history(hash_changed, size_changed);

-- View para análise de padrões de acesso
CREATE VIEW file_access_patterns AS
SELECT 
    fc.file_path,
    fc.file_name,
    fc.file_size as current_size,
    fc.accessed_at as last_access,
    fc.modified_at as last_modified,
    COUNT(fh.id) as scan_count,
    MAX(fh.scanned_at) as last_scan,
    SUM(CASE WHEN fh.accessed_changed THEN 1 ELSE 0 END) as access_count,
    SUM(CASE WHEN fh.hash_changed THEN 1 ELSE 0 END) as modification_count,
    AVG(fh.days_since_last_access) as avg_days_between_access,
    CASE 
        WHEN fc.accessed_at < CURRENT_TIMESTAMP - INTERVAL '6 months' THEN 'COLD'
        WHEN fc.accessed_at < CURRENT_TIMESTAMP - INTERVAL '30 days' THEN 'WARM'
        ELSE 'HOT'
    END as temperature_tier
FROM file_catalog fc
LEFT JOIN file_history fh ON fc.id = fh.file_catalog_id
WHERE fc.is_active = TRUE
GROUP BY fc.id, fc.file_path, fc.file_name, fc.file_size, fc.accessed_at, fc.modified_at;

-- View para detectar arquivos que crescem muito
CREATE VIEW growing_files AS
SELECT 
    fc.file_path,
    fc.file_name,
    fc.file_size as current_size,
    MIN(fh.file_size) as initial_size,
    MAX(fh.file_size) as max_size,
    (fc.file_size - MIN(fh.file_size)) as total_growth,
    COUNT(DISTINCT fh.content_hash) as version_count,
    array_agg(DISTINCT fh.content_hash) as all_hashes
FROM file_catalog fc
JOIN file_history fh ON fc.id = fh.file_catalog_id
WHERE fh.size_changed = TRUE
GROUP BY fc.id, fc.file_path, fc.file_name, fc.file_size
HAVING COUNT(*) > 1
ORDER BY total_growth DESC;

-- Função para registrar mudanças no histórico
CREATE OR REPLACE FUNCTION record_file_change(
    p_file_id UUID,
    p_scan_job_id UUID,
    p_new_size BIGINT,
    p_new_hash VARCHAR(64),
    p_new_modified TIMESTAMP,
    p_new_accessed TIMESTAMP,
    p_scan_type VARCHAR(50) DEFAULT 'manual',
    p_backup_job_id UUID DEFAULT NULL
) RETURNS UUID AS $$
DECLARE
    v_history_id UUID;
    v_old_size BIGINT;
    v_old_hash VARCHAR(64);
    v_old_modified TIMESTAMP;
    v_old_accessed TIMESTAMP;
BEGIN
    -- Buscar dados anteriores
    SELECT file_size, content_hash, modified_at, accessed_at
    INTO v_old_size, v_old_hash, v_old_modified, v_old_accessed
    FROM file_catalog
    WHERE id = p_file_id;
    
    -- Inserir no histórico
    INSERT INTO file_history (
        file_catalog_id,
        scan_job_id,
        file_size,
        content_hash,
        modified_at,
        accessed_at,
        size_changed,
        hash_changed,
        modified_changed,
        accessed_changed,
        size_delta,
        days_since_last_access,
        days_since_last_modification,
        scan_type,
        backup_job_id
    ) VALUES (
        p_file_id,
        p_scan_job_id,
        p_new_size,
        p_new_hash,
        p_new_modified,
        p_new_accessed,
        p_new_size != v_old_size,
        p_new_hash != v_old_hash,
        p_new_modified != v_old_modified,
        p_new_accessed != v_old_accessed,
        p_new_size - v_old_size,
        EXTRACT(DAY FROM (CURRENT_TIMESTAMP - p_new_accessed))::INTEGER,
        EXTRACT(DAY FROM (CURRENT_TIMESTAMP - p_new_modified))::INTEGER,
        p_scan_type,
        p_backup_job_id
    ) RETURNING id INTO v_history_id;
    
    -- Atualizar file_catalog com os novos valores
    UPDATE file_catalog SET
        file_size = p_new_size,
        content_hash = p_new_hash,
        modified_at = p_new_modified,
        accessed_at = p_new_accessed,
        last_scan_at = CURRENT_TIMESTAMP
    WHERE id = p_file_id;
    
    RETURN v_history_id;
END;
$$ LANGUAGE plpgsql;

-- Adicionar coluna para vincular scan com backup
ALTER TABLE scan_jobs ADD COLUMN IF NOT EXISTS backup_job_id UUID REFERENCES backup_jobs(id);
ALTER TABLE scan_jobs ADD COLUMN IF NOT EXISTS scan_type VARCHAR(50) DEFAULT 'manual';

-- Adicionar índice único para evitar duplicatas no file_catalog
CREATE UNIQUE INDEX IF NOT EXISTS idx_file_catalog_path_unique ON file_catalog(file_path);