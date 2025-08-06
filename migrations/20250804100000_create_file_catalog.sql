-- Tabela para catalogar todos os arquivos encontrados no sistema
CREATE TABLE IF NOT EXISTS file_catalog (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Informações básicas do arquivo
    file_path TEXT NOT NULL,           -- Caminho completo (será criptografado)
    file_name TEXT NOT NULL,           -- Nome do arquivo
    extension VARCHAR(50),             -- Extensão (.pdf, .doc, etc)
    mime_type VARCHAR(100),            -- MIME type detectado
    
    -- Metadados
    file_size BIGINT NOT NULL,         -- Tamanho em bytes
    created_at TIMESTAMP,              -- Data de criação do arquivo
    modified_at TIMESTAMP,             -- Última modificação
    accessed_at TIMESTAMP,             -- Último acesso
    
    -- Análise e classificação
    content_hash VARCHAR(64),          -- SHA256 do conteúdo
    is_duplicate BOOLEAN DEFAULT FALSE, -- Se é duplicata de outro arquivo
    duplicate_of UUID REFERENCES file_catalog(id), -- Referência ao arquivo original
    
    -- Hierarquia
    parent_directory TEXT,             -- Diretório pai (criptografado)
    depth INTEGER,                     -- Profundidade na árvore de diretórios
    
    -- Status
    is_active BOOLEAN DEFAULT TRUE,    -- Se o arquivo ainda existe
    last_scan_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP, -- Última varredura
    scan_error TEXT,                   -- Erro durante scan, se houver
    
    -- Backup status
    backup_count INTEGER DEFAULT 0,    -- Quantas vezes foi backupeado
    last_backup_at TIMESTAMP,          -- Último backup realizado
    backup_job_ids UUID[] DEFAULT '{}', -- Jobs que incluem este arquivo
    
    -- Metadados adicionais (JSON para flexibilidade)
    metadata JSONB DEFAULT '{}',       -- Metadados extras (permissões, owner, etc)
    
    -- Timestamps do sistema
    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Índices para buscas eficientes
CREATE INDEX idx_file_catalog_path ON file_catalog(file_path);
CREATE INDEX idx_file_catalog_name ON file_catalog(file_name);
CREATE INDEX idx_file_catalog_extension ON file_catalog(extension);
CREATE INDEX idx_file_catalog_size ON file_catalog(file_size);
CREATE INDEX idx_file_catalog_modified ON file_catalog(modified_at DESC);
CREATE INDEX idx_file_catalog_hash ON file_catalog(content_hash);
CREATE INDEX idx_file_catalog_parent ON file_catalog(parent_directory);
CREATE INDEX idx_file_catalog_active ON file_catalog(is_active);

-- Índice para busca full-text no nome do arquivo
CREATE INDEX idx_file_catalog_name_text ON file_catalog USING gin(to_tsvector('english', file_name));

-- Tabela para diretórios (otimiza consultas de hierarquia)
CREATE TABLE IF NOT EXISTS directory_catalog (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Informações do diretório
    directory_path TEXT NOT NULL UNIQUE, -- Caminho completo (criptografado)
    directory_name TEXT NOT NULL,        -- Nome do diretório
    parent_path TEXT,                    -- Caminho do diretório pai
    depth INTEGER NOT NULL,              -- Profundidade na árvore
    
    -- Estatísticas
    total_files BIGINT DEFAULT 0,        -- Total de arquivos (recursivo)
    direct_files BIGINT DEFAULT 0,       -- Arquivos diretos (não recursivo)
    total_size BIGINT DEFAULT 0,         -- Tamanho total em bytes
    subdirectory_count INTEGER DEFAULT 0, -- Número de subdiretórios
    
    -- Análise
    file_types JSONB DEFAULT '{}',       -- {"pdf": 10, "doc": 5, ...}
    largest_file UUID REFERENCES file_catalog(id), -- Maior arquivo
    oldest_file UUID REFERENCES file_catalog(id),  -- Arquivo mais antigo
    newest_file UUID REFERENCES file_catalog(id),  -- Arquivo mais recente
    
    -- Status
    is_active BOOLEAN DEFAULT TRUE,
    last_scan_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    scan_duration_ms INTEGER,            -- Tempo de scan em ms
    
    -- Timestamps
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Índices para diretórios
CREATE INDEX idx_directory_path ON directory_catalog(directory_path);
CREATE INDEX idx_directory_parent ON directory_catalog(parent_path);
CREATE INDEX idx_directory_size ON directory_catalog(total_size DESC);
CREATE INDEX idx_directory_files ON directory_catalog(total_files DESC);

-- Tabela para jobs de varredura
CREATE TABLE IF NOT EXISTS scan_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Configuração do scan
    root_path TEXT NOT NULL,             -- Diretório raiz do scan
    recursive BOOLEAN DEFAULT TRUE,      -- Scan recursivo
    follow_symlinks BOOLEAN DEFAULT FALSE, -- Seguir links simbólicos
    max_depth INTEGER,                   -- Profundidade máxima
    
    -- Filtros
    include_patterns TEXT[],             -- Padrões para incluir (glob)
    exclude_patterns TEXT[],             -- Padrões para excluir (glob)
    min_file_size BIGINT,                -- Tamanho mínimo
    max_file_size BIGINT,                -- Tamanho máximo
    
    -- Status
    status VARCHAR(50) DEFAULT 'pending', -- pending, running, completed, failed
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    
    -- Estatísticas
    files_scanned BIGINT DEFAULT 0,
    directories_scanned BIGINT DEFAULT 0,
    total_size_bytes BIGINT DEFAULT 0,
    errors_count INTEGER DEFAULT 0,
    duration_seconds INTEGER,
    
    -- Resultados
    error_message TEXT,
    scan_output JSONB DEFAULT '{}',     -- Estatísticas detalhadas
    
    -- Timestamps
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Trigger para atualizar updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_file_catalog_updated_at BEFORE UPDATE ON file_catalog
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_directory_catalog_updated_at BEFORE UPDATE ON directory_catalog
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_scan_jobs_updated_at BEFORE UPDATE ON scan_jobs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();