use crate::db;
use crate::models::{BackupJob, NewBackupJob, ErrorResponse};
use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;
use crate::backup_worker;
use std::fmt;
use std::path::StripPrefixError;

// Custom Error Type
pub enum AppError {
    SqlxError(sqlx::Error),
    BackupError(Box<dyn std::error::Error + Send + Sync>),
    NotFound(String),
    InternalServerError(String),
    SerdeJsonError(serde_json::Error),
    IoError(std::io::Error),
    StripPrefixError(StripPrefixError),
}

// Implement From for sqlx::Error
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> AppError {
        AppError::SqlxError(err)
    }
}

// Implement From for Box<dyn std::error::Error>
impl From<Box<dyn std::error::Error + Send + Sync>> for AppError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> AppError {
        AppError::BackupError(err)
    }
}

// Implement From for serde_json::Error
impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> AppError {
        AppError::SerdeJsonError(err)
    }
}

// Implement From for std::io::Error
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> AppError {
        AppError::IoError(err)
    }
}

// Implement From for std::path::StripPrefixError
impl From<StripPrefixError> for AppError {
    fn from(err: StripPrefixError) -> AppError {
        AppError::StripPrefixError(err)
    }
}

// Implement Display for AppError
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::SqlxError(e) => write!(f, "Database error: {}", e),
            AppError::BackupError(e) => write!(f, "Backup operation error: {}", e),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::InternalServerError(msg) => write!(f, "Internal server error: {}", msg),
            AppError::SerdeJsonError(e) => write!(f, "JSON error: {}", e),
            AppError::IoError(e) => write!(f, "IO error: {}", e),
            AppError::StripPrefixError(e) => write!(f, "Path prefix error: {}", e),
        }
    }
}

// Implement IntoResponse for AppError
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AppError::SqlxError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::BackupError(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Backup operation failed: {}", e)),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::SerdeJsonError(e) => (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)),
            AppError::IoError(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("IO Error: {}", e)),
            AppError::StripPrefixError(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Path Error: {}", e)),
        };

        let error_response = ErrorResponse { message: error_message };
        (status, Json(error_response)).into_response()
    }
}

#[utoipa::path(
    post,
    path = "/backups",
    tag = "Backups",
    request_body(content = NewBackupJob, description = "New backup job details", example = json!({ "name": "My Daily Backup", "mappings": { "/home/user/docs": ["/mnt/backups/daily", "s3://my-bucket/daily"] } })),
    responses(
        (status = 201, description = "Backup job created successfully", body = BackupJob),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn create_backup(
    State(state): State<AppState>,
    Json(payload): Json<NewBackupJob>,
) -> Result<impl IntoResponse, AppError> {
    let backup_job = db::create_backup_job(&state.db_pool, &payload).await?;
    Ok((StatusCode::CREATED, Json(backup_job)))
}

#[utoipa::path(
    post,
    path = "/backups/{id}/run",
    tag = "Backups",
    params(
        ("id" = Uuid, Path, description = "Backup Job ID")
    ),
    responses(
        (status = 200, description = "Backup job started successfully"),
        (status = 404, description = "Backup job not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn run_backup(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let job = db::get_backup_job_by_id(&state.db_pool, id).await?;

    match job {
        Some(job) => {
            backup_worker::perform_backup(&state.db_pool, &job).await?;
            Ok(StatusCode::OK)
        }
        None => Err(AppError::NotFound(format!("Backup job with ID {} not found", id.to_string()))),
    }
}

#[utoipa::path(
    get,
    path = "/backups",
    tag = "Backups",
    responses(
        (status = 200, description = "List all backup jobs", body = [BackupJob]),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn list_backups(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let jobs = db::list_backup_jobs(&state.db_pool).await?;
    Ok((StatusCode::OK, Json(jobs)))
}

#[utoipa::path(
    get,
    path = "/backups/{id}",
    tag = "Backups",
    params(
        ("id" = Uuid, Path, description = "Backup Job ID")
    ),
    responses(
        (status = 200, description = "Show backup job details", body = BackupJob),
        (status = 404, description = "Backup job not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_backup(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let job = db::get_backup_job_by_id(&state.db_pool, id).await?;

    match job {
        Some(job) => Ok((StatusCode::OK, Json(job))),
        None => Err(AppError::NotFound(format!("Backup job with ID {} not found", id.to_string()))),
    }
}

#[utoipa::path(
    delete,
    path = "/backups/{id}",
    tag = "Backups",
    params(
        ("id" = Uuid, Path, description = "Backup Job ID")
    ),
    responses(
        (status = 204, description = "Backup job deleted successfully"),
        (status = 404, description = "Backup job not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn delete_backup(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let rows_affected = db::delete_backup_job(&state.db_pool, id).await?;

    if rows_affected == 0 {
        Err(AppError::NotFound(format!("Backup job with ID {} not found", id.to_string())))
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}