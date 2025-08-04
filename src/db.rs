use crate::models::{
    BackupJob, NewBackupJob, BackupSchedule, NewBackupSchedule, UpdateBackupJob, UpdateBackupSchedule,
    CloudProvider, NewCloudProvider, UpdateCloudProvider, CloudProviderType, ConnectivityTestResult, ConnectivityStatus
};
use sqlx::PgPool;
use chrono::{DateTime, Utc, Datelike, Timelike, Duration};

/// Calcula a próxima execução baseada na cron expression.
/// 
/// Implementação simplificada para casos comuns do sistema B2CLI.
/// Suporta expressões no formato de 6 campos: "sec min hour day month day_of_week".
/// 
/// # Argumentos
/// * `cron_expr` - String com cron expression no formato "sec min hour day month dow"
/// 
/// # Retorna
/// * `Some(DateTime<Utc>)` - Próxima execução calculada com sucesso
/// * `None` - Se a expressão for inválida ou não puder ser parseada
/// 
/// # Formatos suportados
/// - `*` - Qualquer valor (para minuto, hora, etc.)
/// - Números específicos - `0`, `10`, `15`, etc.
/// - Dia da semana: `0` = domingo, `1` = segunda, ..., `6` = sábado
/// 
/// # Exemplos
/// ```
/// use chrono::{DateTime, Utc, Datelike};
/// // Todo domingo às 10h
/// let next = calculate_next_run("0 0 10 * * 0");
/// assert!(next.is_some());
/// 
/// // Todo dia às 15h30
/// let next = calculate_next_run("0 30 15 * * *");
/// assert!(next.is_some());
/// 
/// // Expressão inválida
/// let next = calculate_next_run("invalid");
/// assert!(next.is_none());
/// ```
fn calculate_next_run(cron_expr: &str) -> Option<DateTime<Utc>> {
    let parts: Vec<&str> = cron_expr.split_whitespace().collect();
    
    // Esperamos formato: "sec min hour day month day_of_week" (6 campos)
    if parts.len() != 6 {
        return None;
    }
    
    let _second = parts[0];
    let minute = parts[1];
    let hour = parts[2];
    let _day = parts[3];
    let _month = parts[4];
    let day_of_week = parts[5];
    
    // Parse simples para casos comuns
    let target_minute = if minute == "*" { 0 } else { minute.parse::<u32>().ok()? };
    let target_hour = if hour == "*" { 0 } else { hour.parse::<u32>().ok()? };
    let target_dow = if day_of_week == "*" { 
        None 
    } else { 
        Some(day_of_week.parse::<u32>().ok()?) 
    };
    
    let now = Utc::now();
    let mut next_run = now.with_minute(target_minute)?.with_second(0)?.with_nanosecond(0)?;
    
    // Ajustar hora se necessário
    if hour != "*" {
        next_run = next_run.with_hour(target_hour)?;
    }
    
    // Se a próxima execução é no passado, adicionar tempo
    if next_run <= now {
        if let Some(dow) = target_dow {
            // Encontrar próximo dia da semana (0 = domingo)
            let current_dow = now.weekday().num_days_from_sunday();
            let days_ahead = if dow <= current_dow {
                7 - (current_dow - dow)
            } else {
                dow - current_dow
            };
            next_run = next_run + Duration::days(days_ahead as i64);
        } else {
            // Caso simples: próximo dia
            next_run = next_run + Duration::days(1);
        }
    }
    
    Some(next_run)
}

/// Cria um novo backup job no banco de dados.
/// 
/// Se o job incluir dados de agendamento, também cria o schedule associado
/// e calcula automaticamente a próxima execução (`next_run`).
/// 
/// # Argumentos
/// * `pool` - Pool de conexão com PostgreSQL
/// * `new_job` - Dados do novo backup job
/// 
/// # Retorna
/// * `Ok((BackupJob, Some(BackupSchedule)))` - Job e schedule criados com sucesso
/// * `Ok((BackupJob, None))` - Job criado sem schedule
/// * `Err(sqlx::Error)` - Erro de banco de dados
/// 
/// # Exemplos
/// ```no_run
/// use sqlx::PgPool;
/// use std::collections::HashMap;
/// 
/// let mut mappings = HashMap::new();
/// mappings.insert("/home/docs".to_string(), vec!["/backup/docs".to_string()]);
/// 
/// let new_job = NewBackupJob {
///     name: "Backup Documents".to_string(),
///     mappings,
///     schedule: None,
/// };
/// 
/// let (job, schedule) = create_backup_job(&pool, &new_job).await?;
/// ```
pub async fn create_backup_job(pool: &PgPool, new_job: &NewBackupJob) -> Result<(BackupJob, Option<BackupSchedule>), sqlx::Error> {
    let job = sqlx::query_as!(
        BackupJob,
        r#"
        INSERT INTO backup_jobs (name, mappings)
        VALUES ($1, $2)
        RETURNING id, name, mappings, created_at, updated_at, deleted_at, status, is_active
        "#,
        new_job.name,
        serde_json::to_value(&new_job.mappings).unwrap()
    )
    .fetch_one(pool)
    .await?;

    if let Some(schedule_data) = &new_job.schedule {
        let schedule = create_backup_schedule(pool, job.id, schedule_data).await?;
        Ok((job, Some(schedule)))
    } else {
        Ok((job, None))
    }
}

