/// Rotas para varredura e busca de arquivos
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use tracing::{debug, info};
use uuid::Uuid;
use utoipa::ToSchema;

use crate::{
    file_scanner::{FileScanner, ScanConfig},
    AppError, AppState,
};

/// Parâmetros para criar uma configuração de scan
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateScanConfig {
    /// Nome da configuração
    pub name: String,
    /// Descrição opcional
    pub description: Option<String>,
    /// Diretório raiz para varredura
    pub root_path: String,
    /// Se deve varrer recursivamente
    #[serde(default = "default_true")]
    pub recursive: bool,
    /// Profundidade máxima
    pub max_depth: Option<i32>,
    /// Padrões para excluir
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Cria uma configuração de scan
/// 
/// Cria uma configuração de scan que pode ser executada posteriormente.
/// Segue o mesmo padrão de backup_jobs: criar primeiro, executar depois.
/// 
/// # Exemplos de uso:
/// 
/// **Configuração básica:**
/// ```json
/// {
///   "name": "Scan Documentos",
///   "root_path": "/home/user/Documents",
///   "recursive": true
/// }
/// ```
/// 
/// **Com exclusões e limite:**
/// ```json
/// {
///   "name": "Scan Projeto",
///   "description": "Varre o projeto excluindo node_modules",
///   "root_path": "/workspace/projeto",
///   "recursive": true,
///   "max_depth": 5,
///   "exclude_patterns": ["node_modules/*", "*.tmp", ".git/*"]
/// }
/// ```
/// 
/// # Parâmetros
/// * `name` - Nome da configuração
/// * `root_path` - Caminho absoluto da pasta para escanear
/// * `recursive` - true = escanea subpastas, false = só a pasta atual
/// * `max_depth` - (Opcional) Profundidade máxima. Se omitido = sem limite
/// * `exclude_patterns` - (Opcional) Padrões glob para ignorar arquivos
/// 
/// # Retorna
/// * `Ok(Json)` - Configuração criada com ID
#[utoipa::path(
    post,
    path = "/files/scan",
    tag = "File Catalog",
    request_body = CreateScanConfig,
    responses(
        (status = 201, description = "Configuração criada"),
        (status = 400, description = "Parâmetros inválidos"),
        (status = 500, description = "Erro interno")
    )
)]
pub async fn create_scan_config(
    State(state): State<AppState>,
    Json(payload): Json<CreateScanConfig>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        name = %payload.name,
        root_path = %payload.root_path,
        "Criando configuração de scan"
    );

    // Inserir no banco
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO scan_configs (
            name, description, root_path, recursive, 
            max_depth, exclude_patterns, status
        ) VALUES ($1, $2, $3, $4, $5, $6, 'PENDING')
        RETURNING id
        "#,
        payload.name,
        payload.description,
        payload.root_path,
        payload.recursive,
        payload.max_depth,
        &payload.exclude_patterns
    )
    .fetch_one(&state.db_pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": id,
            "name": payload.name,
            "root_path": payload.root_path,
            "status": "PENDING",
            "message": "Configuração de scan criada. Use POST /files/scan/{id}/run para executar"
        }))
    ))
}

/// Busca arquivos duplicados
/// 
/// Encontra arquivos com o mesmo hash (conteúdo idêntico)
/// 
/// # Retorna
/// * `Ok(Json)` - Lista de arquivos duplicados
#[utoipa::path(
    get,
    path = "/files/duplicates",
    tag = "File Catalog",
    responses(
        (status = 200, description = "Arquivos duplicados encontrados"),
        (status = 500, description = "Erro ao buscar duplicados")
    )
)]
pub async fn find_duplicate_files(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    debug!("Buscando arquivos duplicados");

    let duplicates = sqlx::query!(
        r#"
        SELECT 
            content_hash,
            array_agg(file_path) as paths,
            array_agg(file_name) as names,
            COUNT(*) as count,
            MAX(file_size) as file_size
        FROM file_catalog
        WHERE is_active = true 
          AND content_hash IS NOT NULL
        GROUP BY content_hash
        HAVING COUNT(*) > 1
        ORDER BY file_size DESC
        "#
    )
    .fetch_all(&state.db_pool)
    .await?;

    let result: Vec<_> = duplicates.into_iter().map(|d| {
        json!({
            "hash": d.content_hash,
            "count": d.count,
            "size_bytes": d.file_size,
            "size_mb": d.file_size.unwrap_or(0) as f64 / 1_048_576.0,
            "paths": d.paths,
            "names": d.names,
            "wasted_space_bytes": d.file_size.unwrap_or(0) * (d.count.unwrap_or(1) - 1)
        })
    }).collect();

    info!(count = result.len(), "Duplicados encontrados");

    Ok((StatusCode::OK, Json(result)))
}

