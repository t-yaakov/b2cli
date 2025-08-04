-- Tabela para armazenar configurações de provedores cloud
CREATE TABLE cloud_providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL UNIQUE,
    provider_type VARCHAR(50) NOT NULL, -- 'backblaze_b2', 'idrive_e2', 'wasabi', 'scaleway'
    
    -- Configurações genéricas S3-compatible
    endpoint VARCHAR(500),           -- S3 endpoint URL
    region VARCHAR(100),              -- Região do provedor
    bucket VARCHAR(255) NOT NULL,     -- Nome do bucket
    path_prefix VARCHAR(500),         -- Prefixo opcional no bucket
    
    -- Credenciais (serão criptografadas em produção)
    access_key VARCHAR(500) NOT NULL,
    secret_key VARCHAR(500) NOT NULL,
    
    -- Configurações específicas do Backblaze B2
    b2_account_id VARCHAR(255),      -- Para API nativa B2
    b2_application_key VARCHAR(500),  -- Para API nativa B2
    use_b2_native_api BOOLEAN DEFAULT false,
    
    -- Metadados
    is_active BOOLEAN DEFAULT true,
    is_default BOOLEAN DEFAULT false,  -- Provider padrão
    test_connectivity_at TIMESTAMP,    -- Última vez que testou conectividade
    test_connectivity_status VARCHAR(50), -- 'success', 'failed', 'pending'
    test_connectivity_message TEXT,    -- Mensagem de erro/sucesso
    
    -- Métricas de uso
    total_storage_bytes BIGINT DEFAULT 0,
    total_egress_bytes BIGINT DEFAULT 0,
    last_sync_at TIMESTAMP,
    
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    
    CONSTRAINT unique_default_provider EXCLUDE (is_default WITH =) WHERE (is_default = true AND is_active = true)
);

-- Índices para performance
CREATE INDEX idx_cloud_providers_active ON cloud_providers(is_active);
CREATE INDEX idx_cloud_providers_type ON cloud_providers(provider_type);
CREATE INDEX idx_cloud_providers_default ON cloud_providers(is_default) WHERE is_default = true;

-- Comentários para documentação
COMMENT ON TABLE cloud_providers IS 'Configurações de provedores de armazenamento cloud para backup';
COMMENT ON COLUMN cloud_providers.provider_type IS 'Tipo do provedor: backblaze_b2, idrive_e2, wasabi, scaleway';
COMMENT ON COLUMN cloud_providers.use_b2_native_api IS 'Se true, usa API nativa B2. Se false, usa API S3-compatible';
COMMENT ON COLUMN cloud_providers.is_default IS 'Apenas um provider pode ser padrão por vez';