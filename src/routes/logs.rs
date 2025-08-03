// src/routes/logs.rs
// HTTP handlers for backup execution logs

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use sqlx::Row;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    db,
    models::{BackupExecutionLog, NewBackupExecutionLog, ErrorResponse},
    AppError, AppState,
};

#[derive(Deserialize, IntoParams)]
pub struct LogsQueryParams {
    #[serde(rename = "backup_job_id")]
    pub backup_job_id: Option<Uuid>,
    pub limit: Option<i32>,
}

#[utoipa::path(
    get,
    path = "/logs",
    tag = "Logs",
    params(LogsQueryParams),
    responses(
        (status = 200, description = "List of backup execution logs", body = Vec<BackupExecutionLog>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn list_logs(
    State(state): State<AppState>,
    Query(params): Query<LogsQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    let logs = db::list_backup_execution_logs(
        &state.db_pool, 
        params.backup_job_id,
        params.limit
    ).await?;
    
    Ok((StatusCode::OK, Json(logs)))
}

#[utoipa::path(
    get,
    path = "/logs/{id}",
    tag = "Logs",
    params(
        ("id" = Uuid, Path, description = "Backup execution log ID")
    ),
    responses(
        (status = 200, description = "Backup execution log details", body = BackupExecutionLog),
        (status = 404, description = "Log not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_log(
    State(state): State<AppState>,
    Path(log_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let log = db::get_backup_execution_log_by_id(&state.db_pool, log_id).await?;
    
    match log {
        Some(log) => Ok((StatusCode::OK, Json(log))),
        None => Err(AppError::NotFound(format!("Log with ID {} not found", log_id))),
    }
}

#[utoipa::path(
    post,
    path = "/logs",
    tag = "Logs",
    request_body(content = NewBackupExecutionLog, description = "Backup execution log data"),
    responses(
        (status = 201, description = "Backup execution log created", body = BackupExecutionLog),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn create_log(
    State(state): State<AppState>,
    Json(log_data): Json<NewBackupExecutionLog>,
) -> Result<impl IntoResponse, AppError> {
    // Verify backup job exists
    let job = db::get_backup_job_by_id(&state.db_pool, log_data.backup_job_id).await?;
    if job.is_none() {
        return Err(AppError::NotFound(format!(
            "Backup job with ID {} not found",
            log_data.backup_job_id
        )));
    }

    let log = db::create_backup_execution_log(&state.db_pool, &log_data).await?;
    Ok((StatusCode::CREATED, Json(log)))
}

#[utoipa::path(
    delete,
    path = "/logs/{id}",
    tag = "Logs",
    params(
        ("id" = Uuid, Path, description = "Backup execution log ID")
    ),
    responses(
        (status = 200, description = "Log deleted successfully"),
        (status = 404, description = "Log not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn delete_log(
    State(state): State<AppState>,
    Path(log_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = db::delete_backup_execution_log(&state.db_pool, log_id).await?;
    
    if deleted {
        Ok((StatusCode::OK, Json(serde_json::json!({"message": "Log deleted successfully"}))))
    } else {
        Err(AppError::NotFound(format!("Log with ID {} not found", log_id)))
    }
}

// Helper endpoint to get logs for a specific backup job
#[utoipa::path(
    get,
    path = "/backups/{id}/logs",
    tag = "Logs",
    params(
        ("id" = Uuid, Path, description = "Backup job ID"),
        ("limit" = Option<i32>, Query, description = "Maximum number of logs to return (default: 50, max: 200)")
    ),
    responses(
        (status = 200, description = "List of execution logs for the backup job", body = Vec<BackupExecutionLog>),
        (status = 404, description = "Backup job not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_backup_logs(
    State(state): State<AppState>,
    Path(backup_job_id): Path<Uuid>,
    Query(params): Query<LogsQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    // Verify backup job exists
    let job = db::get_backup_job_by_id(&state.db_pool, backup_job_id).await?;
    if job.is_none() {
        return Err(AppError::NotFound(format!(
            "Backup job with ID {} not found",
            backup_job_id
        )));
    }

    let logs = db::list_backup_execution_logs(
        &state.db_pool, 
        Some(backup_job_id),
        params.limit
    ).await?;
    
    Ok((StatusCode::OK, Json(logs)))
}

#[derive(serde::Serialize, ToSchema)]
pub struct LogsStatsResponse {
    pub total_executions: i64,
    pub successful_executions: i64,
    pub failed_executions: i64,
    pub success_rate: f64,
    pub total_files_transferred: i64,
    pub total_bytes_transferred: i64,
    pub average_duration_seconds: f64,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
}

#[utoipa::path(
    get,
    path = "/logs/stats",
    tag = "Logs",
    params(
        ("backup_job_id" = Option<Uuid>, Query, description = "Filter stats by backup job ID")
    ),
    responses(
        (status = 200, description = "Backup execution statistics", body = LogsStatsResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_logs_stats(
    State(state): State<AppState>,
    Query(params): Query<LogsQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    let job_filter = if let Some(job_id) = params.backup_job_id {
        format!("WHERE backup_job_id = '{}'", job_id)
    } else {
        String::new()
    };

    let stats_query = format!(
        r#"
        SELECT 
            COUNT(*) as total_executions,
            COUNT(*) FILTER (WHERE status = 'completed') as successful_executions,
            COUNT(*) FILTER (WHERE status = 'failed') as failed_executions,
            COALESCE(SUM(files_transferred), 0) as total_files_transferred,
            COALESCE(SUM(bytes_transferred), 0) as total_bytes_transferred,
            COALESCE(AVG(duration_seconds), 0) as average_duration_seconds,
            MAX(started_at) as last_execution
        FROM backup_execution_logs
        {}
        "#,
        job_filter
    );

    let row = sqlx::query(&stats_query)
        .fetch_one(&state.db_pool)
        .await?;

    let total: i64 = row.get("total_executions");
    let successful: i64 = row.get("successful_executions");
    let failed: i64 = row.get("failed_executions");
    let success_rate = if total > 0 { 
        (successful as f64 / total as f64) * 100.0 
    } else { 
        0.0 
    };

    let stats = LogsStatsResponse {
        total_executions: total,
        successful_executions: successful,
        failed_executions: failed,
        success_rate,
        total_files_transferred: row.get("total_files_transferred"),
        total_bytes_transferred: row.get("total_bytes_transferred"),
        average_duration_seconds: row.get("average_duration_seconds"),
        last_execution: row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_execution"),
    };

    Ok((StatusCode::OK, Json(stats)))
}