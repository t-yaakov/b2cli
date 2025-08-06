/// Rotas para agendamento de varreduras
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use tracing::{debug, info, error};
use uuid::Uuid;
use utoipa::ToSchema;
use crate::{AppError, AppState};

/// Parâmetros para criar um agendamento de scan
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateScanSchedule {
    /// Nome do agendamento
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
    /// Expressão cron (ex: "0 2 * * *" para 2AM diariamente)
    pub cron_expression: String,
    /// Se está habilitado
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Cria um novo agendamento de scan
/// 
/// # Exemplos de cron:
/// - `"0 2 * * *"` - Todos os dias às 2AM
/// - `"0 3 * * 0"` - Domingos às 3AM
/// - `"0 */6 * * *"` - A cada 6 horas
/// - `"0 0 1 * *"` - Primeiro dia do mês à meia-noite
/// 
/// # Retorna
/// * `Ok(Json)` - Agendamento criado
#[utoipa::path(
    post,
    path = "/files/scan/schedule",
    tag = "File Catalog",
    request_body = CreateScanSchedule,
    responses(
        (status = 201, description = "Agendamento criado"),
        (status = 400, description = "Parâmetros inválidos"),
        (status = 500, description = "Erro interno")
    )
)]
pub async fn create_scan_schedule(
    State(state): State<AppState>,
    Json(payload): Json<CreateScanSchedule>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        name = %payload.name,
        cron = %payload.cron_expression,
        "Criando agendamento de scan"
    );

    // Validar expressão cron básica (podemos melhorar isso depois)
    // Por enquanto, apenas verificar se não está vazia
    if payload.cron_expression.is_empty() {
        return Err(AppError::BadRequest("Expressão cron não pode ser vazia".to_string()));
    }

    // Inserir no banco
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO scan_schedules (
            name, description, root_path, recursive, max_depth,
            exclude_patterns, cron_expression, enabled
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
        "#,
        payload.name,
        payload.description,
        payload.root_path,
        payload.recursive,
        payload.max_depth,
        &payload.exclude_patterns,
        payload.cron_expression,
        payload.enabled
    )
    .fetch_one(&state.db_pool)
    .await?;

    // Adicionar ao scheduler se habilitado
    if payload.enabled {
        let db_pool = state.db_pool.clone();
        let cron_expr = payload.cron_expression.clone();
        let root_path = payload.root_path.clone();
        
        let job = tokio_cron_scheduler::Job::new_async(&cron_expr, move |_uuid, _l| {
            let db_pool = db_pool.clone();
            let schedule_id = id;
            let root_path = root_path.clone();
            
            Box::pin(async move {
                info!(schedule_id = %schedule_id, "Executando scan agendado");
                
                // Atualizar status
                let _ = sqlx::query!(
                    "UPDATE scan_schedules SET last_run_at = CURRENT_TIMESTAMP, last_run_status = 'running' WHERE id = $1",
                    schedule_id
                )
                .execute(&db_pool)
                .await;
                
                // Criar configuração
                let config = crate::file_scanner::ScanConfig {
                    root_path: std::path::PathBuf::from(&root_path),
                    recursive: true,
                    ..Default::default()
                };
                
                // Executar scan
                let mut scanner = crate::file_scanner::FileScanner::new(db_pool.clone(), config);
                match scanner.start_scan().await {
                    Ok(scan_job_id) => {
                        info!(scan_job_id = %scan_job_id, "Scan agendado concluído");
                        
                        // Atualizar com sucesso
                        let _ = sqlx::query!(
                            r#"
                            UPDATE scan_schedules 
                            SET last_run_status = 'success',
                                last_scan_job_id = $2,
                                total_runs = total_runs + 1,
                                successful_runs = successful_runs + 1
                            WHERE id = $1
                            "#,
                            schedule_id,
                            scan_job_id
                        )
                        .execute(&db_pool)
                        .await;
                    }
                    Err(e) => {
                        error!(error = %e, "Erro no scan agendado");
                        
                        // Atualizar com falha
                        let _ = sqlx::query!(
                            r#"
                            UPDATE scan_schedules 
                            SET last_run_status = 'failed',
                                total_runs = total_runs + 1,
                                failed_runs = failed_runs + 1
                            WHERE id = $1
                            "#,
                            schedule_id
                        )
                        .execute(&db_pool)
                        .await;
                    }
                }
            })
        });

        if let Ok(job) = job {
            if let Err(e) = state.scheduler.add(job).await {
                error!(error = %e, "Erro ao adicionar job ao scheduler");
            }
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": id,
            "name": payload.name,
            "cron_expression": payload.cron_expression,
            "enabled": payload.enabled,
            "message": "Agendamento criado com sucesso"
        }))
    ))
}

