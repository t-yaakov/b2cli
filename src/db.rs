use crate::models::{BackupJob, NewBackupJob};
use sqlx::PgPool;

pub async fn create_backup_job(pool: &PgPool, new_job: &NewBackupJob) -> Result<BackupJob, sqlx::Error> {
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

    Ok(job)
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