//! Módulo de File Intelligence para varredura e catalogação de arquivos
//!
//! Este módulo fornece funcionalidades avançadas para:
//! - Varredura recursiva de diretórios com filtros
//! - Catalogação de arquivos com metadados completos
//! - Detecção de duplicatas via SHA256
//! - Estatísticas detalhadas por diretório
//! - Integração com PostgreSQL para persistência

use async_recursion::async_recursion;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Representa um arquivo catalogado no sistema
/// 
/// Esta struct contém todos os metadados de um arquivo incluindo:
/// - Informações básicas (nome, tamanho, caminho)
/// - Timestamps do sistema de arquivos
/// - Hash SHA256 para detecção de duplicatas
/// - Metadados adicionais em formato JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogedFile {
    pub id: Uuid,
    pub file_path: String,
    pub file_name: String,
    pub extension: Option<String>,
    pub mime_type: Option<String>,
    pub file_size: i64,
    pub created_at: Option<NaiveDateTime>,
    pub modified_at: Option<NaiveDateTime>,
    pub accessed_at: Option<NaiveDateTime>,
    pub content_hash: Option<String>,
    pub parent_directory: String,
    pub depth: i32,
    pub metadata: serde_json::Value,
}

/// Estatísticas agregadas de um diretório
/// 
/// Fornece uma visão consolidada de:
/// - Contagem de arquivos (total e diretos)
/// - Tamanho total em bytes
/// - Número de subdiretórios
/// - Distribuição por tipo de arquivo
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectoryStats {
    pub path: String,
    pub total_files: i64,
    pub direct_files: i64,
    pub total_size: i64,
    pub subdirectory_count: i32,
    pub file_types: HashMap<String, i32>,
}

/// Configuração para o scanner de arquivos
/// 
/// Define parâmetros como:
/// - Caminho raiz para varredura
/// - Se deve ser recursivo
/// - Filtros de inclusão/exclusão
/// - Limites de tamanho e profundidade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub root_path: PathBuf,
    pub recursive: bool,
    pub follow_symlinks: bool,
    pub max_depth: Option<i32>,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub min_file_size: Option<i64>,
    pub max_file_size: Option<i64>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            root_path: PathBuf::from("."),
            recursive: true,
            follow_symlinks: false,
            max_depth: None,
            include_patterns: vec![],
            exclude_patterns: vec![
                "*.tmp".to_string(),
                "*.cache".to_string(),
                ".git/*".to_string(),
                "node_modules/*".to_string(),
                "target/*".to_string(),
                "__pycache__/*".to_string(),
            ],
            min_file_size: None,
            max_file_size: None,
        }
    }
}

/// Scanner de arquivos principal do sistema
/// 
/// Responsável por:
/// - Varrer diretórios de forma assíncrona
/// - Catalogar arquivos no banco de dados
/// - Calcular hashes SHA256 para detecção de duplicatas
/// - Gerar estatísticas por diretório
/// - Integrar com o sistema de backup
pub struct FileScanner {
    pool: PgPool,
    config: ScanConfig,
    scan_job_id: Option<Uuid>,
}

impl FileScanner {
    /// Cria um novo scanner
    pub fn new(pool: PgPool, config: ScanConfig) -> Self {
        Self {
            pool,
            config,
            scan_job_id: None,
        }
    }

    /// Inicia a varredura
    pub async fn start_scan(&mut self) -> Result<Uuid, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("🔥 SCANNER: Iniciando start_scan()");
        tracing::debug!("🔥 SCANNER: Iniciando start_scan() - DEBUG");
        tracing::trace!("🔥 SCANNER: Iniciando start_scan() - TRACE");
        info!(
            root_path = %self.config.root_path.display(),
            recursive = self.config.recursive,
            "🔥 SCANNER: Iniciando varredura de arquivos"
        );

