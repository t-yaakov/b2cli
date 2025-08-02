use crate::models::{BackupJob, NewBackupJob, BackupSchedule, NewBackupSchedule, UpdateBackupJob, UpdateBackupSchedule};
use sqlx::PgPool;
use chrono::{DateTime, Utc};

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
    let schedule = sqlx::query!(
        r#"
        INSERT INTO backup_schedules (backup_job_id, name, cron_expression, enabled)
        VALUES ($1, $2, $3, $4)
        RETURNING id, backup_job_id, name, cron_expression, enabled, next_run, last_run, last_status, created_at, updated_at
        "#,
        backup_job_id,
        new_schedule.name,
        new_schedule.cron_expression,
        new_schedule.enabled.unwrap_or(true)
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
    sqlx::query!(
        r#"
        UPDATE backup_schedules
        SET last_run = NOW(), last_status = $1, updated_at = NOW()
        WHERE id = $2
        "#,
        status,
        schedule_id
    )
    .execute(pool)
    .await?;

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
