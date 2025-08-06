use crate::AppError;
use crate::models::{BackupJob, NewBackupExecutionLog};
use crate::{db, rclone::RcloneWrapper};
use crate::file_scanner::{FileScanner, ScanConfig};
use sqlx::PgPool;
use std::path::PathBuf;
use uuid::Uuid;

/// Executa um backup job manualmente (sem schedule).
/// 
/// Wrapper para `perform_backup_with_schedule` quando o backup
/// é executado manualmente via API ou interface.
/// 
/// # Argumentos
/// * `pool` - Pool de conexão PostgreSQL
/// * `job` - Backup job a ser executado
/// 
/// # Retorna
/// * `Ok(())` - Backup executado com sucesso
/// * `Err(AppError)` - Falha na execução
/// 
/// # Exemplos
/// ```no_run
/// let result = perform_backup(&pool, &job).await;
/// ```
pub async fn perform_backup(pool: &PgPool, job: &BackupJob) -> Result<(), AppError> {
    perform_backup_with_schedule(pool, job, None).await
}

/// Executa um backup job com suporte a agendamento.
/// 
/// Esta é a função principal de execução de backup que:
/// 1. Atualiza status do job para RUNNING
/// 2. Parsea os mapeamentos origem -> destinos
/// 3. Executa rclone sync para cada mapeamento
/// 4. Cria logs de execução detalhados
/// 5. Atualiza status final e próxima execução do schedule
/// 
/// # Argumentos
/// * `pool` - Pool de conexão PostgreSQL
/// * `job` - Backup job a ser executado
/// * `schedule_id` - ID do schedule que triggou a execução (opcional)
/// 
/// # Retorna
/// * `Ok(())` - Backup executado com sucesso
/// * `Err(AppError)` - Falha na execução
/// 
/// # Comportamento
/// - Se qualquer transferência falhar, marca job como FAILED
/// - Atualiza last_run e next_run do schedule automaticamente
/// - Salva métricas detalhadas no backup_execution_logs
/// - Usa rclone com logs estruturados para debugging
/// 
/// # Exemplos
/// ```no_run
/// // Backup manual
/// let result = perform_backup_with_schedule(&pool, &job, None).await;
/// 
/// // Backup via scheduler
/// let result = perform_backup_with_schedule(&pool, &job, Some(schedule_id)).await;
/// ```
pub async fn perform_backup_with_schedule(pool: &PgPool, job: &BackupJob, schedule_id: Option<Uuid>) -> Result<(), AppError> {
    tracing::debug!(job_id = %job.id, job_name = %job.name, "Starting backup job");
    
    // Update job status to RUNNING
    db::update_backup_job_status(pool, job.id, "RUNNING").await?;

    let mappings: std::collections::HashMap<String, Vec<String>> = serde_json::from_value(job.mappings.clone())?;
    let rclone = RcloneWrapper::new(Default::default(), Some(PathBuf::from("./logs")));
    
    let mut all_success = true;
    let mut scan_job_ids = Vec::new();

    for (source_path, destination_paths) in mappings {
        // NOVO: Escanear origem ANTES do backup para catalogar arquivos
        tracing::info!(
            job_id = %job.id,
            source = %source_path,
            "Catalogando arquivos antes do backup"
        );
        
        let scan_config = ScanConfig {
            root_path: PathBuf::from(&source_path),
            recursive: true,
            ..Default::default()
        };
        
        let mut scanner = FileScanner::new(pool.clone(), scan_config);
        
        // Executar scan e aguardar conclusão
        match scanner.start_scan().await {
            Ok(scan_job_id) => {
                tracing::info!(
                    job_id = %job.id,
                    scan_job_id = %scan_job_id,
                    "Catalogação concluída com sucesso"
                );
                scan_job_ids.push(scan_job_id);
                
                // Atualizar scan_job com referência ao backup
                sqlx::query!(
                    "UPDATE scan_jobs SET backup_job_id = $1, scan_type = 'backup_pre' WHERE id = $2",
                    job.id,
                    scan_job_id
                )
                .execute(pool)
                .await?;
            }
            Err(e) => {
                tracing::warn!(
                    job_id = %job.id,
                    error = %e,
                    "Falha na catalogação, continuando com backup"
                );
            }
        }
        for destination in destination_paths {
            // Criar log de execução
            let triggered_by = if schedule_id.is_some() { "scheduler" } else { "manual" };
            let log_data = NewBackupExecutionLog {
                backup_job_id: job.id,
                schedule_id,
                rclone_command: format!("rclone sync {:?} {:?}", source_path, destination),
                source_path: source_path.clone(),
                destination_path: destination.clone(),
                rclone_config: None,
                triggered_by: Some(triggered_by.to_string()),
            };

            let execution_log = db::create_backup_execution_log(pool, &log_data).await?;
            
            // Executar rclone sync
            match rclone.sync(execution_log.id, &source_path, &destination).await {
                Ok(result) => {
                    // Atualizar log com resultados
                    db::update_backup_execution_log_completion(pool, execution_log.id, &result).await?;
                    tracing::debug!(
                        job_id = %job.id,
                        files_transferred = result.files_transferred,
                        "Backup completed for path {} -> {}", source_path, destination
                    );
                    
                    // NOVO: Marcar arquivos como backupeados
                    if !scan_job_ids.is_empty() {
                        let update_result = sqlx::query!(
                            r#"
                            UPDATE file_catalog 
                            SET 
                                last_backup_at = CURRENT_TIMESTAMP,
                                backup_count = backup_count + 1,
                                backup_job_ids = array_append(backup_job_ids, $1)
                            WHERE file_path LIKE $2 || '%'
                              AND is_active = true
                            "#,
                            job.id,
                            source_path
                        )
                        .execute(pool)
                        .await;
                        
                        if let Err(e) = update_result {
                            tracing::warn!(
                                job_id = %job.id,
                                error = %e,
                                "Falha ao marcar arquivos como backupeados"
                            );
                        } else {
                            tracing::info!(
                                job_id = %job.id,
                                "Arquivos marcados como backupeados"
                            );
                        }
                    }
                }
                Err(e) => {
                    all_success = false;
                    tracing::error!(
                        job_id = %job.id,
                        error = %e,
                        "Backup failed for path {} -> {}", source_path, destination
                    );
                }
            }
        }
    }

    // Update job status based on result
    let final_status = if all_success { "COMPLETED" } else { "FAILED" };
    db::update_backup_job_status(pool, job.id, final_status).await?;
    
    if all_success {
        tracing::debug!(job_id = %job.id, "Backup job completed successfully");
        Ok(())
    } else {
        tracing::error!(job_id = %job.id, "Some backup operations failed");
        Err(AppError::InternalServerError("Some backup operations failed".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;
    use std::collections::HashMap;

    fn create_test_job() -> BackupJob {
        BackupJob {
            id: Uuid::new_v4(),
            name: "Test Job".to_string(),
            mappings: json!({
                "/tmp/source": ["/tmp/dest1", "/tmp/dest2"],
                "/home/docs": ["/backup/docs"]
            }),
            status: "PENDING".to_string(),
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deleted_at: None,
        }
    }

    #[test]
    fn test_backup_job_creation() {
        let job = create_test_job();
        assert_eq!(job.name, "Test Job");
        assert_eq!(job.status, "PENDING");
        assert!(job.is_active);
        assert!(job.deleted_at.is_none());
    }

    #[test]
    fn test_mappings_parsing() {
        let job = create_test_job();
        let mappings: HashMap<String, Vec<String>> = 
            serde_json::from_value(job.mappings).unwrap();
        
        // Verificar se tem as chaves esperadas
        assert!(mappings.contains_key("/tmp/source"));
        assert!(mappings.contains_key("/home/docs"));
        
        // Verificar múltiplos destinos
        assert_eq!(mappings["/tmp/source"], vec!["/tmp/dest1", "/tmp/dest2"]);
        assert_eq!(mappings["/home/docs"], vec!["/backup/docs"]);
    }

    #[test]
    fn test_invalid_mappings() {
        let invalid_mappings = json!({
            "source": "not_an_array"  // Deve ser array
        });
        
        let result: Result<HashMap<String, Vec<String>>, _> = 
            serde_json::from_value(invalid_mappings);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_mappings() {
        let empty_mappings = json!({});
        let mappings: HashMap<String, Vec<String>> = 
            serde_json::from_value(empty_mappings).unwrap();
        
        assert!(mappings.is_empty());
    }

    #[test]
    fn test_job_status_transitions() {
        let mut job = create_test_job();
        
        assert_eq!(job.status, "PENDING");
        
        job.status = "RUNNING".to_string();
        assert_eq!(job.status, "RUNNING");
        
        job.status = "COMPLETED".to_string();
        assert_eq!(job.status, "COMPLETED");
        
        job.status = "FAILED".to_string();
        assert_eq!(job.status, "FAILED");
    }

    #[test]
    fn test_job_soft_delete() {
        let mut job = create_test_job();
        
        assert!(job.is_active);
        assert!(job.deleted_at.is_none());
        
        // Simular soft delete
        job.is_active = false;
        job.deleted_at = Some(chrono::Utc::now());
        
        assert!(!job.is_active);
        assert!(job.deleted_at.is_some());
    }
}