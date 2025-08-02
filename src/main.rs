use axum::{
    routing::{get, post},
    Router,
};
use b2cli::{
    db,
    logging,
    models::{BackupJob, NewBackupJob, BackupSchedule, NewBackupSchedule, UpdateBackupJob, UpdateBackupSchedule, ErrorResponse},
    routes::{self, backups::*, health::*, readiness::*},
    scheduler,
    AppState,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tracing::{error, info};
use utoipa::OpenApi;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::health::health_check,
        routes::readiness::readiness_check,
        routes::backups::create_backup,
        routes::backups::list_backups,
        routes::backups::get_backup,
        routes::backups::delete_backup,
        routes::backups::update_backup,
        routes::backups::run_backup,
        routes::backups::create_schedule,
        routes::backups::get_schedule,
        routes::backups::delete_schedule,
        routes::backups::update_schedule,
        routes::backups::patch_backup,
        routes::backups::patch_schedule,
    ),
    components(
        schemas(ReadinessResponse, DependencyStatus, BackupJob, NewBackupJob, BackupSchedule, NewBackupSchedule, UpdateBackupJob, UpdateBackupSchedule, ErrorResponse)
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

    // Create the scheduler
    let scheduler = scheduler::create_scheduler()
        .await
        .expect("Failed to create scheduler");

    // Load schedules from the database and add them to the scheduler
    let schedules = db::list_active_schedules(&db_pool)
        .await
        .expect("Failed to load schedules");
    for schedule in schedules {
        let db_pool_clone = db_pool.clone();
        let schedule_id = schedule.id;
        let backup_job_id = schedule.backup_job_id;
        let job = tokio_cron_scheduler::Job::new_async(schedule.cron_expression.as_str(), move |_uuid, _l| {
            let db_pool = db_pool_clone.clone();
            Box::pin(async move {
                info!("Running scheduled backup for job {}", backup_job_id);
                if let Err(e) = db::update_schedule_last_run(&db_pool, schedule_id, "running").await {
                    error!("Failed to update schedule status: {}", e);
                }

                let job = db::get_backup_job_by_id(&db_pool, backup_job_id)
                    .await
                    .unwrap();
                if let Some(job) = job {
                    b2cli::backup_worker::perform_backup(&db_pool, &job)
                        .await
                        .unwrap();
                }

                if let Err(e) = db::update_schedule_last_run(&db_pool, schedule_id, "completed").await {
                    error!("Failed to update schedule status: {}", e);
                }
            })
        });

        if let Ok(job) = job {
            if let Err(e) = scheduler.add(job).await {
                error!("Failed to add job to scheduler: {}", e);
            }
        }
    }

    let app_state = AppState {
        db_pool,
        scheduler: Arc::new(scheduler),
    };

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(Redoc::with_url("/redoc", ApiDoc::openapi()))
        .route("/health", get(health_check))
        .route("/readiness", get(readiness_check))
        .route("/backups", post(create_backup).get(list_backups))
        .route(
            "/backups/{id}",
            get(get_backup)
                .put(update_backup)
                .patch(patch_backup)
                .delete(delete_backup),
        )
        .route("/backups/{id}/run", post(run_backup))
        .route(
            "/backups/{id}/schedule",
            post(create_schedule)
                .get(get_schedule)
                .put(update_schedule)
                .patch(patch_schedule)
                .delete(delete_schedule),
        )
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("🚀 Server started successfully");
    tracing::info!(
        "    Swagger UI: http://{}/swagger-ui",
        listener.local_addr().unwrap()
    );
    tracing::info!(
        "    Redoc:      http://{}/redoc",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, app).await.unwrap();
}
