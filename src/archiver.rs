// src/archiver.rs
// Sistema de arquivamento inteligente para logs de backup

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use sqlx::PgPool;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, warn};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArchivePolicy {
    /// Minutos para manter no banco (default: 43200 = 30 dias)
    pub hot_retention_minutes: i32,
    /// Meses para manter em Parquet (default: 24)  
    pub warm_retention_months: i32,
    /// Arquivamento automático ativo
    pub auto_archive_enabled: bool,
    /// Tamanho em GB para comprimir (default: 1.0)
    pub compress_threshold_gb: f64,
    /// Intervalo em minutos para executar arquivamento automático (default: 60)
    pub auto_archive_interval_minutes: i32,
}

impl Default for ArchivePolicy {
    fn default() -> Self {
        Self {
            hot_retention_minutes: 43200, // 30 dias em minutos
            warm_retention_months: 24,
            auto_archive_enabled: true,
            compress_threshold_gb: 1.0,
            auto_archive_interval_minutes: 60,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ArchiveStatus {
    pub hot_records: i64,              // Registros no banco
    pub warm_files: Vec<WarmFileInfo>, // Arquivos Parquet
    pub cold_files: Vec<ColdFileInfo>, // Arquivos comprimidos
    pub total_size_gb: f64,           // Tamanho total
    pub last_archive_run: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WarmFileInfo {
    pub file_path: String,
    pub month: String,                // "2025-01"
    pub record_count: i64,
    pub size_mb: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ColdFileInfo {
    pub file_path: String,
    pub year: String,                 // "2024"
    pub compressed_size_mb: f64,
    pub original_size_mb: f64,
    pub compression_ratio: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ArchiveResult {
    pub archived_records: i64,
    pub created_files: Vec<String>,
    pub freed_space_mb: f64,
    pub duration_seconds: f64,
}

pub struct LogArchiver {
    pub db_pool: PgPool,
    pub archive_dir: PathBuf,
    pub policy: ArchivePolicy,
}

impl LogArchiver {
    pub fn new(db_pool: PgPool, archive_dir: PathBuf, policy: Option<ArchivePolicy>) -> Self {
        Self {
            db_pool,
            archive_dir,
            policy: policy.unwrap_or_default(),
        }
    }

    /// Executa arquivamento automático baseado na política
    pub async fn run_auto_archive(&self) -> Result<ArchiveResult> {
        if !self.policy.auto_archive_enabled {
            return Err(anyhow!("Auto archive is disabled"));
        }

        info!("Starting automatic log archiving");
        self.archive_to_warm().await
    }

    /// Força arquivamento manual (API endpoint)
    pub async fn force_archive_to_warm(&self) -> Result<ArchiveResult> {
        info!("Starting manual archive to warm storage");
        self.archive_to_warm().await
    }

    /// Força compressão manual para cold storage
    pub async fn force_compress_to_cold(&self) -> Result<ArchiveResult> {
        info!("Starting manual compression to cold storage");
        self.compress_to_cold().await
    }

    /// Move logs antigos do banco para arquivos Parquet
    async fn archive_to_warm(&self) -> Result<ArchiveResult> {
        let start_time = std::time::Instant::now();
        let cutoff_date = Utc::now() - Duration::minutes(self.policy.hot_retention_minutes.into());
        
        // 1. Buscar logs antigos
        let old_logs = self.get_logs_older_than(&cutoff_date).await?;
        
        if old_logs.is_empty() {
            info!("No logs to archive");
            return Ok(ArchiveResult {
                archived_records: 0,
                created_files: vec![],
                freed_space_mb: 0.0,
                duration_seconds: start_time.elapsed().as_secs_f64(),
            });
        }

        // 2. Agrupar por mês
        let grouped_logs = self.group_logs_by_month(&old_logs);
        let mut created_files = Vec::new();
        let mut total_archived = 0i64;

        // 3. Criar arquivos Parquet por mês
        for (month, logs) in grouped_logs {
            let file_path = self.create_warm_file_path(&month);
            
            // Criar diretório se não existir
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            // Exportar para Parquet (simulado por enquanto)
            let record_count = self.export_logs_to_parquet(&logs, &file_path).await?;
            
            info!(
                month = %month,
                records = record_count,
                file = ?file_path,
                "Created warm archive file"
            );
            
            created_files.push(file_path.to_string_lossy().to_string());
            total_archived += record_count;
        }

        // 4. Deletar logs do banco após confirmação
        let deleted_count = self.delete_archived_logs(&cutoff_date).await?;
        
        // 5. Calcular espaço liberado (estimativa)
        let freed_space_mb = (deleted_count as f64) * 0.001; // ~1KB por log

        Ok(ArchiveResult {
            archived_records: total_archived,
            created_files,
            freed_space_mb,
            duration_seconds: start_time.elapsed().as_secs_f64(),
        })
    }

    /// Comprime arquivos Parquet antigos para cold storage
    async fn compress_to_cold(&self) -> Result<ArchiveResult> {
        let start_time = std::time::Instant::now();
        let cutoff_date = Utc::now() - Duration::days((self.policy.warm_retention_months * 30).into());
        
        // Encontrar arquivos Parquet antigos
        let old_parquet_files = self.find_old_parquet_files(&cutoff_date).await?;
        
        if old_parquet_files.is_empty() {
            info!("No warm files to compress");
            return Ok(ArchiveResult {
                archived_records: 0,
                created_files: vec![],
                freed_space_mb: 0.0,
                duration_seconds: start_time.elapsed().as_secs_f64(),
            });
        }

        let mut compressed_files = Vec::new();
        let mut total_freed_space = 0.0;

        // Agrupar por ano e comprimir
        let grouped_by_year = self.group_parquet_files_by_year(&old_parquet_files);
        
        for (year, files) in grouped_by_year {
            let compressed_file = self.create_cold_file_path(&year);
            
            // Comprimir arquivos (tar.gz)
            let (original_size, compressed_size) = self.compress_files_to_archive(&files, &compressed_file).await?;
            
            info!(
                year = %year,
                files_count = files.len(),
                original_mb = original_size,
                compressed_mb = compressed_size,
                ratio = compressed_size / original_size,
                "Created cold archive"
            );

            // Deletar arquivos originais após compressão
            for file in &files {
                if let Err(e) = fs::remove_file(file).await {
                    warn!("Failed to delete original file {:?}: {}", file, e);
                }
            }

            compressed_files.push(compressed_file.to_string_lossy().to_string());
            total_freed_space += original_size - compressed_size;
        }

        Ok(ArchiveResult {
            archived_records: 0, // Não conta registros na compressão
            created_files: compressed_files,
            freed_space_mb: total_freed_space,
            duration_seconds: start_time.elapsed().as_secs_f64(),
        })
    }

    /// Obtém status atual do arquivamento
    pub async fn get_archive_status(&self) -> Result<ArchiveStatus> {
        // Contar registros no banco
        let hot_records = self.count_hot_records().await?;
        
        // Listar arquivos warm
        let warm_files = self.list_warm_files().await?;
        
        // Listar arquivos cold
        let cold_files = self.list_cold_files().await?;
        
        // Calcular tamanho total
        let total_size_gb = self.calculate_total_size(&warm_files, &cold_files).await?;

        Ok(ArchiveStatus {
            hot_records,
            warm_files,
            cold_files,
            total_size_gb,
            last_archive_run: None, // TODO: implementar tracking
        })
    }

    // === Métodos auxiliares ===

    async fn get_logs_older_than(&self, cutoff_date: &DateTime<Utc>) -> Result<Vec<crate::models::BackupExecutionLog>> {
        let logs = sqlx::query_as!(
            crate::models::BackupExecutionLog,
            r#"
            SELECT id, backup_job_id, schedule_id, started_at, completed_at, status,
                   rclone_command, source_path, destination_path, rclone_config,
                   files_transferred, files_checked, files_deleted, bytes_transferred,
                   transfer_rate_mbps, duration_seconds, error_count, retry_count,
                   error_message, rclone_stdout, rclone_stderr, rclone_log_file_path,
                   triggered_by, created_at, updated_at
            FROM backup_execution_logs
            WHERE created_at < $1
            ORDER BY created_at ASC
            "#,
            cutoff_date
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(logs)
    }

    fn group_logs_by_month<'a>(&self, logs: &'a [crate::models::BackupExecutionLog]) -> std::collections::HashMap<String, Vec<&'a crate::models::BackupExecutionLog>> {
        let mut grouped = std::collections::HashMap::new();
        
        for log in logs {
            let month_key = log.created_at.format("%Y-%m").to_string();
            grouped.entry(month_key).or_insert_with(Vec::new).push(log);
        }
        
        grouped
    }

    fn create_warm_file_path(&self, month: &str) -> PathBuf {
        let year = &month[..4];
        self.archive_dir
            .join("warm")
            .join(year)
            .join(format!("backup_logs_{}.json.gz", month))
    }

    fn create_cold_file_path(&self, year: &str) -> PathBuf {
        self.archive_dir
            .join("cold")
            .join(format!("backup_logs_{}.tar.gz", year))
    }

    async fn export_logs_to_parquet(&self, logs: &[&crate::models::BackupExecutionLog], file_path: &Path) -> Result<i64> {
        if logs.is_empty() {
            return Ok(0);
        }

        let record_count = logs.len();
        
        // Serializar para JSON
        let json_data = serde_json::to_string_pretty(logs)?;
        
        // Comprimir com gzip
        let file = std::fs::File::create(file_path)?;
        let mut encoder = GzEncoder::new(file, Compression::best());
        encoder.write_all(json_data.as_bytes())?;
        encoder.finish()?;

        let file_size = fs::metadata(file_path).await?.len();
        
        info!(
            records = record_count,
            file = ?file_path,
            size_kb = file_size / 1024,
            compression_ratio = (json_data.len() as f64 / file_size as f64),
            "Exported logs to compressed JSON file"
        );
        
        Ok(record_count as i64)
    }

    async fn delete_archived_logs(&self, cutoff_date: &DateTime<Utc>) -> Result<i64> {
        let result = sqlx::query!(
            "DELETE FROM backup_execution_logs WHERE created_at < $1",
            cutoff_date
        )
        .execute(&self.db_pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    async fn find_old_parquet_files(&self, _cutoff_date: &DateTime<Utc>) -> Result<Vec<PathBuf>> {
        // TODO: Implementar busca de arquivos Parquet antigos
        Ok(vec![])
    }

    fn group_parquet_files_by_year(&self, _files: &[PathBuf]) -> std::collections::HashMap<String, Vec<PathBuf>> {
        // TODO: Implementar agrupamento por ano
        std::collections::HashMap::new()
    }

    async fn compress_files_to_archive(&self, _files: &[PathBuf], _compressed_file: &Path) -> Result<(f64, f64)> {
        // TODO: Implementar compressão real com tar.gz
        Ok((100.0, 30.0)) // Simulado: 100MB → 30MB
    }

    async fn count_hot_records(&self) -> Result<i64> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM backup_execution_logs"
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(count.unwrap_or(0))
    }

    async fn list_warm_files(&self) -> Result<Vec<WarmFileInfo>> {
        // TODO: Implementar listagem real de arquivos warm
        Ok(vec![])
    }

    async fn list_cold_files(&self) -> Result<Vec<ColdFileInfo>> {
        // TODO: Implementar listagem real de arquivos cold
        Ok(vec![])
    }

    async fn calculate_total_size(&self, warm_files: &[WarmFileInfo], cold_files: &[ColdFileInfo]) -> Result<f64> {
        let warm_size: f64 = warm_files.iter().map(|f| f.size_mb).sum();
        let cold_size: f64 = cold_files.iter().map(|f| f.compressed_size_mb).sum();
        Ok((warm_size + cold_size) / 1024.0) // Converter MB para GB
    }
}