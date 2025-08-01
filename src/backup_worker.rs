use crate::routes::backups::AppError;
use crate::models::{BackupJob, BackedUpFile};
use sqlx::PgPool;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncReadExt;
use sha2::{Sha256, Digest};
use uuid::Uuid;
use chrono::Utc;
use std::future::Future;
use std::pin::Pin;

pub async fn perform_backup(pool: &PgPool, job: &BackupJob) -> Result<(), AppError> {
    tracing::info!(job_id = %job.id, job_name = %job.name, "Starting backup job");
    tracing::debug!(job_id = %job.id, mappings = ?job.mappings, "Job mappings");
    
    // Update job status to RUNNING
    crate::db::update_backup_job_status(pool, job.id, "RUNNING").await?;

    let mappings: std::collections::HashMap<String, Vec<String>> = serde_json::from_value(job.mappings.clone())?;

    let result = async {
        for (source_path_str, destination_paths) in mappings {
            tracing::debug!(source = %source_path_str, destinations = ?destination_paths, "Processing mapping");
            let source_path = PathBuf::from(&source_path_str);

            if source_path.exists() {
                let metadata = fs::metadata(&source_path).await?;
                tracing::debug!(
                    path = ?source_path,
                    is_file = metadata.is_file(), 
                    is_dir = metadata.is_dir(), 
                    "Source path metadata"
                );
                
                traverse_and_process_path(pool, job.id, &source_path, &source_path, &destination_paths).await?;
            } else {
                tracing::warn!(path = ?source_path, "Source path does not exist");
            }
        }
        Ok::<(), AppError>(()) // Explicitly return Ok with unit type
    }.await;

    // Update job status based on result
    match result {
        Ok(_) => {
            crate::db::update_backup_job_status(pool, job.id, "COMPLETED").await?;
            tracing::info!(job_id = %job.id, job_name = %job.name, "Backup job completed successfully");
            Ok(())
        },
        Err(e) => {
            crate::db::update_backup_job_status(pool, job.id, "FAILED").await?;
            tracing::error!(job_id = %job.id, job_name = %job.name, error = %e, "Backup job failed");
            Err(e)
        }
    }
}

// This function now returns a Pin<Box<dyn Future>> to handle recursion
fn traverse_and_process_path<'a>(
    pool: &'a PgPool,
    backup_job_id: Uuid,
    base_source_path: &'a Path,
    current_path: &'a Path,
    destination_paths: &'a Vec<String>,
) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
    Box::pin(async move {
        tracing::trace!(path = ?current_path, "Traversing path");
        
        // Get metadata to check file type
        let metadata = match fs::metadata(current_path).await {
            Ok(m) => m,
            Err(e) => {
                tracing::error!(path = ?current_path, error = %e, "Error getting metadata");
                return Err(AppError::IoError(e));
            }
        };
        
        if metadata.is_file() {
            tracing::trace!(path = ?current_path, "Processing file");
            process_file(pool, backup_job_id, base_source_path, current_path, destination_paths).await?;
        } else if metadata.is_dir() {
            tracing::trace!(path = ?current_path, "Processing directory");
            let mut entries = fs::read_dir(current_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                traverse_and_process_path(pool, backup_job_id, base_source_path, &path, destination_paths).await?;
            }
        } else {
            tracing::warn!(path = ?current_path, "Skipping special file");
        }
        Ok(())
    })
}

async fn process_file(
    pool: &PgPool,
    backup_job_id: Uuid,
    base_source_path: &Path,
    original_file_path: &Path,
    destination_paths: &Vec<String>,
) -> Result<(), AppError> {
    tracing::trace!(path = ?original_file_path, "Processing file for backup");
    
    // Double-check that this is actually a file
    let metadata = fs::metadata(&original_file_path).await?;
    if !metadata.is_file() {
        tracing::error!(
            path = ?original_file_path, 
            is_dir = metadata.is_dir(), 
            is_file = metadata.is_file(),
            "process_file called on non-file"
        );
        return Err(AppError::InternalServerError(
            format!("Expected file but found directory or special file: {:?}", original_file_path)
        ));
    }
    
    let file_name = original_file_path
        .file_name()
        .ok_or_else(|| AppError::InternalServerError("Invalid file path".to_string()))?
        .to_string_lossy()
        .to_string();
    let file_extension = original_file_path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file_size = metadata.len() as i64;

    tracing::trace!(path = ?original_file_path, "Opening file for reading");
    let mut file = fs::File::open(&original_file_path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;
    hasher.update(&buffer);
    let checksum = format!("{:x}", hasher.finalize());

    // Calculate relative path from base_source_path
    let relative_path = original_file_path.strip_prefix(base_source_path)?;

    for dest_path_str in destination_paths {
        let dest_path = PathBuf::from(dest_path_str);
        let backed_up_file_path = if relative_path.as_os_str().is_empty() {
            // If relative_path is empty, we're backing up a single file
            // Use the original file name in the destination directory
            dest_path.join(original_file_path.file_name().unwrap())
        } else {
            // Otherwise, preserve the directory structure
            dest_path.join(relative_path)
        };

        tracing::debug!(
            source = ?original_file_path, 
            destination = ?backed_up_file_path, 
            "Copying file"
        );

        // Ensure parent directories exist
        if let Some(parent) = backed_up_file_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::copy(&original_file_path, &backed_up_file_path).await?;

        let backed_up_file = BackedUpFile {
            id: Uuid::new_v4(),
            backup_job_id,
            original_path: original_file_path.to_string_lossy().to_string(),
            backed_up_path: backed_up_file_path.to_string_lossy().to_string(),
            file_name: file_name.clone(),
            file_extension: file_extension.clone(),
            file_size,
            checksum: checksum.clone(),
            backed_up_at: Utc::now(),
        };

        sqlx::query!(
            r#"
            INSERT INTO backed_up_files (
                id, backup_job_id, original_path, backed_up_path,
                file_name, file_extension, file_size, checksum, backed_up_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            backed_up_file.id,
            backed_up_file.backup_job_id,
            backed_up_file.original_path,
            backed_up_file.backed_up_path,
            backed_up_file.file_name,
            backed_up_file.file_extension,
            backed_up_file.file_size,
            backed_up_file.checksum,
            backed_up_file.backed_up_at,
        )
        .execute(pool)
        .await?;

        tracing::info!(
            source = %original_file_path.to_string_lossy(), 
            destination = %backed_up_file_path.to_string_lossy(),
            size_bytes = file_size,
            checksum = %checksum,
            "File backed up successfully"
        );
    }

    Ok(())
}