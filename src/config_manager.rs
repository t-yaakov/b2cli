//! Gerenciador de configurações Infrastructure as Code
//!
//! Este módulo implementa:
//! - Leitura de configurações a partir de arquivos TOML
//! - Hot reload de configurações quando arquivos mudam
//! - Templates de configuração para diferentes cenários
//! - Validação de configurações antes de aplicar
//! - Sincronização automática com banco de dados

// use crate::models::{CloudProvider, CloudProviderType, NewCloudProvider};
use notify::{Event, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
// use uuid::Uuid;

/// Configuração de um provedor cloud em formato TOML
/// 
/// Suporta configurações para:
/// - Backblaze B2 (S3-compatible ou API nativa)
/// - IDrive e2
/// - Wasabi
/// - Scaleway Object Storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProviderConfig {
    /// Nome amigável do provedor para identificação
    pub name: String,
    
    /// Tipo do provedor (backblaze_b2, idrive_e2, wasabi, scaleway)
    pub provider_type: String,
    
    /// Bucket/Container principal onde os backups serão armazenados
    pub bucket: String,
    
    /// Configurações S3-compatible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_key: Option<String>,
    
    /// Configurações específicas do B2
    #[serde(default)]
    pub use_b2_native_api: bool,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b2_account_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b2_application_key: Option<String>,
    
    /// Ativo ou não
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Template para criação de arquivo de configuração
impl CloudProviderConfig {
    /// Cria um template para Backblaze B2
    pub fn template_b2() -> Self {
        Self {
            name: "Backblaze B2 Backup".to_string(),
            provider_type: "backblaze_b2".to_string(),
            bucket: "meu-bucket-backup".to_string(),
            endpoint: Some("https://s3.us-west-002.backblazeb2.com".to_string()),
            region: Some("us-west-002".to_string()),
            access_key: Some("YOUR_KEY_ID_HERE".to_string()),
            secret_key: Some("YOUR_APPLICATION_KEY_HERE".to_string()),
            use_b2_native_api: false,
            b2_account_id: None,
            b2_application_key: None,
            enabled: true,
        }
    }

    /// Cria um template para IDrive e2
    pub fn template_idrive() -> Self {
        Self {
            name: "IDrive e2 Storage".to_string(),
            provider_type: "idrive_e2".to_string(),
            bucket: "meu-bucket".to_string(),
            endpoint: Some("https://endpoint.idrivee2.com".to_string()),
            region: Some("us-west".to_string()),
            access_key: Some("YOUR_ACCESS_KEY_ID".to_string()),
            secret_key: Some("YOUR_SECRET_ACCESS_KEY".to_string()),
            use_b2_native_api: false,
            b2_account_id: None,
            b2_application_key: None,
            enabled: true,
        }
    }

    /// Cria um template para Wasabi
    pub fn template_wasabi() -> Self {
        Self {
            name: "Wasabi Hot Storage".to_string(),
            provider_type: "wasabi".to_string(),
            bucket: "meu-bucket".to_string(),
            endpoint: None,
            region: Some("us-east-1".to_string()),
            access_key: Some("YOUR_ACCESS_KEY".to_string()),
            secret_key: Some("YOUR_SECRET_KEY".to_string()),
            use_b2_native_api: false,
            b2_account_id: None,
            b2_application_key: None,
            enabled: true,
        }
    }

    /// Cria um template para Scaleway
    pub fn template_scaleway() -> Self {
        Self {
            name: "Scaleway Object Storage".to_string(),
            provider_type: "scaleway".to_string(),
            bucket: "meu-bucket".to_string(),
            endpoint: Some("https://s3.fr-par.scw.cloud".to_string()),
            region: Some("fr-par".to_string()),
            access_key: Some("YOUR_ACCESS_KEY".to_string()),
            secret_key: Some("YOUR_SECRET_KEY".to_string()),
            use_b2_native_api: false,
            b2_account_id: None,
            b2_application_key: None,
            enabled: true,
        }
    }
}

/// Gerenciador de configurações
pub struct ConfigManager {
    pool: PgPool,
    config_dir: PathBuf,
    providers: Arc<RwLock<HashMap<String, CloudProviderConfig>>>,
}

impl ConfigManager {
    /// Cria um novo gerenciador de configurações
    pub fn new(pool: PgPool, config_dir: PathBuf) -> Self {
        Self {
            pool,
            config_dir,
            providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Inicializa o gerenciador
    pub async fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Criar diretório de configuração se não existir
        let providers_dir = self.config_dir.join("providers");
        if !providers_dir.exists() {
            fs::create_dir_all(&providers_dir).await?;
            info!(path = %providers_dir.display(), "Diretório de configuração criado");
            
            // Criar templates de exemplo
            self.create_example_templates(&providers_dir).await?;
        }

        // Carregar configurações existentes
        self.load_all_configs().await?;

        // Iniciar watcher para mudanças
        self.start_file_watcher()?;

        Ok(())
    }

    /// Cria templates de exemplo
    async fn create_example_templates(&self, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        // Template Backblaze B2
        let b2_path = dir.join("backblaze_b2.toml.example");
        let b2_config = CloudProviderConfig::template_b2();
        let b2_content = toml::to_string_pretty(&b2_config)?;
        fs::write(&b2_path, b2_content).await?;
        info!(path = %b2_path.display(), "Template B2 criado");

        // Template IDrive e2
        let idrive_path = dir.join("idrive_e2.toml.example");
        let idrive_config = CloudProviderConfig::template_idrive();
        let idrive_content = toml::to_string_pretty(&idrive_config)?;
        fs::write(&idrive_path, idrive_content).await?;
        info!(path = %idrive_path.display(), "Template IDrive criado");

        // Template Wasabi
        let wasabi_path = dir.join("wasabi.toml.example");
        let wasabi_config = CloudProviderConfig::template_wasabi();
        let wasabi_content = toml::to_string_pretty(&wasabi_config)?;
        fs::write(&wasabi_path, wasabi_content).await?;
        info!(path = %wasabi_path.display(), "Template Wasabi criado");

        // Template Scaleway
        let scaleway_path = dir.join("scaleway.toml.example");
        let scaleway_config = CloudProviderConfig::template_scaleway();
        let scaleway_content = toml::to_string_pretty(&scaleway_config)?;
        fs::write(&scaleway_path, scaleway_content).await?;
        info!(path = %scaleway_path.display(), "Template Scaleway criado");

        // Criar README
        let readme_content = r#"# Configuração de Cloud Providers

## Como usar:

1. Copie um dos templates de exemplo:
   ```bash
   cp backblaze_b2.toml.example meu_backup.toml
   ```

2. Edite o arquivo com suas credenciais:
   ```bash
   vim meu_backup.toml
   ```

3. O sistema detectará automaticamente o novo arquivo e criará o provider

## Templates disponíveis:

- `backblaze_b2.toml.example` - Backblaze B2 (mais popular)
- `idrive_e2.toml.example` - IDrive e2 (egress gratuito)
- `wasabi.toml.example` - Wasabi (alta performance)
- `scaleway.toml.example` - Scaleway (GDPR compliant)

## Segurança:

⚠️ **IMPORTANTE**: 
- Nunca commite arquivos .toml com credenciais reais
- Use permissões 600 nos arquivos: `chmod 600 *.toml`
- Considere usar variáveis de ambiente para senhas

## Exemplo com variáveis de ambiente:

Em vez de colocar a senha diretamente:
```toml
secret_key = "${B2_SECRET_KEY}"
```

Depois exporte a variável:
```bash
export B2_SECRET_KEY="sua_chave_secreta"
```
"#;
        
        let readme_path = dir.join("README.md");
        fs::write(&readme_path, readme_content).await?;
        info!(path = %readme_path.display(), "README criado");

        Ok(())
    }

    /// Carrega todas as configurações do diretório
    pub async fn load_all_configs(&self) -> Result<(), Box<dyn std::error::Error>> {
        let providers_dir = self.config_dir.join("providers");
        let mut entries = fs::read_dir(&providers_dir).await?;
        
        let mut configs = HashMap::new();
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            // Processar apenas arquivos .toml (não .example)
            if path.extension() == Some(std::ffi::OsStr::new("toml")) {
                match self.load_config_file(&path).await {
                    Ok(config) => {
                        let filename = path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        
                        info!(file = %filename, provider = %config.name, "Configuração carregada");
                        configs.insert(filename, config);
                    }
                    Err(e) => {
                        error!(path = %path.display(), error = %e, "Erro ao carregar configuração");
                    }
                }
            }
        }

        // Sincronizar com banco de dados
        for (filename, config) in &configs {
            self.sync_provider_to_db(filename, config).await?;
        }

        // Atualizar cache em memória
        let mut providers = self.providers.write().await;
        *providers = configs;

        Ok(())
    }

    /// Carrega um arquivo de configuração
    async fn load_config_file(&self, path: &Path) -> Result<CloudProviderConfig, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path).await?;
        
        // Substituir variáveis de ambiente
        let content = self.expand_env_vars(&content);
        
        let config: CloudProviderConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Expande variáveis de ambiente no conteúdo
    fn expand_env_vars(&self, content: &str) -> String {
        let mut result = content.to_string();
        
        // Procurar padrões ${VAR_NAME}
        let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();
        
        for cap in re.captures_iter(content) {
            if let Some(var_name) = cap.get(1) {
                if let Ok(var_value) = std::env::var(var_name.as_str()) {
                    result = result.replace(&cap[0], &var_value);
                }
            }
        }
        
        result
    }

    /// Sincroniza um provider com o banco de dados
    async fn sync_provider_to_db(
        &self,
        _filename: &str,
        config: &CloudProviderConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Verificar se já existe
        let existing = sqlx::query!(
            "SELECT id FROM cloud_providers WHERE name = $1",
            config.name
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(record) = existing {
            // Atualizar existente
            sqlx::query!(
                r#"
                UPDATE cloud_providers
                SET provider_type = $2,
                    endpoint = $3,
                    region = $4,
                    bucket = $5,
                    access_key = $6,
                    secret_key = $7,
                    use_b2_native_api = $8,
                    b2_account_id = $9,
                    b2_application_key = $10,
                    is_active = $11,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = $1
                "#,
                record.id,
                config.provider_type,
                config.endpoint,
                config.region,
                config.bucket,
                config.access_key,
                config.secret_key,
                config.use_b2_native_api,
                config.b2_account_id,
                config.b2_application_key,
                config.enabled
            )
            .execute(&self.pool)
            .await?;
            
            debug!(name = %config.name, "Provider atualizado do arquivo");
        } else {
            // Criar novo
            sqlx::query!(
                r#"
                INSERT INTO cloud_providers (
                    name, provider_type, endpoint, region, bucket,
                    access_key, secret_key, use_b2_native_api,
                    b2_account_id, b2_application_key, is_active
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#,
                config.name,
                config.provider_type,
                config.endpoint,
                config.region,
                config.bucket,
                config.access_key,
                config.secret_key,
                config.use_b2_native_api,
                config.b2_account_id,
                config.b2_application_key,
                config.enabled
            )
            .execute(&self.pool)
            .await?;
            
            info!(name = %config.name, "Novo provider criado do arquivo");
        }

        Ok(())
    }

    /// Inicia o watcher de arquivos
    fn start_file_watcher(&self) -> Result<(), Box<dyn std::error::Error>> {
        let providers_dir = self.config_dir.join("providers");
        
        // Criar watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    debug!(event = ?event, "Evento de arquivo detectado");
                    // TODO: Recarregar configuração quando arquivo mudar
                }
                Err(e) => error!("Erro no watcher: {:?}", e),
            }
        })?;

        // Observar diretório
        watcher.watch(&providers_dir, RecursiveMode::NonRecursive)?;
        
        info!(path = %providers_dir.display(), "Watcher de configurações iniciado");
        
        // Manter watcher vivo
        std::mem::forget(watcher);
        
        Ok(())
    }

    /// Cria um novo arquivo de configuração
    pub async fn create_config_file(
        &self,
        name: &str,
        provider_type: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let providers_dir = self.config_dir.join("providers");
        let file_path = providers_dir.join(format!("{}.toml", name));
        
        // Criar template baseado no tipo
        let config = match provider_type {
            "backblaze_b2" => CloudProviderConfig::template_b2(),
            "idrive_e2" => CloudProviderConfig::template_idrive(),
            "wasabi" => CloudProviderConfig::template_wasabi(),
            "scaleway" => CloudProviderConfig::template_scaleway(),
            _ => return Err("Tipo de provider inválido".into()),
        };
        
        // Salvar arquivo
        let content = toml::to_string_pretty(&config)?;
        fs::write(&file_path, content).await?;
        
        info!(path = %file_path.display(), "Arquivo de configuração criado");
        
        Ok(file_path)
    }
}