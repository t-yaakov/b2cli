// src/scheduler.rs

use anyhow::Result;
use tokio_cron_scheduler::JobScheduler;
use tracing::info;

pub async fn create_scheduler() -> Result<JobScheduler> {
    info!("Creating a new scheduler");
    let scheduler = JobScheduler::new().await?;
    Ok(scheduler)
}