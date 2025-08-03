// src/routes/archive.rs
// HTTP handlers para sistema de arquivamento de logs

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use std::path::PathBuf;

use crate::{
    archiver::{LogArchiver, ArchivePolicy, ArchiveStatus, ArchiveResult},
    models::ErrorResponse,
    AppError, AppState,
};

#[derive(Deserialize, ToSchema)]
pub struct ArchivePolicyUpdate {
    /// Minutos para manter logs no banco (mín: 1, máx: 525600 = 1 ano)
    pub hot_retention_minutes: Option<i32>,
    /// Meses para manter em Parquet (mín: 1, máx: 120 = 10 anos)
    pub warm_retention_months: Option<i32>,
    /// Ativar/desativar arquivamento automático
    pub auto_archive_enabled: Option<bool>,
    /// Tamanho em GB para trigger de compressão (mín: 0.001 = 1MB)
    pub compress_threshold_gb: Option<f64>,
    /// Intervalo em minutos para arquivamento automático (mín: 1, máx: 10080 = 1 semana)
    pub auto_archive_interval_minutes: Option<i32>,
}

#[derive(Deserialize)]
pub struct ForceArchiveQuery {
    #[serde(default)]
    pub target: ArchiveTarget,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ArchiveTarget {
    #[default]
    Warm,  // Para Parquet
    Cold,  // Para compressão
}

#[utoipa::path(
    get,
    path = "/archive/status",
    tag = "Archive",
    responses(
        (status = 200, description = "Archive system status", body = ArchiveStatus),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_archive_status(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let archive_dir = PathBuf::from("./archive");
    let archiver = LogArchiver::new(state.db_pool.clone(), archive_dir, None);
    
    let status = archiver.get_archive_status().await
        .map_err(|e| AppError::InternalServerError(format!("Failed to get archive status: {}", e)))?;
    
    Ok((StatusCode::OK, Json(status)))
}

#[utoipa::path(
    get,
    path = "/archive/policy",
    tag = "Archive",
    responses(
        (status = 200, description = "Current archive policy", body = ArchivePolicy),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_archive_policy(
    State(_state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Carregar política do banco/config
    let policy = ArchivePolicy::default();
    Ok((StatusCode::OK, Json(policy)))
}

#[utoipa::path(
    put,
    path = "/archive/policy",
    tag = "Archive",
    request_body(content = ArchivePolicyUpdate, description = "Archive policy updates"),
    responses(
        (status = 200, description = "Policy updated successfully", body = ArchivePolicy),
        (status = 400, description = "Invalid policy parameters", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_archive_policy(
    State(_state): State<AppState>,
    Json(policy_update): Json<ArchivePolicyUpdate>,
) -> Result<impl IntoResponse, AppError> {
    // Validar parâmetros
    if let Some(minutes) = policy_update.hot_retention_minutes {
        if minutes < 1 || minutes > 525600 {
            return Err(AppError::NotFound("hot_retention_minutes must be between 1 and 525600 (1 year)".to_string()));
        }
    }
    
    if let Some(months) = policy_update.warm_retention_months {
        if months < 1 || months > 120 {
            return Err(AppError::NotFound("warm_retention_months must be between 1 and 120 (10 years)".to_string()));
        }
    }

    if let Some(threshold) = policy_update.compress_threshold_gb {
        if threshold < 0.001 {
            return Err(AppError::NotFound("compress_threshold_gb must be at least 0.001 (1MB)".to_string()));
        }
    }

    if let Some(interval) = policy_update.auto_archive_interval_minutes {
        if interval < 1 || interval > 10080 {
            return Err(AppError::NotFound("auto_archive_interval_minutes must be between 1 and 10080 (1 week)".to_string()));
        }
    }

    // TODO: Salvar política no banco/config
    let mut current_policy = ArchivePolicy::default();
    
    if let Some(minutes) = policy_update.hot_retention_minutes {
        current_policy.hot_retention_minutes = minutes;
    }
    if let Some(months) = policy_update.warm_retention_months {
        current_policy.warm_retention_months = months;
    }
    if let Some(enabled) = policy_update.auto_archive_enabled {
        current_policy.auto_archive_enabled = enabled;
    }
    if let Some(threshold) = policy_update.compress_threshold_gb {
        current_policy.compress_threshold_gb = threshold;
    }
    if let Some(interval) = policy_update.auto_archive_interval_minutes {
        current_policy.auto_archive_interval_minutes = interval;
    }

    Ok((StatusCode::OK, Json(current_policy)))
}

#[utoipa::path(
    post,
    path = "/archive/manual",
    tag = "Archive",
    params(
        ("target" = Option<String>, Query, description = "Archive target: 'warm' or 'cold' (default: warm)")
    ),
    responses(
        (status = 200, description = "Manual archive completed", body = ArchiveResult),
        (status = 400, description = "Invalid target parameter", body = ErrorResponse),
        (status = 500, description = "Archive operation failed", body = ErrorResponse)
    )
)]
pub async fn force_manual_archive(
    State(state): State<AppState>,
    Query(params): Query<ForceArchiveQuery>,
) -> Result<impl IntoResponse, AppError> {
    let archive_dir = PathBuf::from("./archive");
    let archiver = LogArchiver::new(state.db_pool.clone(), archive_dir, None);
    
    let result = match params.target {
        ArchiveTarget::Warm => {
            tracing::info!("Manual archive to warm storage requested");
            archiver.force_archive_to_warm().await
        }
        ArchiveTarget::Cold => {
            tracing::info!("Manual compression to cold storage requested");
            archiver.force_compress_to_cold().await
        }
    };

    match result {
        Ok(archive_result) => {
            tracing::info!(
                archived_records = archive_result.archived_records,
                created_files = archive_result.created_files.len(),
                freed_space_mb = archive_result.freed_space_mb,
                duration_seconds = archive_result.duration_seconds,
                "Manual archive completed successfully"
            );
            Ok((StatusCode::OK, Json(archive_result)))
        }
        Err(e) => {
            tracing::error!("Manual archive failed: {}", e);
            Err(AppError::InternalServerError(format!("Archive operation failed: {}", e)))
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct ArchiveOperationResponse {
    pub message: String,
    pub operation: String,
    pub estimated_duration_minutes: f64,
    pub job_id: Option<String>, // Para operações assíncronas no futuro
}

#[utoipa::path(
    post,
    path = "/archive/compress",
    tag = "Archive",
    responses(
        (status = 200, description = "Compression started", body = ArchiveResult),
        (status = 500, description = "Compression failed", body = ErrorResponse)
    )
)]
pub async fn force_compress_archive(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let archive_dir = PathBuf::from("./archive");
    let archiver = LogArchiver::new(state.db_pool.clone(), archive_dir, None);
    
    tracing::info!("Manual compression to cold storage requested");
    
    let result = archiver.force_compress_to_cold().await
        .map_err(|e| AppError::InternalServerError(format!("Compression failed: {}", e)))?;
    
    tracing::info!(
        created_files = result.created_files.len(),
        freed_space_mb = result.freed_space_mb,
        duration_seconds = result.duration_seconds,
        "Manual compression completed successfully"
    );

    Ok((StatusCode::OK, Json(result)))
}

#[derive(Serialize, ToSchema)]
pub struct CleanupPreviewResponse {
    pub hot_records_to_archive: i64,
    pub estimated_freed_space_mb: f64,
    pub warm_files_to_compress: i64,
    pub estimated_compression_ratio: f64,
}

#[utoipa::path(
    get,
    path = "/archive/preview",
    tag = "Archive",
    responses(
        (status = 200, description = "Preview of what would be archived", body = CleanupPreviewResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn preview_archive_operation(
    State(_state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Implementar preview real
    let preview = CleanupPreviewResponse {
        hot_records_to_archive: 15420,
        estimated_freed_space_mb: 89.3,
        warm_files_to_compress: 3,
        estimated_compression_ratio: 0.68,
    };

    Ok((StatusCode::OK, Json(preview)))
}