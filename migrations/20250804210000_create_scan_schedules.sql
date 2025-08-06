-- Tabela para agendamento de scans periódicos
CREATE TABLE IF NOT EXISTS scan_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Informações básicas
    name VARCHAR(255) NOT NULL,
    description TEXT,
    
    -- Configuração do scan
    root_path TEXT NOT NULL,
    recursive BOOLEAN DEFAULT TRUE,
    max_depth INTEGER,
    exclude_patterns TEXT[],
    
    -- Agendamento (cron format)
    cron_expression VARCHAR(100) NOT NULL, -- Ex: "0 2 * * *" para 2AM diariamente
    enabled BOOLEAN DEFAULT TRUE,
    
    -- Status
    last_run_at TIMESTAMP,
    last_run_status VARCHAR(50), -- 'success', 'failed', 'running'
    last_scan_job_id UUID REFERENCES scan_jobs(id),
    next_run_at TIMESTAMP,
    
    -- Estatísticas
    total_runs INTEGER DEFAULT 0,
    successful_runs INTEGER DEFAULT 0,
    failed_runs INTEGER DEFAULT 0,
    
    -- Timestamps
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Índices
CREATE INDEX idx_scan_schedules_enabled ON scan_schedules(enabled);
CREATE INDEX idx_scan_schedules_next_run ON scan_schedules(next_run_at);

-- Trigger para updated_at
CREATE TRIGGER update_scan_schedules_updated_at BEFORE UPDATE ON scan_schedules
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Exemplos de schedules úteis
COMMENT ON TABLE scan_schedules IS 'Agendamentos para varredura periódica de arquivos';
COMMENT ON COLUMN scan_schedules.cron_expression IS 'Formato cron: MIN HOUR DAY MONTH WEEKDAY. Ex: "0 2 * * *" = 2AM diariamente, "0 3 * * 0" = 3AM domingos';