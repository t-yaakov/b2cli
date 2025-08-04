use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use sqlx::FromRow;
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, ToSchema, Debug, FromRow)]
pub struct BackupJob {
    #[serde(skip_deserializing)]
    pub id: Uuid,
    pub name: String,
    #[schema(example = json!({ "/home/user/docs": ["/mnt/backups/daily", "s3://my-bucket/daily"] }),
              value_type = HashMap<String, Vec<String>>)]
    pub mappings: serde_json::Value,
    #[serde(skip_deserializing)]
    pub created_at: DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub updated_at: DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub deleted_at: Option<DateTime<Utc>>,
    pub status: String,
    #[serde(skip_deserializing)]
    pub is_active: bool,
}

// A version of BackupJob for creating new entries, without the ID
#[derive(Deserialize, ToSchema)]
pub struct NewBackupJob {
    pub schedule: Option<NewBackupSchedule>,
    pub name: String,
    pub mappings: HashMap<String, Vec<String>>,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub message: String,
}

#[derive(Serialize, Deserialize, ToSchema, Debug, FromRow)]
pub struct BackedUpFile {
    #[serde(skip_deserializing)]
    pub id: Uuid,
    pub backup_job_id: Uuid,
    pub original_path: String,
    pub backed_up_path: String,
    pub file_name: String,
    pub file_extension: String,
    pub file_size: i64,
    pub checksum: String,
    #[serde(skip_deserializing)]
    pub backed_up_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema, Debug, FromRow)]
pub struct BackupSchedule {
    #[serde(skip_deserializing)]
    pub id: Uuid,
    pub backup_job_id: Uuid,
    pub name: String,
    pub cron_expression: String,
    pub enabled: bool,
    #[serde(skip_deserializing)]
    pub next_run: Option<DateTime<Utc>>,
    #[serde(skip_deserializing)]
    pub last_run: Option<DateTime<Utc>>,
    #[serde(skip_deserializing)]
    pub last_status: String,
    #[serde(skip_deserializing)]
    pub created_at: DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct NewBackupSchedule {
    pub name: String,
    #[schema(example = "0 17 * * *")]
    pub cron_expression: String,
    pub enabled: Option<bool>,
}

// Update models for PATCH operations
#[derive(Deserialize, ToSchema)]
pub struct UpdateBackupJob {
    pub name: Option<String>,
    pub mappings: Option<HashMap<String, Vec<String>>>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateBackupSchedule {
    pub name: Option<String>,
    #[schema(example = "0 18 * * *")]
    pub cron_expression: Option<String>,
    pub enabled: Option<bool>,
}

// Backup execution logs
#[derive(Serialize, Deserialize, ToSchema, Debug, FromRow)]
pub struct BackupExecutionLog {
    pub id: Uuid,
    pub backup_job_id: Uuid,
    pub schedule_id: Option<Uuid>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: String,
    pub rclone_command: String,
    pub source_path: String,
    pub destination_path: String,
    pub rclone_config: Option<serde_json::Value>,
    pub files_transferred: Option<i32>,
    pub files_checked: Option<i32>,
    pub files_deleted: Option<i32>,
    pub bytes_transferred: Option<i64>,
    pub transfer_rate_mbps: Option<f32>,
    pub duration_seconds: Option<i32>,
    pub error_count: Option<i32>,
    pub retry_count: Option<i32>,
    pub error_message: Option<String>,
    pub rclone_stdout: Option<String>,
    pub rclone_stderr: Option<String>,
    pub rclone_log_file_path: Option<String>,
    pub triggered_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct NewBackupExecutionLog {
    pub backup_job_id: Uuid,
    pub schedule_id: Option<Uuid>,
    pub rclone_command: String,
    pub source_path: String,
    pub destination_path: String,
    pub rclone_config: Option<serde_json::Value>,
    pub triggered_by: Option<String>,
}

// Rclone specific models
#[derive(Debug, Deserialize)]
pub struct RcloneLogEntry {
    pub level: String,
    pub msg: String,
    pub time: String,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct RcloneExecutionResult {
    pub exit_code: i32,
    pub files_transferred: i32,
    pub files_checked: i32,
    pub files_deleted: i32,
    pub bytes_transferred: i64,
    pub transfer_rate_mbps: f32,
    pub duration_seconds: i32,
    pub error_count: i32,
    pub errors: Vec<String>,
    pub stdout: String,
    pub stderr: String,
}

// ========================================
// CLOUD PROVIDERS MODELS
// ========================================

/// Tipos de provedores cloud suportados
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CloudProviderType {
    /// Backblaze B2 - Focado em backup com boa relação custo-benefício
    BackblazeB2,
    /// IDrive e2 - Mais barato com egress gratuito
    IdriveE2,
    /// Wasabi - Performance alta com egress gratuito (até limites)
    Wasabi,
    /// Scaleway - GDPR compliant, baseado na Europa
    Scaleway,
}

/// Status de teste de conectividade
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConnectivityStatus {
    /// Conectividade testada com sucesso
    Success,
    /// Falha no teste de conectividade
    Failed,
    /// Teste pendente/nunca executado
    Pending,
}

/// Provedor de armazenamento cloud configurado
#[derive(Serialize, Deserialize, ToSchema, Debug, FromRow)]
pub struct CloudProvider {
    /// ID único do provedor
    #[serde(skip_deserializing)]
    pub id: Uuid,
    /// Nome descritivo para identificação
    pub name: String,
    /// Tipo do provedor (backblaze_b2, idrive_e2, etc.)
    pub provider_type: String, // Stored as string in DB, converted to/from enum
    
