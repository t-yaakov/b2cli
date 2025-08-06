-- Relatórios avançados para sistema de backup B2CLI
-- Autor: DBA Team
-- Data: 2025-08-04

-- 1. Relatório de Performance de Backups por Mês
CREATE OR REPLACE VIEW backup_performance_monthly AS
SELECT 
    DATE_TRUNC('month', started_at) as month,
    COUNT(*) as total_backups,
    COUNT(*) FILTER(WHERE status = 'completed') as successful_backups,
    COUNT(*) FILTER(WHERE status = 'failed') as failed_backups,
    ROUND(
        COUNT(*) FILTER(WHERE status = 'completed')::NUMERIC / COUNT(*) * 100, 2
    ) as success_rate_percent,
    AVG(duration_seconds) FILTER(WHERE status = 'completed') as avg_duration_seconds,
    SUM(bytes_transferred) FILTER(WHERE status = 'completed') as total_bytes_transferred,
    AVG(transfer_rate_mbps) FILTER(WHERE status = 'completed') as avg_transfer_rate_mbps
FROM backup_execution_logs
WHERE started_at >= CURRENT_DATE - INTERVAL '12 months'
GROUP BY DATE_TRUNC('month', started_at)
ORDER BY month DESC;

-- 2. Top 10 Jobs com Mais Falhas
CREATE OR REPLACE VIEW top_failing_jobs AS
SELECT 
    bj.id,
    bj.name,
    COUNT(*) FILTER(WHERE bel.status = 'failed') as failure_count,
    COUNT(*) as total_executions,
    ROUND(
        COUNT(*) FILTER(WHERE bel.status = 'failed')::NUMERIC / COUNT(*) * 100, 2
    ) as failure_rate_percent,
    MAX(bel.started_at) as last_execution,
    STRING_AGG(DISTINCT bel.error_message, '; ') as common_errors
FROM backup_jobs bj
JOIN backup_execution_logs bel ON bj.id = bel.backup_job_id
WHERE bj.is_active = true
  AND bel.started_at >= CURRENT_DATE - INTERVAL '30 days'
GROUP BY bj.id, bj.name
HAVING COUNT(*) FILTER(WHERE bel.status = 'failed') > 0
ORDER BY failure_count DESC, failure_rate_percent DESC
LIMIT 10;

-- 3. Análise de Crescimento de Dados
CREATE OR REPLACE VIEW data_growth_analysis AS
WITH monthly_data AS (
    SELECT 
        DATE_TRUNC('month', started_at) as month,
        SUM(bytes_transferred) as monthly_bytes
    FROM backup_execution_logs
    WHERE status = 'completed'
      AND started_at >= CURRENT_DATE - INTERVAL '12 months'
    GROUP BY DATE_TRUNC('month', started_at)
)
SELECT 
    month,
    monthly_bytes,
    ROUND(monthly_bytes / 1024.0 / 1024.0 / 1024.0, 2) as monthly_gb,
    LAG(monthly_bytes) OVER (ORDER BY month) as previous_month_bytes,
    CASE 
        WHEN LAG(monthly_bytes) OVER (ORDER BY month) IS NOT NULL THEN
            ROUND(
                (monthly_bytes - LAG(monthly_bytes) OVER (ORDER BY month))::NUMERIC 
                / LAG(monthly_bytes) OVER (ORDER BY month) * 100, 2
            )
        ELSE NULL
    END as growth_percent
FROM monthly_data
ORDER BY month DESC;

-- 4. Arquivos Críticos Sem Backup Recente
CREATE OR REPLACE VIEW critical_files_no_recent_backup AS
SELECT 
    fc.file_path,
    fc.file_name,
    fc.file_size,
    ROUND(fc.file_size / 1024.0 / 1024.0, 2) as size_mb,
    fc.modified_at as file_modified_at,
    fc.last_backup_at,
    CASE 
        WHEN fc.last_backup_at IS NULL THEN 'NUNCA'
        ELSE CURRENT_DATE - fc.last_backup_at::DATE || ' dias atrás'
    END as days_since_backup,
    CASE
        WHEN fc.file_name ~* '(contrato|contract|financeiro|senha|password|key|chave)' THEN 'CRÍTICO'
        WHEN fc.file_path ~* '(documents|contratos|financeiro|backup)' THEN 'IMPORTANTE'
        ELSE 'NORMAL'
    END as criticality_level