        // Criar job no banco
        tracing::info!("🔥 SCANNER: Criando job no banco de dados");
        debug!("🔥 SCANNER: Criando job no banco de dados");
        let job_id = self.create_scan_job().await?;
        self.scan_job_id = Some(job_id); // Armazenar o ID criado!
        tracing::info!(job_id = %job_id, "🔥 SCANNER: Job criado no banco");
        info!(job_id = %job_id, "🔥 SCANNER: Job criado no banco");

        // Atualizar status para running
        tracing::info!("🔥 SCANNER: Atualizando status do job para running");
        debug!("🔥 SCANNER: Atualizando status do job para running");
        sqlx::query!(
            "UPDATE scan_jobs SET status = 'running', started_at = CURRENT_TIMESTAMP WHERE id = $1",
            job_id
        )
        .execute(&self.pool)
        .await?;
        tracing::info!(job_id = %job_id, "🔥 SCANNER: Status atualizado para running");
        info!(job_id = %job_id, "🔥 SCANNER: Status atualizado para running");

        // Iniciar varredura
        debug!("🔥 SCANNER: Iniciando varredura do diretório");
        let mut stats = ScanStats::default();
        match self.scan_directory(&self.config.root_path, 0, &mut stats).await {
            Ok(_) => {
                info!("🔥 SCANNER: Varredura do diretório concluída");
            }
            Err(e) => {
                tracing::error!(error = %e, error_debug = ?e, "🔥 SCANNER: Erro durante varredura");
                return Err(e);
            }
        }

        // Atualizar job com estatísticas finais
        sqlx::query!(
            r#"
            UPDATE scan_jobs 
            SET status = 'completed',
                completed_at = CURRENT_TIMESTAMP,
                files_scanned = $2,
                directories_scanned = $3,
                total_size_bytes = $4,
                errors_count = $5,
                duration_seconds = EXTRACT(EPOCH FROM (CURRENT_TIMESTAMP - started_at))::INTEGER
            WHERE id = $1
            "#,
            job_id,
            stats.files_scanned,
            stats.directories_scanned,
            stats.total_size,
            stats.errors_count
        )
        .execute(&self.pool)
        .await?;

        info!(
            files = stats.files_scanned,
            directories = stats.directories_scanned,
            size_mb = stats.total_size / 1_048_576,
            "Varredura concluída"
        );

