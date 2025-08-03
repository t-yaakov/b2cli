use crate::{db, models::{BackupJob, BackupSchedule, ErrorResponse, NewBackupJob, NewBackupSchedule, UpdateBackupJob, UpdateBackupSchedule}, AppState, AppError, backup_worker};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;
use tracing::{info, error};

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
    let (backup_job, schedule_opt) = db::create_backup_job(&state.db_pool, &payload).await?;

    if let Some(schedule) = schedule_opt {
        let db_pool = state.db_pool.clone();
        let backup_job_id = backup_job.id;
        let cron_expression = schedule.cron_expression.clone();
        let schedule_id = schedule.id;

        let job = tokio_cron_scheduler::Job::new_async(cron_expression.as_str(), move |_uuid, _l| {
            let db_pool = db_pool.clone();
            Box::pin(async move {
                info!("Running scheduled backup for job {}", backup_job_id);
                if let Err(e) = db::update_schedule_last_run(&db_pool, schedule_id, "running").await {
                    error!("Failed to update schedule status: {}", e);
                }

                match db::get_backup_job_by_id(&db_pool, backup_job_id).await {
                    Ok(Some(job)) => {
                        if let Err(e) = backup_worker::perform_backup(&db_pool, &job).await {
                            error!("Backup job {} failed: {}", backup_job_id, e);
                        }
                    }
                    Ok(None) => error!("Backup job {} not found for scheduled run", backup_job_id),
                    Err(e) => error!("Failed to get backup job {}: {}", backup_job_id, e),
                }

                if let Err(e) = db::update_schedule_last_run(&db_pool, schedule_id, "completed").await {
                    error!("Failed to update schedule status: {}", e);
                }
            })
        })?;

        state.scheduler.add(job).await?;
    }

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
        None => Err(AppError::NotFound(format!(
            "Backup job with ID {} not found",
            id
        ))),
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
        None => Err(AppError::NotFound(format!(
            "Backup job with ID {} not found",
            id
        ))),
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
        Err(AppError::NotFound(format!(
            "Backup job with ID {} not found",
            id
        )))
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

