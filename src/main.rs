use axum::{
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use utoipa::OpenApi;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

pub mod db;
pub mod models;
pub mod routes;
pub mod backup_worker;
pub mod logging;

use models::{BackupJob, NewBackupJob, ErrorResponse};
use routes::health::health_check;
use routes::readiness::{readiness_check, ReadinessResponse, DependencyStatus};
use routes::backups::{create_backup, list_backups, get_backup, delete_backup, run_backup};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::PgPool,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::health::health_check,
        routes::readiness::readiness_check,
        routes::backups::create_backup,
        routes::backups::list_backups,
        routes::backups::get_backup,
        routes::backups::delete_backup,
        routes::backups::run_backup,
    ),
    components(
        schemas(ReadinessResponse, DependencyStatus, BackupJob, NewBackupJob, ErrorResponse)
    ),
    tags(
        (name = "System", description = "System health and status endpoints"),
        (name = "Backups", description = "Backup job management endpoints")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    // Load .env file
    dotenvy::dotenv().expect("Failed to read .env file");

    // Initialize logging
    logging::init_logging().expect("Failed to initialize logging");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create database pool");

    let app_state = AppState {
        db_pool,
    };

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(Redoc::with_url("/redoc", ApiDoc::openapi()))
        .route("/health", get(health_check))
        .route("/readiness", get(readiness_check))
        .route("/backups", post(create_backup).get(list_backups))
        .route("/backups/{id}", get(get_backup).delete(delete_backup))
        .route("/backups/{id}/run", post(run_backup))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("🚀 Server started successfully");
    tracing::info!("    Swagger UI: http://{}/swagger-ui", listener.local_addr().unwrap());
    tracing::info!("    Redoc:      http://{}/redoc", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