FROM file_catalog fc
WHERE fc.is_active = true
  AND (
    fc.last_backup_at IS NULL 
    OR fc.last_backup_at < CURRENT_DATE - INTERVAL '7 days'
  )
  AND fc.file_size > 1024 -- Arquivos maiores que 1KB
ORDER BY 
    CASE 
        WHEN fc.file_name ~* '(contrato|contract|financeiro|senha|password|key|chave)' THEN 1
        WHEN fc.file_path ~* '(documents|contratos|financeiro|backup)' THEN 2
        ELSE 3
    END,
    fc.file_size DESC;

-- 5. Estatísticas de Storage por Provedor Cloud
CREATE OR REPLACE VIEW storage_by_provider AS
SELECT 
    cp.name as provider_name,
    cp.provider_type,
    COUNT(DISTINCT bj.id) as active_jobs,
    SUM(bel.bytes_transferred) FILTER(WHERE bel.status = 'completed') as total_bytes_stored,
    ROUND(
        SUM(bel.bytes_transferred) FILTER(WHERE bel.status = 'completed') / 1024.0 / 1024.0 / 1024.0, 2
    ) as total_gb_stored,
    COUNT(*) FILTER(WHERE bel.status = 'completed') as successful_transfers,
    COUNT(*) FILTER(WHERE bel.status = 'failed') as failed_transfers,
    AVG(bel.transfer_rate_mbps) FILTER(WHERE bel.status = 'completed') as avg_transfer_rate_mbps,
    MAX(bel.started_at) as last_backup_date
FROM cloud_providers cp
LEFT JOIN backup_jobs bj ON cp.id = ANY(bj.backup_job_ids) -- Assumindo array de provider IDs
LEFT JOIN backup_execution_logs bel ON bj.id = bel.backup_job_id
WHERE cp.is_active = true
GROUP BY cp.id, cp.name, cp.provider_type
ORDER BY total_bytes_stored DESC NULLS LAST;

-- 6. Relatório Executivo - Dashboard KPIs
CREATE OR REPLACE VIEW executive_dashboard_kpis AS
SELECT 
    -- Estatísticas gerais
    (SELECT COUNT(*) FROM backup_jobs WHERE is_active = true) as active_backup_jobs,
    (SELECT COUNT(*) FROM cloud_providers WHERE is_active = true) as active_cloud_providers,
    
    -- Performance últimos 30 dias
    (SELECT 
        ROUND(COUNT(*) FILTER(WHERE status = 'completed')::NUMERIC / COUNT(*) * 100, 1)
        FROM backup_execution_logs 
        WHERE started_at >= CURRENT_DATE - INTERVAL '30 days'
    ) as success_rate_30d,
    
    -- Dados transferidos último mês
    (SELECT 
        ROUND(SUM(bytes_transferred) / 1024.0 / 1024.0 / 1024.0, 2)
        FROM backup_execution_logs 
        WHERE started_at >= CURRENT_DATE - INTERVAL '30 days'
          AND status = 'completed'
    ) as gb_transferred_30d,
    
    -- Arquivos críticos
    (SELECT COUNT(*) 
        FROM file_catalog 
        WHERE is_active = true 
          AND (file_name ~* '(contrato|contract|financeiro|senha|password)' 
               OR file_path ~* '(contracts|financial|confidential)')
    ) as critical_files_count,
    
    -- Arquivos sem backup recente
    (SELECT COUNT(*) 
        FROM file_catalog 
        WHERE is_active = true 
          AND (last_backup_at IS NULL OR last_backup_at < CURRENT_DATE - INTERVAL '7 days')
          AND file_size > 1024
    ) as files_no_recent_backup,
    
    -- Último backup
    (SELECT MAX(started_at) FROM backup_execution_logs) as last_backup_timestamp,
    
    -- Storage total
    (SELECT 
        ROUND(SUM(bytes_transferred) / 1024.0 / 1024.0 / 1024.0, 2)
        FROM backup_execution_logs 
        WHERE status = 'completed'
    ) as total_storage_gb;

