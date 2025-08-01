use crate::db;
use crate::models::{BackupJob, NewBackupJob, ErrorResponse};
use crate::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};

#[utoipa::path(
    post,
    path = "/backups",
    tag = "Backups",
    request_body = NewBackupJob,
    responses(
        (status = 201, description = "Backup job created successfully", body = BackupJob),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn create_backup(
    State(state): State<AppState>,
    Json(payload): Json<NewBackupJob>,
) -> impl IntoResponse {
    match db::create_backup_job(&state.db_pool, &payload).await {
        Ok(backup_job) => (StatusCode::CREATED, Json(backup_job)).into_response(),
        Err(e) => {
            let error_response = ErrorResponse {
                message: e.to_string(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
        }
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
pub async fn list_backups(State(state): State<AppState>) -> impl IntoResponse {
    match db::list_backup_jobs(&state.db_pool).await {
        Ok(jobs) => (StatusCode::OK, Json(jobs)).into_response(),
        Err(e) => {
            let error_response = ErrorResponse {
                message: e.to_string(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
        }
    }
}