pub async fn update_backup_job_status(pool: &PgPool, id: uuid::Uuid, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE backup_jobs
        SET status = $1, updated_at = NOW()
        WHERE id = $2
        "#,
        status,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_backup_jobs(pool: &PgPool) -> Result<Vec<BackupJob>, sqlx::Error> {
    let jobs = sqlx::query_as!(
        BackupJob,
        r#"
        SELECT id, name, mappings, created_at, updated_at, deleted_at, status, is_active
        FROM backup_jobs
        WHERE is_active = true
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(jobs)
}

pub async fn get_backup_job_by_id(pool: &PgPool, id: uuid::Uuid) -> Result<Option<BackupJob>, sqlx::Error> {
    let job = sqlx::query_as!(
        BackupJob,
        r#"
        SELECT id, name, mappings, created_at, updated_at, deleted_at, status, is_active
        FROM backup_jobs
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(job)
}

pub async fn update_backup_job(pool: &PgPool, id: uuid::Uuid, updated_job: &NewBackupJob) -> Result<Option<BackupJob>, sqlx::Error> {
    let job = sqlx::query_as!(
        BackupJob,
        r#"
        UPDATE backup_jobs
        SET name = $1, mappings = $2, updated_at = NOW()
        WHERE id = $3 AND is_active = true
        RETURNING id, name, mappings, created_at, updated_at, deleted_at, status, is_active
        "#,
        updated_job.name,
        serde_json::to_value(&updated_job.mappings).unwrap(),
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(job)
}

pub async fn delete_backup_job(pool: &PgPool, id: uuid::Uuid) -> Result<u64, sqlx::Error> {
    let rows_affected = sqlx::query!(
        r#"
        UPDATE backup_jobs
        SET deleted_at = NOW(), updated_at = NOW(), is_active = false
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected)
}

pub async fn get_postgres_version(pool: &PgPool) -> Result<String, sqlx::Error> {
    let row: (String,) = sqlx::query_as("SHOW server_version").fetch_one(pool).await?;
    Ok(row.0)
}

// Backup Schedule functions
pub async fn create_backup_schedule(pool: &PgPool, backup_job_id: uuid::Uuid, new_schedule: &NewBackupSchedule) -> Result<BackupSchedule, sqlx::Error> {
    // Calcular próxima execução
    let next_run = calculate_next_run(&new_schedule.cron_expression);
    
    let schedule = sqlx::query!(
        r#"
        INSERT INTO backup_schedules (backup_job_id, name, cron_expression, enabled, next_run)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, backup_job_id, name, cron_expression, enabled, next_run, last_run, last_status, created_at, updated_at
        "#,
        backup_job_id,
        new_schedule.name,
        new_schedule.cron_expression,
        new_schedule.enabled.unwrap_or(true),
        next_run.map(|dt| dt.naive_utc())
    )
    .fetch_one(pool)
    .await?;

    let schedule = BackupSchedule {
        id: schedule.id,
        backup_job_id: schedule.backup_job_id,
        name: schedule.name,
        cron_expression: schedule.cron_expression,
        enabled: schedule.enabled,
        next_run: schedule.next_run.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        last_run: schedule.last_run.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        last_status: schedule.last_status.unwrap_or_else(|| "pending".to_string()),
        created_at: DateTime::from_naive_utc_and_offset(schedule.created_at, Utc),
        updated_at: DateTime::from_naive_utc_and_offset(schedule.updated_at, Utc),
    };

    Ok(schedule)
}

pub async fn get_backup_schedule_by_job_id(pool: &PgPool, backup_job_id: uuid::Uuid) -> Result<Option<BackupSchedule>, sqlx::Error> {
    let schedule = sqlx::query!(
        r#"
        SELECT id, backup_job_id, name, cron_expression, enabled, next_run, last_run, last_status, created_at, updated_at
        FROM backup_schedules
        WHERE backup_job_id = $1
        "#,
        backup_job_id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(row) = schedule {
        Ok(Some(BackupSchedule {
            id: row.id,
            backup_job_id: row.backup_job_id,
            name: row.name,
            cron_expression: row.cron_expression,
            enabled: row.enabled,
            next_run: row.next_run.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            last_run: row.last_run.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            last_status: row.last_status.unwrap_or_else(|| "pending".to_string()),
            created_at: DateTime::from_naive_utc_and_offset(row.created_at, Utc),
            updated_at: DateTime::from_naive_utc_and_offset(row.updated_at, Utc),
        }))
    } else {
        Ok(None)
    }
}

pub async fn list_active_schedules(pool: &PgPool) -> Result<Vec<BackupSchedule>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT id, backup_job_id, name, cron_expression, enabled, next_run, last_run, last_status, created_at, updated_at
        FROM backup_schedules
        WHERE enabled = true
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    let schedules: Vec<BackupSchedule> = rows.into_iter().map(|row| {
        BackupSchedule {
            id: row.id,
            backup_job_id: row.backup_job_id,
            name: row.name,
            cron_expression: row.cron_expression,
            enabled: row.enabled,
            next_run: row.next_run.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            last_run: row.last_run.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            last_status: row.last_status.unwrap_or_else(|| "pending".to_string()),
            created_at: DateTime::from_naive_utc_and_offset(row.created_at, Utc),
            updated_at: DateTime::from_naive_utc_and_offset(row.updated_at, Utc),
        }
    }).collect();

    Ok(schedules)
}

pub async fn update_backup_schedule(pool: &PgPool, backup_job_id: uuid::Uuid, updated_schedule: &NewBackupSchedule) -> Result<Option<BackupSchedule>, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        UPDATE backup_schedules
        SET name = $1, cron_expression = $2, enabled = $3, updated_at = NOW()
        WHERE backup_job_id = $4
        RETURNING id, backup_job_id, name, cron_expression, enabled, next_run, last_run, last_status, created_at, updated_at
        "#,
        updated_schedule.name,
        updated_schedule.cron_expression,
        updated_schedule.enabled.unwrap_or(true),
        backup_job_id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        Ok(Some(BackupSchedule {
            id: row.id,
            backup_job_id: row.backup_job_id,
            name: row.name,
            cron_expression: row.cron_expression,
            enabled: row.enabled,
            next_run: row.next_run.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            last_run: row.last_run.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            last_status: row.last_status.unwrap_or_else(|| "pending".to_string()),
            created_at: DateTime::from_naive_utc_and_offset(row.created_at, Utc),
            updated_at: DateTime::from_naive_utc_and_offset(row.updated_at, Utc),
        }))
    } else {
        Ok(None)
    }
}

pub async fn delete_backup_schedule(pool: &PgPool, backup_job_id: uuid::Uuid) -> Result<u64, sqlx::Error> {
    let rows_affected = sqlx::query!(
        r#"
        DELETE FROM backup_schedules
        WHERE backup_job_id = $1
        "#,
        backup_job_id
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected)
}

pub async fn update_schedule_last_run(pool: &PgPool, schedule_id: uuid::Uuid, status: &str) -> Result<(), sqlx::Error> {
    // Primeiro, buscar a cron expression atual
    let schedule = sqlx::query!(
        "SELECT cron_expression FROM backup_schedules WHERE id = $1",
        schedule_id
    )
    .fetch_optional(pool)
    .await?;
    
    if let Some(row) = schedule {
        // Calcular próxima execução
        let next_run = calculate_next_run(&row.cron_expression);
        
        // Atualizar com last_run e next_run
        sqlx::query!(
            r#"
            UPDATE backup_schedules
            SET last_run = NOW(), 
                last_status = $1, 
                next_run = $2,
                updated_at = NOW()
            WHERE id = $3
            "#,
            status,
            next_run.map(|dt| dt.naive_utc()),
            schedule_id
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

// PATCH functions for partial updates
pub async fn patch_backup_job(pool: &PgPool, id: uuid::Uuid, patch_data: &UpdateBackupJob) -> Result<Option<BackupJob>, sqlx::Error> {
    let current_job = get_backup_job_by_id(pool, id).await?;
    
    if let Some(job) = current_job {
        let updated_name = patch_data.name.as_ref().unwrap_or(&job.name);
        let updated_mappings = if let Some(mappings) = &patch_data.mappings {
            serde_json::to_value(mappings).unwrap()
        } else {
            job.mappings
        };

        let updated_job = sqlx::query_as!(
            BackupJob,
            r#"
            UPDATE backup_jobs
            SET name = $1, mappings = $2, updated_at = NOW()
            WHERE id = $3 AND is_active = true
            RETURNING id, name, mappings, created_at, updated_at, deleted_at, status, is_active
            "#,
            updated_name,
            updated_mappings,
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(updated_job)
    } else {
        Ok(None)
    }
}

pub async fn patch_backup_schedule(pool: &PgPool, backup_job_id: uuid::Uuid, patch_data: &UpdateBackupSchedule) -> Result<Option<BackupSchedule>, sqlx::Error> {
    let current_schedule = get_backup_schedule_by_job_id(pool, backup_job_id).await?;
    
    if let Some(schedule) = current_schedule {
        let updated_name = patch_data.name.as_ref().unwrap_or(&schedule.name);
        let updated_cron = patch_data.cron_expression.as_ref().unwrap_or(&schedule.cron_expression);
        let updated_enabled = patch_data.enabled.unwrap_or(schedule.enabled);

        let row = sqlx::query!(
            r#"
            UPDATE backup_schedules
            SET name = $1, cron_expression = $2, enabled = $3, updated_at = NOW()
            WHERE backup_job_id = $4
            RETURNING id, backup_job_id, name, cron_expression, enabled, next_run, last_run, last_status, created_at, updated_at
            "#,
            updated_name,
            updated_cron,
            updated_enabled,
            backup_job_id
        )
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(BackupSchedule {
                id: row.id,
                backup_job_id: row.backup_job_id,
                name: row.name,
                cron_expression: row.cron_expression,
                enabled: row.enabled,
                next_run: row.next_run.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
                last_run: row.last_run.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
                last_status: row.last_status.unwrap_or_else(|| "pending".to_string()),
                created_at: DateTime::from_naive_utc_and_offset(row.created_at, Utc),
                updated_at: DateTime::from_naive_utc_and_offset(row.updated_at, Utc),
            }))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

// ========================================
// BACKUP EXECUTION LOGS FUNCTIONS
// ========================================

pub async fn create_backup_execution_log(
    pool: &PgPool, 
    log_data: &crate::models::NewBackupExecutionLog
) -> Result<crate::models::BackupExecutionLog, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        INSERT INTO backup_execution_logs (
            backup_job_id, schedule_id, rclone_command, source_path, 
            destination_path, rclone_config, triggered_by
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, backup_job_id, schedule_id, started_at, completed_at, status,
                  rclone_command, source_path, destination_path, rclone_config,
                  files_transferred, files_checked, files_deleted, bytes_transferred,
                  transfer_rate_mbps, duration_seconds, error_count, retry_count,
                  error_message, rclone_stdout, rclone_stderr, rclone_log_file_path,
                  triggered_by, created_at, updated_at
        "#,
        log_data.backup_job_id,
        log_data.schedule_id,
        log_data.rclone_command,
        log_data.source_path,
        log_data.destination_path,
        log_data.rclone_config,
        log_data.triggered_by.as_deref().unwrap_or("manual")
    )
    .fetch_one(pool)
    .await?;

    Ok(crate::models::BackupExecutionLog {
        id: row.id,
        backup_job_id: row.backup_job_id,
        schedule_id: row.schedule_id,
        started_at: row.started_at,
        completed_at: row.completed_at,
        status: row.status,
        rclone_command: row.rclone_command,
        source_path: row.source_path,
        destination_path: row.destination_path,
        rclone_config: row.rclone_config,
        files_transferred: row.files_transferred,
        files_checked: row.files_checked,
        files_deleted: row.files_deleted,
        bytes_transferred: row.bytes_transferred,
        transfer_rate_mbps: row.transfer_rate_mbps,
        duration_seconds: row.duration_seconds,
        error_count: row.error_count,
        retry_count: row.retry_count,
        error_message: row.error_message,
        rclone_stdout: row.rclone_stdout,
        rclone_stderr: row.rclone_stderr,
        rclone_log_file_path: row.rclone_log_file_path,
        triggered_by: row.triggered_by,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub async fn update_backup_execution_log_completion(
    pool: &PgPool,
    log_id: uuid::Uuid,
    result: &crate::models::RcloneExecutionResult,
) -> Result<(), sqlx::Error> {
    let status = if result.exit_code == 0 { "completed" } else { "failed" };
    
    sqlx::query!(
        r#"
        UPDATE backup_execution_logs 
        SET completed_at = NOW(),
            status = $1,
            files_transferred = $2,
            files_checked = $3,
            files_deleted = $4,
            bytes_transferred = $5,
            transfer_rate_mbps = $6,
            duration_seconds = $7,
            error_count = $8,
            error_message = $9,
            rclone_stdout = $10,
            rclone_stderr = $11,
            updated_at = NOW()
        WHERE id = $12
        "#,
        status,
        result.files_transferred,
        result.files_checked,
        result.files_deleted,
        result.bytes_transferred,
        result.transfer_rate_mbps as f32,
        result.duration_seconds,
        result.error_count,
        if result.errors.is_empty() { None } else { Some(result.errors.join("; ")) },
        result.stdout,
        result.stderr,
        log_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_backup_execution_logs(
    pool: &PgPool,
    backup_job_id: Option<uuid::Uuid>,
    limit: Option<i32>,
) -> Result<Vec<crate::models::BackupExecutionLog>, sqlx::Error> {
    let limit = limit.unwrap_or(50).min(200) as i64; // Max 200 registros

    let rows = sqlx::query!(
        r#"
        SELECT id, backup_job_id, schedule_id, started_at, completed_at, status,
               rclone_command, source_path, destination_path, rclone_config,
               files_transferred, files_checked, files_deleted, bytes_transferred,
               transfer_rate_mbps, duration_seconds, error_count, retry_count,
               error_message, rclone_stdout, rclone_stderr, rclone_log_file_path,
               triggered_by, created_at, updated_at
        FROM backup_execution_logs
        WHERE ($1::uuid IS NULL OR backup_job_id = $1)
        ORDER BY started_at DESC
        LIMIT $2
        "#,
        backup_job_id,
        limit
    )
    .fetch_all(pool)
    .await?;

    let logs: Vec<crate::models::BackupExecutionLog> = rows.into_iter().map(|row| {
        crate::models::BackupExecutionLog {
            id: row.id,
            backup_job_id: row.backup_job_id,
            schedule_id: row.schedule_id,
            started_at: row.started_at,
            completed_at: row.completed_at,
            status: row.status,
            rclone_command: row.rclone_command,
            source_path: row.source_path,
            destination_path: row.destination_path,
            rclone_config: row.rclone_config,
            files_transferred: row.files_transferred,
            files_checked: row.files_checked,
            files_deleted: row.files_deleted,
            bytes_transferred: row.bytes_transferred,
            transfer_rate_mbps: row.transfer_rate_mbps,
            duration_seconds: row.duration_seconds,
            error_count: row.error_count,
            retry_count: row.retry_count,
            error_message: row.error_message,
            rclone_stdout: row.rclone_stdout,
            rclone_stderr: row.rclone_stderr,
            rclone_log_file_path: row.rclone_log_file_path,
            triggered_by: row.triggered_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }).collect();

    Ok(logs)
}

pub async fn get_backup_execution_log_by_id(
    pool: &PgPool, 
    log_id: uuid::Uuid
) -> Result<Option<crate::models::BackupExecutionLog>, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        SELECT id, backup_job_id, schedule_id, started_at, completed_at, status,
               rclone_command, source_path, destination_path, rclone_config,
               files_transferred, files_checked, files_deleted, bytes_transferred,
               transfer_rate_mbps, duration_seconds, error_count, retry_count,
               error_message, rclone_stdout, rclone_stderr, rclone_log_file_path,
               triggered_by, created_at, updated_at
        FROM backup_execution_logs
        WHERE id = $1
        "#,
        log_id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        Ok(Some(crate::models::BackupExecutionLog {
            id: row.id,
            backup_job_id: row.backup_job_id,
            schedule_id: row.schedule_id,
            started_at: row.started_at,
            completed_at: row.completed_at,
            status: row.status,
            rclone_command: row.rclone_command,
            source_path: row.source_path,
            destination_path: row.destination_path,
            rclone_config: row.rclone_config,
            files_transferred: row.files_transferred,
            files_checked: row.files_checked,
            files_deleted: row.files_deleted,
            bytes_transferred: row.bytes_transferred,
            transfer_rate_mbps: row.transfer_rate_mbps,
            duration_seconds: row.duration_seconds,
            error_count: row.error_count,
            retry_count: row.retry_count,
            error_message: row.error_message,
            rclone_stdout: row.rclone_stdout,
            rclone_stderr: row.rclone_stderr,
            rclone_log_file_path: row.rclone_log_file_path,
            triggered_by: row.triggered_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    } else {
        Ok(None)
    }
}

pub async fn delete_backup_execution_log(
    pool: &PgPool, 
    log_id: uuid::Uuid
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM backup_execution_logs WHERE id = $1",
        log_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

// ========================================
// CLOUD PROVIDERS FUNCTIONS
// ========================================

/// Cria um novo provedor de armazenamento cloud.
/// 
/// Insere configurações de um provedor (Backblaze B2, IDrive e2, etc.)
/// no banco de dados. Se `is_default` for true, remove o padrão atual.
/// 
/// # Argumentos
/// * `pool` - Pool de conexão PostgreSQL
/// * `new_provider` - Dados do novo provedor
/// 
/// # Retorna
/// * `Ok(CloudProvider)` - Provedor criado com sucesso
/// * `Err(sqlx::Error)` - Erro de banco de dados
/// 
/// # Exemplos
/// ```no_run
/// use crate::models::{NewCloudProvider, CloudProviderType};
/// 
/// let new_provider = NewCloudProvider {
///     name: "My Backblaze B2".to_string(),
///     provider_type: CloudProviderType::BackblazeB2,
///     bucket: "my-backup-bucket".to_string(),
///     access_key: "key_id".to_string(),
///     secret_key: "app_key".to_string(),
///     endpoint: Some("https://s3.us-west-002.backblazeb2.com".to_string()),
///     region: Some("us-west-002".to_string()),
///     // ... outros campos
/// };
/// 
/// let provider = create_cloud_provider(&pool, &new_provider).await?;
/// ```
pub async fn create_cloud_provider(
    pool: &PgPool, 
    new_provider: &NewCloudProvider
) -> Result<CloudProvider, sqlx::Error> {
    let provider_type_str = match new_provider.provider_type {
        CloudProviderType::BackblazeB2 => "backblaze_b2",
        CloudProviderType::IdriveE2 => "idrive_e2",
        CloudProviderType::Wasabi => "wasabi",
        CloudProviderType::Scaleway => "scaleway",
    };

    // Se this provider deve ser default, remove o default atual
    if new_provider.is_default.unwrap_or(false) {
        sqlx::query!(
            "UPDATE cloud_providers SET is_default = false WHERE is_default = true AND is_active = true"
        )
        .execute(pool)
        .await?;
    }

    let row = sqlx::query!(
        r#"
        INSERT INTO cloud_providers (
            name, provider_type, endpoint, region, bucket, path_prefix,
            access_key, secret_key, b2_account_id, b2_application_key, 
            use_b2_native_api, is_default
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING id, name, provider_type, endpoint, region, bucket, path_prefix,
                  access_key, secret_key, b2_account_id, b2_application_key,
                  use_b2_native_api, is_active, is_default, test_connectivity_at,
                  test_connectivity_status, test_connectivity_message,
                  total_storage_bytes, total_egress_bytes, last_sync_at,
                  created_at, updated_at
        "#,
        new_provider.name,
        provider_type_str,
        new_provider.endpoint,
        new_provider.region,
        new_provider.bucket,
        new_provider.path_prefix,
        new_provider.access_key,
        new_provider.secret_key,
        new_provider.b2_account_id,
        new_provider.b2_application_key,
        new_provider.use_b2_native_api.unwrap_or(false),
        new_provider.is_default.unwrap_or(false)
    )
    .fetch_one(pool)
    .await?;

    Ok(CloudProvider {
        id: row.id,
        name: row.name,
        provider_type: row.provider_type,
        endpoint: row.endpoint,
        region: row.region,
        bucket: row.bucket,
        path_prefix: row.path_prefix,
        access_key: row.access_key,
        secret_key: row.secret_key,
        b2_account_id: row.b2_account_id,
        b2_application_key: row.b2_application_key,
        use_b2_native_api: row.use_b2_native_api.unwrap_or(false),
        is_active: row.is_active.unwrap_or(true),
        is_default: row.is_default.unwrap_or(false),
        test_connectivity_at: row.test_connectivity_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        test_connectivity_status: row.test_connectivity_status,
        test_connectivity_message: row.test_connectivity_message,
        total_storage_bytes: row.total_storage_bytes.unwrap_or(0),
        total_egress_bytes: row.total_egress_bytes.unwrap_or(0),
        last_sync_at: row.last_sync_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        created_at: DateTime::from_naive_utc_and_offset(row.created_at, Utc),
        updated_at: DateTime::from_naive_utc_and_offset(row.updated_at, Utc),
    })
}

/// Lista todos os provedores cloud ativos.
/// 
/// Retorna uma lista de todos os provedores configurados e ativos,
/// ordenados pelo padrão primeiro, depois por data de criação.
/// 
/// # Argumentos
/// * `pool` - Pool de conexão PostgreSQL
/// 
/// # Retorna
/// * `Ok(Vec<CloudProvider>)` - Lista de provedores
/// * `Err(sqlx::Error)` - Erro de banco de dados
/// 
/// # Exemplos
/// ```no_run
/// let providers = list_cloud_providers(&pool).await?;
/// for provider in providers {
///     println!("Provider: {} ({})", provider.name, provider.provider_type);
/// }
/// ```
pub async fn list_cloud_providers(pool: &PgPool) -> Result<Vec<CloudProvider>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT id, name, provider_type, endpoint, region, bucket, path_prefix,
               access_key, secret_key, b2_account_id, b2_application_key,
               use_b2_native_api, is_active, is_default, test_connectivity_at,
               test_connectivity_status, test_connectivity_message,
               total_storage_bytes, total_egress_bytes, last_sync_at,
               created_at, updated_at
        FROM cloud_providers
        WHERE is_active = true
        ORDER BY is_default DESC, created_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    let providers = rows.into_iter().map(|row| CloudProvider {
        id: row.id,
        name: row.name,
        provider_type: row.provider_type,
        endpoint: row.endpoint,
        region: row.region,
        bucket: row.bucket,
        path_prefix: row.path_prefix,
        access_key: row.access_key,
        secret_key: row.secret_key,
        b2_account_id: row.b2_account_id,
        b2_application_key: row.b2_application_key,
        use_b2_native_api: row.use_b2_native_api.unwrap_or(false),
        is_active: row.is_active.unwrap_or(true),
        is_default: row.is_default.unwrap_or(false),
        test_connectivity_at: row.test_connectivity_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        test_connectivity_status: row.test_connectivity_status,
        test_connectivity_message: row.test_connectivity_message,
        total_storage_bytes: row.total_storage_bytes.unwrap_or(0),
        total_egress_bytes: row.total_egress_bytes.unwrap_or(0),
        last_sync_at: row.last_sync_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        created_at: DateTime::from_naive_utc_and_offset(row.created_at, Utc),
        updated_at: DateTime::from_naive_utc_and_offset(row.updated_at, Utc),
    }).collect();

    Ok(providers)
}

/// Busca um provedor cloud por ID.
/// 
/// # Argumentos
/// * `pool` - Pool de conexão PostgreSQL
/// * `id` - UUID do provedor
/// 
/// # Retorna
/// * `Ok(Some(CloudProvider))` - Provedor encontrado
/// * `Ok(None)` - Provedor não encontrado
/// * `Err(sqlx::Error)` - Erro de banco de dados
pub async fn get_cloud_provider_by_id(
    pool: &PgPool, 
    id: uuid::Uuid
) -> Result<Option<CloudProvider>, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        SELECT id, name, provider_type, endpoint, region, bucket, path_prefix,
               access_key, secret_key, b2_account_id, b2_application_key,
               use_b2_native_api, is_active, is_default, test_connectivity_at,
               test_connectivity_status, test_connectivity_message,
               total_storage_bytes, total_egress_bytes, last_sync_at,
               created_at, updated_at
        FROM cloud_providers
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        Ok(Some(CloudProvider {
            id: row.id,
            name: row.name,
            provider_type: row.provider_type,
            endpoint: row.endpoint,
            region: row.region,
            bucket: row.bucket,
            path_prefix: row.path_prefix,
            access_key: row.access_key,
            secret_key: row.secret_key,
            b2_account_id: row.b2_account_id,
            b2_application_key: row.b2_application_key,
            use_b2_native_api: row.use_b2_native_api.unwrap_or(false),
            is_active: row.is_active.unwrap_or(true),
            is_default: row.is_default.unwrap_or(false),
            test_connectivity_at: row.test_connectivity_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            test_connectivity_status: row.test_connectivity_status,
            test_connectivity_message: row.test_connectivity_message,
            total_storage_bytes: row.total_storage_bytes.unwrap_or(0),
            total_egress_bytes: row.total_egress_bytes.unwrap_or(0),
            last_sync_at: row.last_sync_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            created_at: DateTime::from_naive_utc_and_offset(row.created_at, Utc),
            updated_at: DateTime::from_naive_utc_and_offset(row.updated_at, Utc),
        }))
    } else {
        Ok(None)
    }
}

/// Atualiza um provedor cloud existente.
/// 
/// # Argumentos
/// * `pool` - Pool de conexão PostgreSQL
/// * `id` - UUID do provedor
/// * `update_data` - Dados para atualizar
/// 
/// # Retorna
/// * `Ok(Some(CloudProvider))` - Provedor atualizado
/// * `Ok(None)` - Provedor não encontrado
/// * `Err(sqlx::Error)` - Erro de banco de dados
pub async fn update_cloud_provider(
    pool: &PgPool,
    id: uuid::Uuid,
    update_data: &UpdateCloudProvider
) -> Result<Option<CloudProvider>, sqlx::Error> {
    // Se está sendo definido como default, remove o default atual
    if update_data.is_default == Some(true) {
        sqlx::query!(
            "UPDATE cloud_providers SET is_default = false WHERE is_default = true AND is_active = true AND id != $1",
            id
        )
        .execute(pool)
        .await?;
    }

    // Buscar dados atuais para fazer merge
    let current = get_cloud_provider_by_id(pool, id).await?;
    if let Some(current) = current {
        let row = sqlx::query!(
            r#"
            UPDATE cloud_providers 
            SET name = $1, endpoint = $2, region = $3, bucket = $4, path_prefix = $5,
                access_key = COALESCE($6, access_key),
                secret_key = COALESCE($7, secret_key),
                b2_account_id = COALESCE($8, b2_account_id),
                b2_application_key = COALESCE($9, b2_application_key),
                use_b2_native_api = $10,
                is_active = $11,
                is_default = $12,
                updated_at = NOW()
            WHERE id = $13 AND is_active = true
            RETURNING id, name, provider_type, endpoint, region, bucket, path_prefix,
                      access_key, secret_key, b2_account_id, b2_application_key,
                      use_b2_native_api, is_active, is_default, test_connectivity_at,
                      test_connectivity_status, test_connectivity_message,
                      total_storage_bytes, total_egress_bytes, last_sync_at,
                      created_at, updated_at
            "#,
            update_data.name.as_ref().unwrap_or(&current.name),
            update_data.endpoint.as_ref().or(current.endpoint.as_ref()),
            update_data.region.as_ref().or(current.region.as_ref()),
            update_data.bucket.as_ref().unwrap_or(&current.bucket),
            update_data.path_prefix.as_ref().or(current.path_prefix.as_ref()),
            update_data.access_key.as_ref(),
            update_data.secret_key.as_ref(),
            update_data.b2_account_id.as_ref(),
            update_data.b2_application_key.as_ref(),
            update_data.use_b2_native_api.unwrap_or(current.use_b2_native_api),
            update_data.is_active.unwrap_or(current.is_active),
            update_data.is_default.unwrap_or(current.is_default),
            id
        )
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(CloudProvider {
                id: row.id,
                name: row.name,
                provider_type: row.provider_type,
                endpoint: row.endpoint,
                region: row.region,
                bucket: row.bucket,
                path_prefix: row.path_prefix,
                access_key: row.access_key,
                secret_key: row.secret_key,
                b2_account_id: row.b2_account_id,
                b2_application_key: row.b2_application_key,
                use_b2_native_api: row.use_b2_native_api.unwrap_or(false),
                is_active: row.is_active.unwrap_or(true),
                is_default: row.is_default.unwrap_or(false),
                test_connectivity_at: row.test_connectivity_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
                test_connectivity_status: row.test_connectivity_status,
                test_connectivity_message: row.test_connectivity_message,
                total_storage_bytes: row.total_storage_bytes.unwrap_or(0),
                total_egress_bytes: row.total_egress_bytes.unwrap_or(0),
                last_sync_at: row.last_sync_at.map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
                created_at: DateTime::from_naive_utc_and_offset(row.created_at, Utc),
                updated_at: DateTime::from_naive_utc_and_offset(row.updated_at, Utc),
            }))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Remove um provedor cloud (soft delete).
/// 
/// # Argumentos
/// * `pool` - Pool de conexão PostgreSQL
/// * `id` - UUID do provedor
/// 
/// # Retorna
/// * `Ok(true)` - Provedor removido com sucesso
/// * `Ok(false)` - Provedor não encontrado
/// * `Err(sqlx::Error)` - Erro de banco de dados
pub async fn delete_cloud_provider(
    pool: &PgPool, 
    id: uuid::Uuid
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE cloud_providers 
        SET is_active = false, is_default = false, updated_at = NOW()
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Testa conectividade de um provedor cloud.
/// 
/// Executa validação de credenciais baseada no tipo de provedor e campos obrigatórios.
/// Atualiza o status de conectividade no banco de dados.
/// 
/// Implementa validação por tipo:
/// - **Backblaze B2 Nativo**: Requer `b2_account_id` e `b2_application_key`
/// - **Backblaze B2 S3**: Requer `access_key`, `secret_key` e `endpoint`
/// - **IDrive e2**: Requer `access_key`, `secret_key` e `endpoint`
/// - **Wasabi/Scaleway**: Requer `access_key`, `secret_key` e `region`
/// 
/// # Argumentos  
/// * `pool` - Pool de conexão PostgreSQL
/// * `id` - UUID do provedor
/// 
/// # Retorna
/// * `Ok(ConnectivityTestResult)` - Resultado da validação
/// * `Err(sqlx::Error)` - Erro de banco de dados
/// 
/// # Exemplo
/// ```rust
/// let result = test_cloud_provider_connectivity(&pool, provider_id).await?;
/// if result.success {
///     println!("Provider configured correctly!");
/// }
/// ```
pub async fn test_cloud_provider_connectivity(
    pool: &PgPool,
    id: uuid::Uuid
) -> Result<ConnectivityTestResult, sqlx::Error> {
    let now = Utc::now();
    
    // Por enquanto, simula teste baseado na existência dos campos obrigatórios
    let provider = match get_cloud_provider_by_id(pool, id).await? {
        Some(p) => p,
        None => {
            return Ok(ConnectivityTestResult {
                success: false,
                status: ConnectivityStatus::Failed,
                message: "Provider not found".to_string(),
                tested_at: now,
                details: Some(serde_json::json!({"error": "provider_not_found"})),
            });
        }
    };
    
    // Validar campos obrigatórios baseado no tipo
    let (success, status, message) = match provider.provider_type.as_str() {
        "backblaze_b2" => {
            if provider.use_b2_native_api {
                if provider.b2_account_id.is_some() && provider.b2_application_key.is_some() {
                    (true, ConnectivityStatus::Success, "B2 native API credentials validated".to_string())
                } else {
                    (false, ConnectivityStatus::Failed, "Missing B2 native API credentials (account_id or application_key)".to_string())
                }
            } else {
                if !provider.access_key.is_empty() && !provider.secret_key.is_empty() && provider.endpoint.is_some() {
                    (true, ConnectivityStatus::Success, "B2 S3-compatible credentials validated".to_string())
                } else {
                    (false, ConnectivityStatus::Failed, "Missing B2 S3 credentials (access_key, secret_key, or endpoint)".to_string())
                }
            }
        }
        "idrive_e2" => {
            if !provider.access_key.is_empty() && !provider.secret_key.is_empty() && provider.endpoint.is_some() {
                (true, ConnectivityStatus::Success, "IDrive e2 credentials validated".to_string())
            } else {
                (false, ConnectivityStatus::Failed, "Missing IDrive e2 credentials (access_key, secret_key, or endpoint)".to_string())
            }
        }
        "wasabi" | "scaleway" => {
            if !provider.access_key.is_empty() && !provider.secret_key.is_empty() && provider.region.is_some() {
                (true, ConnectivityStatus::Success, format!("{} credentials validated", provider.provider_type))
            } else {  
                (false, ConnectivityStatus::Failed, format!("Missing {} credentials (access_key, secret_key, or region)", provider.provider_type))
            }
        }
        _ => {
            (false, ConnectivityStatus::Failed, format!("Unsupported provider type: {}", provider.provider_type))
        }
    };

    sqlx::query!(
        r#"
        UPDATE cloud_providers 
        SET test_connectivity_at = $1,
            test_connectivity_status = $2,
            test_connectivity_message = $3,
            updated_at = NOW()
        WHERE id = $4
        "#,
        now.naive_utc(),
        if success { "success" } else { "failed" },
        message,
        id
    )
    .execute(pool)
    .await?;

    Ok(ConnectivityTestResult {
        success,
        status,
        message,
        tested_at: now,
        details: Some(serde_json::json!({
            "provider_type": provider.provider_type,
            "bucket": provider.bucket,
            "use_native_api": provider.use_b2_native_api,
            "validation_only": true
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_calculate_next_run_sunday_10am() {
        // Todo domingo às 10h
        let cron_expr = "0 0 10 * * 0";
        let result = calculate_next_run(cron_expr);
        
        assert!(result.is_some(), "calculate_next_run should return Some for valid cron expression");
        let next_run = result.unwrap();
        
        // Deve ser um domingo
        assert_eq!(next_run.weekday(), chrono::Weekday::Sun);
        // Deve ser às 10h
        assert_eq!(next_run.hour(), 10);
        assert_eq!(next_run.minute(), 0);
        assert_eq!(next_run.second(), 0);
    }

    #[test]
    fn test_calculate_next_run_every_minute() {
        // A cada minuto
        let cron_expr = "0 * * * * *";
        let result = calculate_next_run(cron_expr);
        
        assert!(result.is_some());
        let next_run = result.unwrap();
        
        // Deve ser no próximo minuto ou próximo dia
        let now = Utc::now();
        assert!(next_run > now);
    }

    #[test]
    fn test_calculate_next_run_invalid_cron() {
        // Expressão inválida (menos de 6 campos)
        let cron_expr = "invalid cron";
        let result = calculate_next_run(cron_expr);
        
        assert!(result.is_none());
    }

    #[test]
    fn test_calculate_next_run_5_fields() {
        // Testar com 5 campos (formato incorreto para nossa implementação)
        let cron_expr = "0 10 * * 0";
        let result = calculate_next_run(cron_expr);
        
        // Deve retornar None porque esperamos 6 campos
        assert!(result.is_none());
    }

    #[test] 
    fn test_calculate_next_run_simple_cases() {
        // Testar nossa implementação simplificada
        
        // Caso 1: Todo minuto
        let result = calculate_next_run("0 * * * * *");
        assert!(result.is_some());
        
        // Caso 2: Todo domingo às 10h (formato correto: 6 campos)
        let result = calculate_next_run("0 0 10 * * 0");
        assert!(result.is_some());
        if let Some(next_run) = result {
            assert_eq!(next_run.weekday(), chrono::Weekday::Sun);
            assert_eq!(next_run.hour(), 10);
            assert_eq!(next_run.minute(), 0);
        }
        
        // Caso 3: Formato inválido (5 campos)
        let result = calculate_next_run("0 10 * * 0");
        assert!(result.is_none());
        
        // Caso 4: Horário específico sem dia da semana
        let result = calculate_next_run("0 30 14 * * *");
        assert!(result.is_some());
        if let Some(next_run) = result {
            assert_eq!(next_run.hour(), 14);
            assert_eq!(next_run.minute(), 30);
        }
    }
}
