use crate::{db, AppState};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use std::process::Command;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct DependencyStatus {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ReadinessResponse {
    rclone: DependencyStatus,
    database: DependencyStatus,
}

/// Readiness check endpoint
#[utoipa::path(
    get,
    path = "/readiness",
    tag = "System",
    responses(
        (status = 200, description = "Returns the status of critical dependencies", body = ReadinessResponse)
    )
)]
pub async fn readiness_check(State(state): State<AppState>) -> impl IntoResponse {
    let rclone_status = check_rclone();
    let db_status = check_database(&state.db_pool).await;

    let response = ReadinessResponse {
        rclone: rclone_status,
        database: db_status,
    };

    (StatusCode::OK, Json(response))
}

fn check_rclone() -> DependencyStatus {
    let output = Command::new("rclone").arg("version").output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8(output.stdout)
                .unwrap_or_default()
                .lines()
                .next()
                .unwrap_or("unknown")
                .trim()
                .to_string();
            DependencyStatus {
                status: "ok".to_string(),
                version: Some(version),
                message: None,
            }
        }
        _ => DependencyStatus {
            status: "error".to_string(),
            version: None,
            message: Some("rclone not found in system PATH".to_string()),
        },
    }
}

async fn check_database(pool: &sqlx::PgPool) -> DependencyStatus {
    match db::get_postgres_version(pool).await {
        Ok(version) => DependencyStatus {
            status: "ok".to_string(),
            version: Some(version),
            message: None,
        },
        Err(e) => DependencyStatus {
            status: "error".to_string(),
            version: None,
            message: Some(e.to_string()),
        },
    }
}