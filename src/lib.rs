use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sqlx::postgres::PgPool;
use std::sync::Arc;
use tokio_cron_scheduler::JobScheduler;
use std::path::StripPrefixError;
use std::fmt;

pub mod backup_worker;
pub mod db;
pub mod logging;
pub mod models;
pub mod rclone;
pub mod routes;
pub mod scheduler;
pub mod archiver;
pub mod file_scanner;
pub mod config_manager;
pub mod crypto;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub scheduler: Arc<JobScheduler>,
}

#[derive(Debug)]
pub enum AppError {
    SqlxError(sqlx::Error),
    IoError(std::io::Error),
    SchedulerError(tokio_cron_scheduler::JobSchedulerError),
    BackupError(Box<dyn std::error::Error + Send + Sync>),
    NotFound(String),
    Conflict(String),
    BadRequest(String),
    InternalServerError(String),
    SerdeJsonError(serde_json::Error),
    StripPrefixError(StripPrefixError),
    RcloneError(anyhow::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::SqlxError(e) => write!(f, "Database error: {}", e),
            AppError::IoError(e) => write!(f, "IO error: {}", e),
            AppError::SchedulerError(e) => write!(f, "Scheduler error: {}", e),
            AppError::BackupError(e) => write!(f, "Backup operation failed: {}", e),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::InternalServerError(msg) => write!(f, "Internal server error: {}", msg),
            AppError::SerdeJsonError(e) => write!(f, "JSON error: {}", e),
            AppError::StripPrefixError(e) => write!(f, "Path prefix error: {}", e),
            AppError::RcloneError(e) => write!(f, "Rclone error: {}", e),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::SqlxError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ),
            AppError::IoError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ),
            AppError::SchedulerError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ),
            AppError::BackupError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::SerdeJsonError(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            AppError::StripPrefixError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::RcloneError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::SqlxError(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::IoError(e)
    }
}

impl From<tokio_cron_scheduler::JobSchedulerError> for AppError {
    fn from(e: tokio_cron_scheduler::JobSchedulerError) -> Self {
        AppError::SchedulerError(e)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for AppError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> AppError {
        AppError::BackupError(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> AppError {
        AppError::SerdeJsonError(err)
    }
}

impl From<StripPrefixError> for AppError {
    fn from(err: StripPrefixError) -> AppError {
        AppError::StripPrefixError(err)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> AppError {
        AppError::RcloneError(err)
    }
}