/// Lista todos os agendamentos de scan
#[utoipa::path(
    get,
    path = "/files/scan/schedules",
    tag = "File Catalog",
    responses(
        (status = 200, description = "Lista de agendamentos"),
        (status = 500, description = "Erro ao buscar agendamentos")
    )
)]
pub async fn list_scan_schedules(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    debug!("Listando agendamentos de scan");

    let schedules = sqlx::query!(
        r#"
        SELECT 
            id, name, description, root_path, recursive,
            max_depth, exclude_patterns, cron_expression, enabled,
            last_run_at, last_run_status, last_scan_job_id,
            total_runs, successful_runs, failed_runs,
            created_at, updated_at
        FROM scan_schedules
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(&state.db_pool)
    .await?;

    let result: Vec<_> = schedules.into_iter().map(|s| {
        json!({
            "id": s.id,
            "name": s.name,
            "description": s.description,
            "root_path": s.root_path,
            "recursive": s.recursive,
            "max_depth": s.max_depth,
            "exclude_patterns": s.exclude_patterns,
            "cron_expression": s.cron_expression,
            "enabled": s.enabled,
            "last_run_at": s.last_run_at,
            "last_run_status": s.last_run_status,
            "last_scan_job_id": s.last_scan_job_id,
            "total_runs": s.total_runs,
            "successful_runs": s.successful_runs,
            "failed_runs": s.failed_runs,
            "success_rate": if s.total_runs.unwrap_or(0) > 0 {
                (s.successful_runs.unwrap_or(0) as f64 / s.total_runs.unwrap_or(1) as f64) * 100.0
            } else {
                0.0
            },
            "created_at": s.created_at,
            "updated_at": s.updated_at
        })
    }).collect();

    Ok((StatusCode::OK, Json(result)))
}

/// Deleta um agendamento de scan
#[utoipa::path(
    delete,
    path = "/files/scan/schedule/{id}",
    tag = "File Catalog",
    params(
        ("id" = Uuid, Path, description = "ID do agendamento")
    ),
    responses(
        (status = 204, description = "Agendamento deletado"),
        (status = 404, description = "Agendamento não encontrado"),
        (status = 500, description = "Erro ao deletar")
    )
)]
pub async fn delete_scan_schedule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    info!(schedule_id = %id, "Deletando agendamento de scan");

    let result = sqlx::query!(
        "DELETE FROM scan_schedules WHERE id = $1",
        id
    )
    .execute(&state.db_pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Agendamento {} não encontrado", id)));
    }

    // TODO: Remover do scheduler também

    Ok(StatusCode::NO_CONTENT)
}

/// Habilita/desabilita um agendamento
#[utoipa::path(
    patch,
    path = "/files/scan/schedule/{id}/toggle",
    tag = "File Catalog",
    params(
        ("id" = Uuid, Path, description = "ID do agendamento")
    ),
    responses(
        (status = 200, description = "Status alterado"),
        (status = 404, description = "Agendamento não encontrado"),
        (status = 500, description = "Erro ao alterar")
    )
)]
pub async fn toggle_scan_schedule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    debug!(schedule_id = %id, "Alternando status do agendamento");

    let current = sqlx::query!(
        "SELECT enabled FROM scan_schedules WHERE id = $1",
        id
    )
    .fetch_optional(&state.db_pool)
    .await?;

    match current {
        Some(record) => {
            let new_status = !record.enabled.unwrap_or(false);
            
            sqlx::query!(
                "UPDATE scan_schedules SET enabled = $2 WHERE id = $1",
                id,
                new_status
            )
            .execute(&state.db_pool)
            .await?;

            // TODO: Adicionar/remover do scheduler

            Ok((StatusCode::OK, Json(json!({
                "id": id,
                "enabled": new_status,
                "message": if new_status { "Agendamento habilitado" } else { "Agendamento desabilitado" }
            }))))
        }
        None => Err(AppError::NotFound(format!("Agendamento {} não encontrado", id)))
    }
}