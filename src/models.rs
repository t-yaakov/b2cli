use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct BackupJob {
    #[serde(skip_deserializing)]
    pub id: i32,
    pub name: String,
    pub source_path: String,
    pub destination_path: String,
    #[serde(skip_deserializing)]
    pub created_at: DateTime<Utc>,
}

// A version of BackupJob for creating new entries, without the ID
#[derive(Deserialize, ToSchema)]
pub struct NewBackupJob {
    pub name: String,
    pub source_path: String,
    pub destination_path: String,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub message: String,
}