    // Configurações S3-compatible
    /// Endpoint S3 do provedor
    pub endpoint: Option<String>,
    /// Região do provedor
    pub region: Option<String>,
    /// Nome do bucket para armazenamento
    pub bucket: String,
    /// Prefixo opcional no bucket (ex: "backups/")
    pub path_prefix: Option<String>,
    
    // Credenciais (criptografadas)
    /// Access key / Key ID
    #[serde(skip_serializing)]
    pub access_key: String,
    /// Secret key / Application key
    #[serde(skip_serializing)]
    pub secret_key: String,
    
    // Backblaze B2 específico
    /// Account ID para API nativa B2
    pub b2_account_id: Option<String>,
    /// Application key para API nativa B2
    #[serde(skip_serializing)]
    pub b2_application_key: Option<String>,
    /// Se deve usar API nativa B2 ao invés de S3-compatible
    pub use_b2_native_api: bool,
    
    // Status e metadados
    /// Se o provedor está ativo
    pub is_active: bool,
    /// Se é o provedor padrão
    pub is_default: bool,
    /// Última vez que testou conectividade
    pub test_connectivity_at: Option<DateTime<Utc>>,
    /// Status do último teste
    pub test_connectivity_status: Option<String>,
    /// Mensagem do último teste
    pub test_connectivity_message: Option<String>,
    
    // Métricas de uso
    /// Total de bytes armazenados
    pub total_storage_bytes: i64,
    /// Total de bytes baixados (egress)
    pub total_egress_bytes: i64,
    /// Última sincronização
    pub last_sync_at: Option<DateTime<Utc>>,
    
    #[serde(skip_deserializing)]
    pub created_at: DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub updated_at: DateTime<Utc>,
}

/// Dados para criar um novo cloud provider
#[derive(Serialize, Deserialize, ToSchema)]
pub struct NewCloudProvider {
    /// Nome descritivo único
    #[schema(example = "My Backblaze B2")]
    pub name: String,
    /// Tipo do provedor
    pub provider_type: CloudProviderType,
    
    // Configurações S3
    /// Endpoint customizado (opcional para alguns provedores)
    #[schema(example = "https://s3.us-west-002.backblazeb2.com")]
    pub endpoint: Option<String>,
    /// Região do provedor
    #[schema(example = "us-west-002")]
    pub region: Option<String>,
    /// Nome do bucket
    #[schema(example = "my-backup-bucket")]
    pub bucket: String,
    /// Prefixo opcional
    #[schema(example = "backups/")]
    pub path_prefix: Option<String>,
    
    // Credenciais
    /// Access key ou Key ID
    #[schema(example = "your-access-key-id")]
    pub access_key: String,
    /// Secret key ou Application key
    #[schema(example = "your-secret-access-key")]
    pub secret_key: String,
    
    // Backblaze B2 específico
    /// Account ID para API nativa
    pub b2_account_id: Option<String>,
    /// Application key para API nativa
    pub b2_application_key: Option<String>,
    /// Usar API nativa B2
    pub use_b2_native_api: Option<bool>,
    
    /// Se deve ser o provedor padrão
    pub is_default: Option<bool>,
    /// Testar conectividade após criar
    pub test_connectivity: Option<bool>,
}

/// Dados para atualizar um cloud provider
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCloudProvider {
    /// Nome descritivo
    pub name: Option<String>,
    /// Endpoint customizado
    pub endpoint: Option<String>,
    /// Região
    pub region: Option<String>,
    /// Nome do bucket
    pub bucket: Option<String>,
    /// Prefixo no bucket
    pub path_prefix: Option<String>,
    
    // Credenciais (opcional para security)
    /// Nova access key
    pub access_key: Option<String>,
    /// Nova secret key
    pub secret_key: Option<String>,
    
    // Backblaze B2 específico
    /// Account ID
    pub b2_account_id: Option<String>,
    /// Application key
    pub b2_application_key: Option<String>,
    /// Usar API nativa
    pub use_b2_native_api: Option<bool>,
    
    /// Ativar/desativar
    pub is_active: Option<bool>,
    /// Tornar padrão
    pub is_default: Option<bool>,
}

/// Resultado do teste de conectividade
#[derive(Serialize, ToSchema)]
pub struct ConnectivityTestResult {
    /// Se o teste foi bem sucedido
    pub success: bool,
    /// Status do teste
    pub status: ConnectivityStatus,
    /// Mensagem descritiva
    pub message: String,
    /// Timestamp do teste
    pub tested_at: DateTime<Utc>,
    /// Detalhes adicionais (latency, etc.)
    pub details: Option<serde_json::Value>,
}

/// Resumo de configuração do rclone para um provider
#[derive(Serialize, ToSchema)]
pub struct RcloneConfig {
    /// Nome do remote no rclone
    pub remote_name: String,
    /// Tipo (s3 ou b2)
    pub remote_type: String,
    /// Configuração gerada
    pub config_section: String,
}