/// Executa uma configuração de scan
/// 
/// Executa uma configuração de scan previamente criada.
/// O scan é executado em background e retorna imediatamente.
/// 
/// # Retorna
/// * `Ok(Json)` - Scan iniciado com ID do job
#[utoipa::path(
    post,
    path = "/files/scan/{id}/run",
    tag = "File Catalog",
    params(
        ("id" = Uuid, Path, description = "ID da configuração de scan")
    ),
    responses(
        (status = 202, description = "Scan iniciado"),
        (status = 404, description = "Configuração não encontrada"),
        (status = 409, description = "Scan já está em execução"),
        (status = 500, description = "Erro ao iniciar scan")
    )
)]
pub async fn run_scan_config(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    info!(config_id = %id, "Executando configuração de scan");

    // Buscar configuração
    let config_record = sqlx::query!(
        r#"
        SELECT id, name, root_path, recursive, max_depth, 
               exclude_patterns, status, is_active
        FROM scan_configs
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .fetch_optional(&state.db_pool)
    .await?;

    let config_record = config_record
        .ok_or_else(|| AppError::NotFound(format!("Configuração {} não encontrada", id)))?;

    // Verificar se já está rodando
    if config_record.status == Some("RUNNING".to_string()) {
        return Err(AppError::Conflict("Scan já está em execução".to_string()));
    }

    // Atualizar status para RUNNING
    sqlx::query!(
        "UPDATE scan_configs SET status = 'RUNNING', last_run_at = CURRENT_TIMESTAMP WHERE id = $1",
        id
    )
    .execute(&state.db_pool)
    .await?;

    // Criar configuração do scanner
    let scan_config = ScanConfig {
        root_path: std::path::PathBuf::from(&config_record.root_path),
        recursive: config_record.recursive.unwrap_or(true),
        max_depth: config_record.max_depth,
        exclude_patterns: config_record.exclude_patterns.unwrap_or_default(),
        ..Default::default()
    };

    // Executar scan em background
    let db_pool = state.db_pool.clone();
    let config_id = id;
    let config_name = config_record.name.clone();
    
    tokio::spawn(async move {
        info!(config_id = %config_id, "🔥 ROUTE: Criando scanner");
        let mut scanner = FileScanner::new(db_pool.clone(), scan_config);
        
        info!(config_id = %config_id, "🔥 ROUTE: Iniciando scan");
        match scanner.start_scan().await {
            Ok(scan_job_id) => {
                info!(
                    config_id = %config_id,
                    scan_job_id = %scan_job_id,
                    "🔥 ROUTE: Scan concluído com sucesso"
                );
                
                // Atualizar status e estatísticas
                let _ = sqlx::query!(
                    r#"
                    UPDATE scan_configs 
                    SET status = 'COMPLETED',
                        last_scan_job_id = $2,
                        total_runs = total_runs + 1,
                        successful_runs = successful_runs + 1
                    WHERE id = $1
                    "#,
                    config_id,
                    scan_job_id
                )
                .execute(&db_pool)
                .await;
                
                // Atualizar scan_job com referência ao config
                let _ = sqlx::query!(
                    "UPDATE scan_jobs SET scan_config_id = $1 WHERE id = $2",
                    config_id,
                    scan_job_id
                )
                .execute(&db_pool)
                .await;
            }
            Err(e) => {
                tracing::error!(
                    config_id = %config_id,
                    error = %e,
                    error_debug = ?e,
                    "🔥 ROUTE: Erro ao executar scan"
                );
                
                // Atualizar status para FAILED
                let _ = sqlx::query!(
                    r#"
                    UPDATE scan_configs 
                    SET status = 'FAILED',
                        total_runs = total_runs + 1,
                        failed_runs = failed_runs + 1
                    WHERE id = $1
                    "#,
                    config_id
                )
                .execute(&db_pool)
                .await;
            }
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "id": id,
            "name": config_name,
            "status": "RUNNING",
            "message": "Scan iniciado em background"
        }))
    ))
}