        Ok(job_id)
    }

    /// Varre um diretório recursivamente
    #[async_recursion]
    async fn scan_directory(
        &self,
        path: &Path,
        depth: i32,
        stats: &mut ScanStats,
    ) -> Result<DirectoryStats, Box<dyn std::error::Error + Send + Sync>> {
        debug!(path = %path.display(), depth = depth, "🔥 SCAN_DIR: Varrendo diretório");

        // Verificar profundidade máxima
        if let Some(max_depth) = self.config.max_depth {
            if depth > max_depth {
                return Ok(DirectoryStats::default());
            }
        }

        let mut dir_stats = DirectoryStats {
            path: path.to_string_lossy().to_string(),
            total_files: 0,
            direct_files: 0,
            total_size: 0,
            subdirectory_count: 0,
            file_types: HashMap::new(),
        };

        // Ler conteúdo do diretório
        debug!(path = %path.display(), "Lendo conteúdo do diretório");
        let mut entries = match fs::read_dir(path).await {
            Ok(entries) => {
                debug!(path = %path.display(), "Diretório lido com sucesso");
                entries
            }
            Err(e) => {
                tracing::error!(path = %path.display(), error = %e, "Erro ao ler diretório");
                return Err(e.into());
            }
        };
        
        debug!(path = %path.display(), "Iniciando loop de processamento de entries");
        let mut processed_count = 0;
        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            debug!(entry = %entry_path.display(), count = processed_count, "Processando entry");
            
            let metadata = match entry.metadata().await {
                Ok(m) => {
                    debug!(entry = %entry_path.display(), "Metadata obtido com sucesso");
                    m
                }
                Err(e) => {
                    warn!(path = %entry_path.display(), error = %e, "Erro ao obter metadata");
                    stats.errors_count += 1;
                    continue;
                }
            };

            if metadata.is_dir() {
                debug!(dir = %entry_path.display(), "Processando diretório");
                // Processar subdiretório
                dir_stats.subdirectory_count += 1;
                stats.directories_scanned += 1;

                if self.config.recursive {
                    debug!(dir = %entry_path.display(), "Iniciando scan recursivo");
                    let sub_stats = self.scan_directory(&entry_path, depth + 1, stats).await?;
                    dir_stats.total_files += sub_stats.total_files;
                    dir_stats.total_size += sub_stats.total_size;
                    debug!(dir = %entry_path.display(), "Scan recursivo concluído");
                }
            } else if metadata.is_file() {
                debug!(file = %entry_path.display(), size = metadata.len(), "Processando arquivo");
                
                // Processar arquivo
                if self.should_scan_file(&entry_path, &metadata)? {
                    debug!(file = %entry_path.display(), "Arquivo aprovado para catalogação");
                    match self.catalog_file(&entry_path, &metadata, depth).await {
                        Ok(_) => {
                            debug!(file = %entry_path.display(), "Arquivo catalogado com sucesso");
                        }
                        Err(e) => {
                            tracing::error!(file = %entry_path.display(), error = %e, "Erro ao catalogar arquivo");
                            return Err(e);
                        }
                    }
                    
                    dir_stats.direct_files += 1;
                    dir_stats.total_files += 1;
                    dir_stats.total_size += metadata.len() as i64;
                    stats.files_scanned += 1;
                    stats.total_size += metadata.len() as i64;

                    // Contar tipo de arquivo
                    if let Some(ext) = entry_path.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        *dir_stats.file_types.entry(ext_str).or_insert(0) += 1;
                    }
                    
                    debug!(file = %entry_path.display(), "Arquivo processado completamente");
                } else {
                    debug!(file = %entry_path.display(), "Arquivo rejeitado pelos filtros");
                }
            }
            
            processed_count += 1;
            debug!(count = processed_count, "Entry processado");
        }

        // Salvar estatísticas do diretório
        self.save_directory_stats(&dir_stats, depth).await?;

        Ok(dir_stats)
    }

    /// Verifica se um arquivo deve ser varrido
    fn should_scan_file(&self, _path: &Path, metadata: &Metadata) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let file_size = metadata.len() as i64;

        // Verificar tamanho mínimo
        if let Some(min_size) = self.config.min_file_size {
            if file_size < min_size {
                return Ok(false);
            }
        }

        // Verificar tamanho máximo
        if let Some(max_size) = self.config.max_file_size {
            if file_size > max_size {
                return Ok(false);
            }
        }

        // TODO: Implementar include/exclude patterns com glob

        Ok(true)
    }

    /// Cataloga um arquivo no banco
    async fn catalog_file(
        &self,
        path: &Path,
        metadata: &Metadata,
        depth: i32,
    ) -> Result<Uuid, Box<dyn std::error::Error + Send + Sync>> {
        debug!(path = %path.display(), "Iniciando catalogação de arquivo");
        
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let extension = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        let parent_directory = path.parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());

        let file_size = metadata.len() as i64;
        debug!(path = %path.display(), size = file_size, "Iniciando cálculo de hash");

        // Sempre calcular hash para detecção de duplicados e integridade
        let content_hash = Some(self.calculate_file_hash(path).await?);
        debug!(path = %path.display(), "Hash calculado, inserindo no banco");

        // Converter timestamps
        let modified_at = metadata.modified()
            .ok()
            .and_then(|t| system_time_to_datetime(t));

        let accessed_at = metadata.accessed()
            .ok()
            .and_then(|t| system_time_to_datetime(t));

        let created_at = metadata.created()
            .ok()
            .and_then(|t| system_time_to_datetime(t));

        // Verificar se arquivo já existe
        let existing_file = sqlx::query!(
            r#"
            SELECT id, file_size, content_hash, modified_at, accessed_at
            FROM file_catalog
            WHERE file_path = $1
            "#,
            path.to_string_lossy().to_string()
        )
        .fetch_optional(&self.pool)
        .await?;

        let id = if let Some(existing) = existing_file {
            // Arquivo já existe - registrar no histórico
            
            // Verificar o que mudou
            let size_changed = existing.file_size != file_size;
            let hash_changed = existing.content_hash != content_hash;
            let modified_changed = existing.modified_at != modified_at;
            let accessed_changed = existing.accessed_at != accessed_at;
            
            if size_changed || hash_changed || modified_changed || accessed_changed {
                // Inserir no histórico
                sqlx::query!(
                    r#"
                    INSERT INTO file_history (
                        file_catalog_id, scan_job_id, file_size, content_hash,
                        modified_at, accessed_at, size_changed, hash_changed,
                        modified_changed, accessed_changed, size_delta,
                        days_since_last_access, days_since_last_modification,
                        scan_type
                    ) VALUES (
                        $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                        CASE WHEN $12::TIMESTAMP IS NOT NULL THEN EXTRACT(DAY FROM (CURRENT_TIMESTAMP - $12::TIMESTAMP))::INTEGER ELSE NULL END,
                        CASE WHEN $13::TIMESTAMP IS NOT NULL THEN EXTRACT(DAY FROM (CURRENT_TIMESTAMP - $13::TIMESTAMP))::INTEGER ELSE NULL END,
                        'manual'
                    )
                    "#,
                    existing.id,
                    self.scan_job_id.unwrap_or_default(),
                    file_size,
                    content_hash.clone(),
                    modified_at,
                    accessed_at,
                    size_changed,
                    hash_changed,
                    modified_changed,
                    accessed_changed,
                    file_size - existing.file_size,
                    accessed_at,
                    modified_at
                )
                .execute(&self.pool)
                .await?;
                
                // Atualizar file_catalog
                sqlx::query!(
                    r#"
                    UPDATE file_catalog SET
                        file_size = $2,
                        content_hash = $3,
                        modified_at = $4,
                        accessed_at = $5,
                        last_scan_at = CURRENT_TIMESTAMP,
                        is_active = TRUE
                    WHERE id = $1
                    "#,
                    existing.id,
                    file_size,
                    content_hash,
                    modified_at,
                    accessed_at
                )
                .execute(&self.pool)
                .await?;
            } else {
                // Nada mudou, apenas atualizar last_scan_at
                sqlx::query!(
                    "UPDATE file_catalog SET last_scan_at = CURRENT_TIMESTAMP WHERE id = $1",
                    existing.id
                )
                .execute(&self.pool)
                .await?;
            }
            
            existing.id
        } else {
            // Novo arquivo - inserir no catálogo
            let new_id = sqlx::query_scalar!(
                r#"
                INSERT INTO file_catalog (
                    file_path, file_name, extension, file_size,
                    created_at, modified_at, accessed_at,
                    content_hash, parent_directory, depth
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                RETURNING id
                "#,
                path.to_string_lossy().to_string(),
                file_name,
                extension,
                file_size,
                created_at,
                modified_at,
                accessed_at,
                content_hash.clone(),
                parent_directory,
                depth
            )
            .fetch_one(&self.pool)
            .await?;
            
            // Inserir primeira entrada no histórico
            sqlx::query!(
                r#"
                INSERT INTO file_history (
                    file_catalog_id, scan_job_id, file_size, content_hash,
                    modified_at, accessed_at, scan_type
                ) VALUES ($1, $2, $3, $4, $5, $6, 'initial')
                "#,
                new_id,
                self.scan_job_id.unwrap_or_default(),
                file_size,
                content_hash,
                modified_at,
                accessed_at
            )
            .execute(&self.pool)
            .await?;
            
            new_id
        };

        debug!(file = %path.display(), id = %id, "Arquivo catalogado");

        Ok(id)
    }

    /// Calcula o hash SHA256 de um arquivo
    async fn calculate_file_hash(&self, path: &Path) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        use tokio::io::{AsyncReadExt, BufReader};
        
        debug!(path = %path.display(), "Calculando hash do arquivo");
        
        let file = fs::File::open(path).await?;
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192]; // Buffer de 8KB
        
        loop {
            let bytes_read = reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        
        let hash_result = format!("{:x}", hasher.finalize());
        debug!(path = %path.display(), hash = %hash_result, "Hash calculado com sucesso");
        
        Ok(hash_result)
    }

    /// Salva estatísticas de um diretório
    async fn save_directory_stats(&self, stats: &DirectoryStats, depth: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query!(
            r#"
            INSERT INTO directory_catalog (
                directory_path, directory_name, depth, total_files, direct_files,
                total_size, subdirectory_count, file_types
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (directory_path) DO UPDATE SET
                depth = EXCLUDED.depth,
                total_files = EXCLUDED.total_files,
                direct_files = EXCLUDED.direct_files,
                total_size = EXCLUDED.total_size,
                subdirectory_count = EXCLUDED.subdirectory_count,
                file_types = EXCLUDED.file_types,
                last_scan_at = CURRENT_TIMESTAMP
            "#,
            stats.path,
            Path::new(&stats.path).file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("/"),
            depth,
            stats.total_files,
            stats.direct_files,
            stats.total_size,
            stats.subdirectory_count,
            serde_json::to_value(&stats.file_types)?
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Cria um job de varredura no banco
    async fn create_scan_job(&self) -> Result<Uuid, Box<dyn std::error::Error + Send + Sync>> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO scan_jobs (
                root_path, recursive, follow_symlinks, max_depth,
                include_patterns, exclude_patterns, min_file_size, max_file_size
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
            self.config.root_path.to_string_lossy().to_string(),
            self.config.recursive,
            self.config.follow_symlinks,
            self.config.max_depth,
            &self.config.include_patterns,
            &self.config.exclude_patterns,
            self.config.min_file_size,
            self.config.max_file_size
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }
}

/// Estatísticas da varredura
#[derive(Default)]
struct ScanStats {
    files_scanned: i64,
    directories_scanned: i64,
    total_size: i64,
    errors_count: i32,
}

/// Converte SystemTime para NaiveDateTime
fn system_time_to_datetime(time: SystemTime) -> Option<chrono::NaiveDateTime> {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .and_then(|d| {
            let timestamp = d.as_secs() as i64;
            let nanos = d.subsec_nanos();
            chrono::DateTime::from_timestamp(timestamp, nanos).map(|dt| dt.naive_utc())
        })
}

/// Busca arquivos no catálogo
pub async fn search_files(
    _pool: &PgPool,
    query: Option<String>,
    extension: Option<String>,
    min_size: Option<i64>,
    max_size: Option<i64>,
    limit: i64,
) -> Result<Vec<CatalogedFile>, sqlx::Error> {
    let mut sql = String::from(
        "SELECT * FROM file_catalog WHERE is_active = TRUE"
    );

    if let Some(q) = query {
        sql.push_str(&format!(" AND file_name ILIKE '%{}%'", q));
    }

    if let Some(ext) = extension {
        sql.push_str(&format!(" AND extension = '{}'", ext));
    }

    if let Some(min) = min_size {
        sql.push_str(&format!(" AND file_size >= {}", min));
    }

    if let Some(max) = max_size {
        sql.push_str(&format!(" AND file_size <= {}", max));
    }

    sql.push_str(&format!(" ORDER BY modified_at DESC LIMIT {}", limit));

    // TODO: Usar query builder ou prepared statements adequados
    // Esta é uma versão simplificada para demonstração

    Ok(vec![])
}