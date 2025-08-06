-- Tabela para armazenar configurações de scan (igual backup_jobs)
CREATE TABLE IF NOT EXISTS scan_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Informações básicas
    name VARCHAR(255) NOT NULL,
    description TEXT,
    
    -- Configuração do scan
    root_path TEXT NOT NULL,
    recursive BOOLEAN DEFAULT TRUE,
    max_depth INTEGER,
    exclude_patterns TEXT[] DEFAULT '{}',
    
    -- Status (igual backup_jobs)
    status VARCHAR(50) DEFAULT 'PENDING', -- PENDING, RUNNING, COMPLETED, FAILED
    
    -- Soft delete (igual backup_jobs)
    is_active BOOLEAN DEFAULT TRUE,
    deleted_at TIMESTAMP,
    
    -- Estatísticas do último run
    last_run_at TIMESTAMP,
    last_scan_job_id UUID REFERENCES scan_jobs(id),
    total_runs INTEGER DEFAULT 0,
    successful_runs INTEGER DEFAULT 0,
    failed_runs INTEGER DEFAULT 0,
    
    -- Timestamps
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Índices
CREATE INDEX idx_scan_configs_status ON scan_configs(status);
CREATE INDEX idx_scan_configs_active ON scan_configs(is_active);
CREATE INDEX idx_scan_configs_name ON scan_configs(name);

-- Trigger para updated_at
CREATE TRIGGER update_scan_configs_updated_at BEFORE UPDATE ON scan_configs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Atualizar scan_jobs para referenciar scan_config
ALTER TABLE scan_jobs ADD COLUMN IF NOT EXISTS scan_config_id UUID REFERENCES scan_configs(id);

COMMENT ON TABLE scan_configs IS 'Configurações de scan reutilizáveis (similar a backup_jobs)';
COMMENT ON COLUMN scan_configs.status IS 'PENDING = criado, RUNNING = executando, COMPLETED = concluído, FAILED = falhou';