use crate::models::{BackupJob, NewBackupJob, BackupSchedule, NewBackupSchedule, UpdateBackupJob, UpdateBackupSchedule};
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