-- 7. Alerta de Problemas Críticos
CREATE OR REPLACE VIEW critical_alerts AS
SELECT 
    'BACKUP_FAILURE' as alert_type,
    'CRÍTICO' as severity,
    'Job "' || bj.name || '" falhando há ' || 
    EXTRACT(DAY FROM CURRENT_TIMESTAMP - MAX(bel.started_at)) || ' dias' as message,
    MAX(bel.started_at) as last_occurrence,
    bj.id as related_job_id
FROM backup_jobs bj
JOIN backup_execution_logs bel ON bj.id = bel.backup_job_id
WHERE bj.is_active = true
  AND bel.started_at >= CURRENT_DATE - INTERVAL '7 days'
GROUP BY bj.id, bj.name
HAVING COUNT(*) FILTER(WHERE bel.status = 'completed') = 0

UNION ALL

SELECT 
    'CRITICAL_FILES_NO_BACKUP' as alert_type,
    'ALTO' as severity,
    COUNT(*) || ' arquivos críticos sem backup há mais de 7 dias' as message,
    MAX(fc.modified_at) as last_occurrence,
    NULL as related_job_id
FROM file_catalog fc
WHERE fc.is_active = true
  AND fc.file_name ~* '(contrato|contract|financeiro|senha|password|key|chave)'
  AND (fc.last_backup_at IS NULL OR fc.last_backup_at < CURRENT_DATE - INTERVAL '7 days')
  AND fc.file_size > 1024
HAVING COUNT(*) > 0

UNION ALL

SELECT 
    'STORAGE_GROWTH' as alert_type,
    'MÉDIO' as severity,
    'Crescimento de storage acima de 20% no último mês' as message,
    CURRENT_TIMESTAMP as last_occurrence,
    NULL as related_job_id
FROM (
    SELECT 
        SUM(bytes_transferred) FILTER(WHERE started_at >= CURRENT_DATE - INTERVAL '30 days') as current_month,
        SUM(bytes_transferred) FILTER(WHERE started_at >= CURRENT_DATE - INTERVAL '60 days' 
                                            AND started_at < CURRENT_DATE - INTERVAL '30 days') as previous_month
    FROM backup_execution_logs
    WHERE status = 'completed'
) growth
WHERE previous_month > 0 
  AND (current_month - previous_month)::NUMERIC / previous_month > 0.20

ORDER BY 
    CASE severity 
        WHEN 'CRÍTICO' THEN 1 
        WHEN 'ALTO' THEN 2 
        WHEN 'MÉDIO' THEN 3 
        ELSE 4 
    END,
    last_occurrence DESC;

-- 8. Função para gerar relatório completo
CREATE OR REPLACE FUNCTION generate_backup_report(report_date DATE DEFAULT CURRENT_DATE)
RETURNS TABLE (
    section VARCHAR,
    metric VARCHAR,
    value TEXT,
    details JSONB
) AS $$
BEGIN
    -- KPIs Gerais
    RETURN QUERY
    SELECT 
        'KPIs'::VARCHAR as section,
        'Taxa de Sucesso (30d)'::VARCHAR as metric,
        success_rate_30d || '%' as value,
        jsonb_build_object('period', '30 days') as details
    FROM executive_dashboard_kpis;
    
    -- Adicionar mais seções conforme necessário...
    
END;
$$ LANGUAGE plpgsql;

-- Comentários de uso
COMMENT ON VIEW backup_performance_monthly IS 'Performance mensal dos backups com métricas de sucesso e throughput';
COMMENT ON VIEW top_failing_jobs IS 'Jobs com maior taxa de falha nos últimos 30 dias';
COMMENT ON VIEW critical_files_no_recent_backup IS 'Arquivos críticos que não foram backupeados recentemente';
COMMENT ON VIEW executive_dashboard_kpis IS 'KPIs principais para dashboard executivo';
COMMENT ON VIEW critical_alerts IS 'Alertas críticos que requerem atenção imediata';

-- Índices para performance
CREATE INDEX IF NOT EXISTS idx_backup_execution_logs_started_at_status 
ON backup_execution_logs(started_at, status);

CREATE INDEX IF NOT EXISTS idx_file_catalog_backup_status 
ON file_catalog(last_backup_at, is_active) WHERE file_size > 1024;

-- Grants para usuário de relatórios
-- GRANT SELECT ON ALL TABLES IN SCHEMA public TO backup_reporter;
-- GRANT USAGE ON SCHEMA public TO backup_reporter;