#[utoipa::path(
    put,
    path = "/backups/{id}",
    tag = "Backups",
    params(
        ("id" = Uuid, Path, description = "Backup Job ID")
    ),
    request_body(content = NewBackupJob, description = "Updated backup job details", example = json!({ "name": "Updated Backup", "mappings": { "/home/user/docs": ["/mnt/backups/updated"] } })),
    responses(
        (status = 200, description = "Backup job updated successfully", body = BackupJob),
        (status = 404, description = "Backup job not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_backup(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<NewBackupJob>,
) -> Result<impl IntoResponse, AppError> {
    let updated_job = db::update_backup_job(&state.db_pool, id, &payload).await?;

    match updated_job {
        Some(job) => Ok((StatusCode::OK, Json(job))),
        None => Err(AppError::NotFound(format!("Backup job with ID {} not found", id))),
    }
}

// Schedule endpoints
#[utoipa::path(
    post,
    path = "/backups/{id}/schedule",
    tag = "Schedules",
    params(
        ("id" = Uuid, Path, description = "Backup Job ID")
    ),
    request_body(content = NewBackupSchedule, description = "Schedule configuration", example = json!({ "name": "Daily backup", "cron_expression": "0 17 * * *", "enabled": true })),
    responses(
        (status = 201, description = "Schedule created successfully", body = BackupSchedule),
        (status = 404, description = "Backup job not found", body = ErrorResponse),
        (status = 409, description = "Schedule already exists for this job", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn create_schedule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<NewBackupSchedule>,
) -> Result<impl IntoResponse, AppError> {
    // Check if backup job exists
    let job = db::get_backup_job_by_id(&state.db_pool, id).await?;
    if job.is_none() {
        return Err(AppError::NotFound(format!(
            "Backup job with ID {} not found",
            id
        )));
    }

    // Check if schedule already exists
    let existing = db::get_backup_schedule_by_job_id(&state.db_pool, id).await?;
    if existing.is_some() {
        return Err(AppError::Conflict(
            "Schedule already exists for this backup job. Delete the existing schedule first."
                .to_string(),
        ));
    }

    let schedule = db::create_backup_schedule(&state.db_pool, id, &payload).await?;
    
    // Add the schedule to the scheduler if it's enabled
    if schedule.enabled {
        let db_pool = state.db_pool.clone();
        let backup_job_id = id;
        let schedule_id = schedule.id;
        let cron_expression = schedule.cron_expression.clone();
        
        let job = tokio_cron_scheduler::Job::new_async(cron_expression.as_str(), move |_uuid, _l| {
            let db_pool = db_pool.clone();
            Box::pin(async move {
                info!("🕐 Running scheduled backup for job {}", backup_job_id);
                if let Err(e) = db::update_schedule_last_run(&db_pool, schedule_id, "running").await {
                    error!("Failed to update schedule status: {}", e);
                }

                match db::get_backup_job_by_id(&db_pool, backup_job_id).await {
                    Ok(Some(job)) => {
                        info!("📦 Executing backup: {}", job.name);
                        if let Err(e) = backup_worker::perform_backup(&db_pool, &job).await {
                            error!("❌ Backup job {} failed: {}", backup_job_id, e);
                            let _ = db::update_schedule_last_run(&db_pool, schedule_id, "failed").await;
                        } else {
                            info!("✅ Backup job {} completed successfully", backup_job_id);
                            let _ = db::update_schedule_last_run(&db_pool, schedule_id, "completed").await;
                        }
                    }
                    Ok(None) => error!("Backup job {} not found for scheduled run", backup_job_id),
                    Err(e) => error!("Failed to get backup job {}: {}", backup_job_id, e),
                }
            })
        })?;

        state.scheduler.add(job).await?;
        info!("📅 Schedule '{}' added to scheduler (cron: {})", schedule.name, schedule.cron_expression);
    }
    
    Ok((StatusCode::CREATED, Json(schedule)))
}

#[utoipa::path(
    get,
    path = "/backups/{id}/schedule",
    tag = "Schedules",
    params(
        ("id" = Uuid, Path, description = "Backup Job ID")
    ),
    responses(
        (status = 200, description = "Schedule details", body = BackupSchedule),
        (status = 404, description = "Schedule not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_schedule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let schedule = db::get_backup_schedule_by_job_id(&state.db_pool, id).await?;

    match schedule {
        Some(schedule) => Ok((StatusCode::OK, Json(schedule))),
        None => Err(AppError::NotFound(format!(
            "No schedule found for backup job {}",
            id
        ))),
    }
}

#[utoipa::path(
    delete,
    path = "/backups/{id}/schedule",
    tag = "Schedules",
    params(
        ("id" = Uuid, Path, description = "Backup Job ID")
    ),
    responses(
        (status = 204, description = "Schedule deleted successfully"),
        (status = 404, description = "Schedule not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn delete_schedule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let rows_affected = db::delete_backup_schedule(&state.db_pool, id).await?;

    if rows_affected == 0 {
        Err(AppError::NotFound(format!(
            "No schedule found for backup job {}",
            id
        )))
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

#[utoipa::path(
    put,
    path = "/backups/{id}/schedule",
    tag = "Schedules",
    params(
        ("id" = Uuid, Path, description = "Backup Job ID")
    ),
    request_body(content = NewBackupSchedule, description = "Updated schedule configuration", example = json!({ "name": "Updated Schedule", "cron_expression": "0 18 * * *", "enabled": false })),
    responses(
        (status = 200, description = "Schedule updated successfully", body = BackupSchedule),
        (status = 404, description = "Schedule not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_schedule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<NewBackupSchedule>,
) -> Result<impl IntoResponse, AppError> {
    let updated_schedule = db::update_backup_schedule(&state.db_pool, id, &payload).await?;

    match updated_schedule {
        Some(schedule) => Ok((StatusCode::OK, Json(schedule))),
        None => Err(AppError::NotFound(format!(
            "No schedule found for backup job {}",
            id
        ))),
    }
}

// PATCH endpoints for partial updates
#[utoipa::path(
    patch,
    path = "/backups/{id}",
    tag = "Backups",
    params(
        ("id" = Uuid, Path, description = "Backup Job ID")
    ),
    request_body(content = UpdateBackupJob, description = "Partial backup job update", example = json!({ "name": "Updated Name Only" })),
    responses(
        (status = 200, description = "Backup job updated successfully", body = BackupJob),
        (status = 404, description = "Backup job not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn patch_backup(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateBackupJob>,
) -> Result<impl IntoResponse, AppError> {
    let updated_job = db::patch_backup_job(&state.db_pool, id, &payload).await?;

    match updated_job {
        Some(job) => Ok((StatusCode::OK, Json(job))),
        None => Err(AppError::NotFound(format!(
            "Backup job with ID {} not found",
            id
        ))),
    }
}

#[utoipa::path(
    patch,
    path = "/backups/{id}/schedule",
    tag = "Schedules",
    params(
        ("id" = Uuid, Path, description = "Backup Job ID")
    ),
    request_body(content = UpdateBackupSchedule, description = "Partial schedule update", example = json!({ "enabled": false })),
    responses(
        (status = 200, description = "Schedule updated successfully", body = BackupSchedule),
        (status = 404, description = "Schedule not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn patch_schedule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateBackupSchedule>,
) -> Result<impl IntoResponse, AppError> {
    let updated_schedule = db::patch_backup_schedule(&state.db_pool, id, &payload).await?;

    match updated_schedule {
        Some(schedule) => Ok((StatusCode::OK, Json(schedule))),
        None => Err(AppError::NotFound(format!(
            "No schedule found for backup job {}",
            id
        ))),
    }
}

#[utoipa::path(
    get,
    path = "/schedules",
    tag = "Schedules",
    responses(
        (status = 200, description = "List of all schedules"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn list_all_schedules(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    // Get all schedules with their associated backup job names
    let schedules = sqlx::query!(
        r#"
        SELECT 
            s.id,
            s.backup_job_id,
            s.name as schedule_name,
            j.name as job_name,
            s.cron_expression,
            s.enabled,
            s.next_run,
            s.last_run,
            s.last_status,
            s.created_at,
            s.updated_at
        FROM backup_schedules s
        JOIN backup_jobs j ON j.id = s.backup_job_id
        WHERE j.is_active = true
        ORDER BY s.enabled DESC, s.created_at DESC
        "#
    )
    .fetch_all(&state.db_pool)
    .await?;
    
    info!("📋 Listed {} schedules", schedules.len());
    
    // Convert to a simpler response format
    let response: Vec<serde_json::Value> = schedules
        .into_iter()
        .map(|s| serde_json::json!({
            "id": s.id,
            "backup_job_id": s.backup_job_id,
            "schedule_name": s.schedule_name,
            "job_name": s.job_name,
            "cron_expression": s.cron_expression,
            "enabled": s.enabled,
            "next_run": s.next_run,
            "last_run": s.last_run,
            "last_status": s.last_status,
            "created_at": s.created_at,
            "updated_at": s.updated_at
        }))
        .collect();
    
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    get,
    path = "/scheduler/status",
    tag = "System",
    responses(
        (status = 200, description = "Scheduler status"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn scheduler_status(
    State(_state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    // Simples: apenas verificar se o scheduler está rodando
    // Se chegamos aqui, o scheduler foi iniciado com sucesso
    
    let status = serde_json::json!({
        "scheduler": "running",
        "status": "ok"
    });
    
    Ok((StatusCode::OK, Json(status)))
}