/// Lista todas as configurações de scan
/// 
/// Retorna todas as configurações de scan criadas
/// 
/// # Retorna
/// * `Ok(Json)` - Lista de configurações
#[utoipa::path(
    get,
    path = "/files/scan/configs",
    tag = "File Catalog",
    responses(
        (status = 200, description = "Lista de configurações"),
        (status = 500, description = "Erro ao buscar configurações")
    )
)]
pub async fn list_scan_configs(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    debug!("Listando configurações de scan");

    let configs = sqlx::query!(
        r#"
        SELECT 
            id, name, description, root_path, recursive,
            max_depth, exclude_patterns, status, is_active,
            last_run_at, last_scan_job_id, total_runs,
            successful_runs, failed_runs, created_at
        FROM scan_configs
        WHERE is_active = true
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(&state.db_pool)
    .await?;

    let result: Vec<_> = configs.into_iter().map(|c| {
        json!({
            "id": c.id,
            "name": c.name,
            "description": c.description,
            "root_path": c.root_path,
            "recursive": c.recursive,
            "max_depth": c.max_depth,
            "exclude_patterns": c.exclude_patterns,
            "status": c.status,
            "last_run_at": c.last_run_at,
            "last_scan_job_id": c.last_scan_job_id,
            "total_runs": c.total_runs.unwrap_or(0),
            "successful_runs": c.successful_runs.unwrap_or(0),
            "failed_runs": c.failed_runs.unwrap_or(0),
            "success_rate": if c.total_runs.unwrap_or(0) > 0 {
                (c.successful_runs.unwrap_or(0) as f64 / c.total_runs.unwrap_or(1) as f64) * 100.0
            } else {
                0.0
            },
            "created_at": c.created_at
        })
    }).collect();

    info!(count = result.len(), "Configurações de scan encontradas");

    Ok((StatusCode::OK, Json(result)))
}

/// Lista todos os jobs de varredura
/// 
/// Retorna todos os jobs de varredura com seu status
/// 
/// # Retorna
/// * `Ok(Json)` - Lista de jobs de varredura
#[utoipa::path(
    get,
    path = "/files/scan",
    tag = "File Catalog",
    responses(
        (status = 200, description = "Lista de jobs de varredura"),
        (status = 500, description = "Erro ao buscar jobs")
    )
)]
pub async fn list_scan_jobs(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    debug!("Listando jobs de varredura");

    let jobs = sqlx::query!(
        r#"
        SELECT 
            id,
            root_path,
            status,
            started_at,
            completed_at,
            files_scanned,
            directories_scanned,
            total_size_bytes,
            errors_count,
            duration_seconds,
            created_at
        FROM scan_jobs
        ORDER BY created_at DESC
        LIMIT 100
        "#
    )
    .fetch_all(&state.db_pool)
    .await?;

    let result: Vec<_> = jobs.into_iter().map(|j| {
        json!({
            "id": j.id,
            "root_path": j.root_path,
            "status": j.status,
            "started_at": j.started_at,
            "completed_at": j.completed_at,
            "files_scanned": j.files_scanned,
            "directories_scanned": j.directories_scanned,
            "total_size_bytes": j.total_size_bytes,
            "total_size_gb": j.total_size_bytes.unwrap_or(0) as f64 / 1_073_741_824.0,
            "errors_count": j.errors_count,
            "duration_seconds": j.duration_seconds,
            "created_at": j.created_at
        })
    }).collect();

    info!(count = result.len(), "Jobs de varredura encontrados");

    Ok((StatusCode::OK, Json(result)))
}

/// Status de um job de varredura
/// 
/// Obtém o status de um job de varredura específico
/// 
/// # Argumentos
/// * `id` - ID do job
/// 
/// # Retorna
/// * `Ok(Json)` - Status do job
#[utoipa::path(
    get,
    path = "/files/scan/{id}",
    tag = "File Catalog",
    params(
        ("id" = Uuid, Path, description = "ID do job de varredura")
    ),
    responses(
        (status = 200, description = "Status do job"),
        (status = 404, description = "Job não encontrado"),
        (status = 500, description = "Erro interno")
    )
)]
pub async fn get_scan_job_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    debug!(job_id = %id, "Buscando status do job de varredura");

    let job = sqlx::query!(
        r#"
        SELECT 
            id,
            root_path,
            status,
            started_at,
            completed_at,
            files_scanned,
            directories_scanned,
            total_size_bytes,
            errors_count,
            duration_seconds,
            error_message
        FROM scan_jobs
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&state.db_pool)
    .await?;

    match job {
        Some(j) => {
            let response = json!({
                "id": j.id,
                "root_path": j.root_path,
                "status": j.status,
                "started_at": j.started_at,
                "completed_at": j.completed_at,
                "files_scanned": j.files_scanned,
                "directories_scanned": j.directories_scanned,
                "total_size_bytes": j.total_size_bytes,
                "total_size_gb": j.total_size_bytes.unwrap_or(0) as f64 / 1_073_741_824.0,
                "errors_count": j.errors_count,
                "duration_seconds": j.duration_seconds,
                "error_message": j.error_message
            });
            
            Ok((StatusCode::OK, Json(response)))
        }
        None => Err(AppError::NotFound(format!("Job {} not found", id)))
    }
}