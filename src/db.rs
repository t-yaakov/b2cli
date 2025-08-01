use crate::models::{BackupJob, NewBackupJob};
use sqlx::PgPool;

pub async fn create_backup_job(pool: &PgPool, new_job: &NewBackupJob) -> Result<BackupJob, sqlx::Error> {
    let job = sqlx::query_as!(
        BackupJob,
        r#"
        INSERT INTO backup_jobs (name, source_path, destination_path)
        VALUES ($1, $2, $3)
        RETURNING id, name, source_path, destination_path, created_at
        "#,
        new_job.name,
        new_job.source_path,
        new_job.destination_path
    )
    .fetch_one(pool)
    .await?;

    Ok(job)
}

pub async fn list_backup_jobs(pool: &PgPool) -> Result<Vec<BackupJob>, sqlx::Error> {
    let jobs = sqlx::query_as!(
        BackupJob,
        r#"
        SELECT id, name, source_path, destination_path, created_at
        FROM backup_jobs
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(jobs)
}

pub async fn get_postgres_version(pool: &PgPool) -> Result<String, sqlx::Error> {
    let row: (String,) = sqlx::query_as("SHOW server_version").fetch_one(pool).await?;
    Ok(row.0)
}
