use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    tag = "System",
    responses(
        (status = 200, description = "Service is running")
    )
)]
pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}
