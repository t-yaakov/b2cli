use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use sqlx::FromRow;
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, ToSchema, Debug, FromRow)]
pub struct BackupJob {
    #[serde(skip_deserializing)]
    pub id: Uuid,
    pub name: String,
    #[schema(example = json!({ "/home/user/docs": ["/mnt/backups/daily", "s3://my-bucket/daily"] }),
              value_type = HashMap<String, Vec<String>>)]
    pub mappings: serde_json::Value,
    #[serde(skip_deserializing)]
    pub created_at: DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub updated_at: DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub deleted_at: Option<DateTime<Utc>>,
    pub status: String,
    #[serde(skip_deserializing)]
    pub is_active: bool,
}

// A version of BackupJob for creating new entries, without the ID
#[derive(Deserialize, ToSchema)]
pub struct NewBackupJob {
    pub schedule: Option<NewBackupSchedule>,
    pub name: String,
    pub mappings: HashMap<String, Vec<String>>,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub message: String,
}

#[derive(Serialize, Deserialize, ToSchema, Debug, FromRow)]
pub struct BackedUpFile {
    #[serde(skip_deserializing)]
    pub id: Uuid,
    pub backup_job_id: Uuid,
    pub original_path: String,
    pub backed_up_path: String,
    pub file_name: String,
    pub file_extension: String,
    pub file_size: i64,
    pub checksum: String,
    #[serde(skip_deserializing)]
    pub backed_up_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema, Debug, FromRow)]
pub struct BackupSchedule {
    #[serde(skip_deserializing)]
    pub id: Uuid,
    pub backup_job_id: Uuid,
    pub name: String,
    pub cron_expression: String,
    pub enabled: bool,
    #[serde(skip_deserializing)]
    pub next_run: Option<DateTime<Utc>>,
    #[serde(skip_deserializing)]
    pub last_run: Option<DateTime<Utc>>,
    #[serde(skip_deserializing)]
    pub last_status: String,
    #[serde(skip_deserializing)]
    pub created_at: DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct NewBackupSchedule {
    pub name: String,
    #[schema(example = "0 17 * * *")]
    pub cron_expression: String,
    pub enabled: Option<bool>,
}

// Update models for PATCH operations
#[derive(Deserialize, ToSchema)]
pub struct UpdateBackupJob {
    pub name: Option<String>,
    pub mappings: Option<HashMap<String, Vec<String>>>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateBackupSchedule {
    pub name: Option<String>,
    #[schema(example = "0 18 * * *")]
    pub cron_expression: Option<String>,
    pub enabled: Option<bool>,
}