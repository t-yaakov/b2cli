// src/rclone.rs
// Wrapper for rclone command with comprehensive logging

use crate::models::{RcloneExecutionResult, RcloneLogEntry};
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, error, warn};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RcloneConfig {
    pub log_level: String,
    pub stats_interval: String, 
    pub dry_run: bool,
    pub verbose: bool,
    pub transfers: Option<u32>,
    pub checkers: Option<u32>,
    pub extra_flags: Vec<String>,
}

impl Default for RcloneConfig {
    fn default() -> Self {
        Self {
            log_level: "INFO".to_string(),
            stats_interval: "10s".to_string(),
            dry_run: false,
            verbose: false,
            transfers: Some(4),
            checkers: Some(8),
            extra_flags: vec![],
        }
    }
}

pub struct RcloneWrapper {
    config: RcloneConfig,
    log_dir: PathBuf,
}

impl RcloneWrapper {
    pub fn new(config: RcloneConfig, log_dir: Option<PathBuf>) -> Self {
        let log_dir = log_dir.unwrap_or_else(|| PathBuf::from("/tmp/b2cli_logs"));
        Self { config, log_dir }
    }

    /// Execute rclone sync command with comprehensive logging
    pub async fn sync(
        &self,
        job_id: Uuid,
        source: &str,
        destination: &str,
    ) -> Result<RcloneExecutionResult> {
        // Ensure log directory exists
        fs::create_dir_all(&self.log_dir).await?;

        let log_file = self.log_dir.join(format!("rclone_{}.json", job_id));
        let log_file_str = log_file.to_string_lossy();

        // Build rclone command
        let mut cmd = Command::new("rclone");
        cmd.arg("sync")
            .arg(source)
            .arg(destination)
            .arg("--log-file")
            .arg(&*log_file_str)
            .arg("--use-json-log")
            .arg("--log-level")
            .arg(&self.config.log_level)
            .arg("--stats")
            .arg(&self.config.stats_interval)
            .arg("--stats-log-level")
            .arg("INFO");

        // Add optional config
        if let Some(transfers) = self.config.transfers {
            cmd.arg("--transfers").arg(transfers.to_string());
        }
        if let Some(checkers) = self.config.checkers {
            cmd.arg("--checkers").arg(checkers.to_string());
        }
        if self.config.dry_run {
            cmd.arg("--dry-run");
        }
        if self.config.verbose {
            cmd.arg("-vv");
        }

        // Add extra flags
        for flag in &self.config.extra_flags {
            cmd.arg(flag);
        }

        // Execute command
        let command_str = format!("{:?}", cmd);
        debug!("Executing rclone command for job {}: {}", job_id, command_str);

        let start_time = std::time::Instant::now();
        
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        // Capture stdout and stderr
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to capture stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("Failed to capture stderr"))?;

        let mut stdout_lines = BufReader::new(stdout).lines();
        let mut stderr_lines = BufReader::new(stderr).lines();

        let mut stdout_content = String::new();
        let mut stderr_content = String::new();

        // Read outputs
        tokio::select! {
            _ = async {
                while let Ok(Some(line)) = stdout_lines.next_line().await {
                    stdout_content.push_str(&line);
                    stdout_content.push('\n');
                    debug!("rclone stdout: {}", line);
                }
            } => {},
            _ = async {
                while let Ok(Some(line)) = stderr_lines.next_line().await {
                    stderr_content.push_str(&line);
                    stderr_content.push('\n');
                    if line.contains("ERROR") {
                        error!("rclone stderr: {}", line);
                    } else {
                        debug!("rclone stderr: {}", line);
                    }
                }
            } => {},
        }

        // Wait for command to complete
        let output = child.wait().await?;
        let duration = start_time.elapsed();

        debug!(
            "Rclone command completed for job {} in {:.2}s with exit code: {}",
            job_id,
            duration.as_secs_f64(),
            output.code().unwrap_or(-1)
        );

        // Parse logs
        let result = self.parse_logs(&log_file, output.code().unwrap_or(-1), 
                                   duration.as_secs() as i32, stdout_content, stderr_content).await?;

        // Limpar arquivo de log do rclone após parsear
        if log_file.exists() {
            if let Err(e) = fs::remove_file(&log_file).await {
                warn!("Failed to delete rclone log file {:?}: {}", log_file, e);
            } else {
                debug!("Deleted rclone log file {:?}", log_file);
            }
        }

        Ok(result)
    }

    /// Parse rclone JSON logs to extract statistics
    async fn parse_logs(
        &self,
        log_file: &PathBuf,
        exit_code: i32,
        duration_seconds: i32,
        stdout: String,
        stderr: String,
    ) -> Result<RcloneExecutionResult> {
        let mut files_transferred = 0;
        let mut files_checked = 0;
        let files_deleted = 0;
        let mut bytes_transferred = 0i64;
        let mut transfer_rate_mbps = 0.0f32;
        let mut error_count = 0;
        let mut errors = Vec::new();

        if log_file.exists() {
            let content = fs::read_to_string(log_file).await?;
            
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<RcloneLogEntry>(line) {
                    Ok(entry) => {
                        match entry.level.as_str() {
                            "INFO" => {
                                self.parse_info_log(&entry, &mut files_transferred, 
                                                  &mut files_checked, &mut bytes_transferred, 
                                                  &mut transfer_rate_mbps);
                            },
                            "ERROR" => {
                                error_count += 1;
                                errors.push(entry.msg.clone());
                                warn!("Rclone error: {}", entry.msg);
                            },
                            "NOTICE" => {
                                if entry.msg.contains("Transferred:") {
                                    self.parse_transfer_summary(&entry, &mut files_transferred,
                                                              &mut bytes_transferred);
                                }
                            },
                            _ => {}
                        }
                    },
                    Err(e) => {
                        debug!("Failed to parse log line: {} - Error: {}", line, e);
                    }
                }
            }
        } else {
            warn!("Rclone log file not found: {:?}", log_file);
        }

        Ok(RcloneExecutionResult {
            exit_code,
            files_transferred,
            files_checked,
            files_deleted,
            bytes_transferred,
            transfer_rate_mbps,
            duration_seconds,
            error_count,
            errors,
            stdout,
            stderr,
        })
    }

    fn parse_info_log(
        &self,
        entry: &RcloneLogEntry,
        files_transferred: &mut i32,
        files_checked: &mut i32,
        bytes_transferred: &mut i64,
        transfer_rate_mbps: &mut f32,
    ) {
        // Parse different types of INFO messages
        if entry.msg.contains("Transferred:") && entry.msg.contains("ETA") {
            // Progress message: "Transferred: 123.45 MiB / 500 MiB, 25%, 12.34 MiB/s, ETA 30s"
            if let Some(rate_part) = entry.msg.split("MiB/s").next() {
                if let Some(rate_str) = rate_part.split(',').last() {
                    if let Ok(rate) = rate_str.trim().parse::<f32>() {
                        *transfer_rate_mbps = rate;
                    }
                }
            }
        }

        // Look for specific fields in the extra data
        if let Some(stats) = entry.extra.get("stats") {
            if let Some(obj) = stats.as_object() {
                if let Some(files) = obj.get("transfers") {
                    if let Some(n) = files.as_i64() {
                        *files_transferred = n as i32;
                    }
                }
                if let Some(bytes) = obj.get("bytes") {
                    if let Some(n) = bytes.as_i64() {
                        *bytes_transferred = n;
                    }
                }
                if let Some(checks) = obj.get("checks") {
                    if let Some(n) = checks.as_i64() {
                        *files_checked = n as i32;
                    }
                }
            }
        }
    }

    fn parse_transfer_summary(
        &self,
        entry: &RcloneLogEntry,
        files_transferred: &mut i32,
        _bytes_transferred: &mut i64,
    ) {
        // Parse final summary: "Transferred: 5 / 5, 100%"
        if let Some(files_part) = entry.msg.split("Transferred:").nth(1) {
            if let Some(numbers_part) = files_part.split(',').next() {
                if let Some(total_files) = numbers_part.split('/').next() {
                    if let Ok(files) = total_files.trim().parse::<i32>() {
                        *files_transferred = files;
                    }
                }
            }
        }
    }

    #[cfg(test)]
    /// Mock version for testing - simulates successful rclone execution
    pub async fn mock_sync(
        &self,
        _job_id: Uuid,
        _source: &str,
        _destination: &str,
        mock_result: crate::models::RcloneExecutionResult,
    ) -> Result<crate::models::RcloneExecutionResult> {
        Ok(mock_result)
    }

    /// Check if rclone is installed and accessible
    pub async fn check_installation() -> Result<String> {
        let output = Command::new("rclone")
            .arg("version")
            .output()
            .await?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            let version_line = version.lines().next().unwrap_or("Unknown version");
            Ok(version_line.to_string())
        } else {
            Err(anyhow!("rclone not found or not working properly"))
        }
    }

    /// List available remotes
    pub async fn list_remotes() -> Result<Vec<String>> {
        let output = Command::new("rclone")
            .arg("listremotes")
            .output()
            .await?;

        if output.status.success() {
            let remotes_str = String::from_utf8_lossy(&output.stdout);
            let remotes: Vec<String> = remotes_str
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| line.trim_end_matches(':').to_string())
                .collect();
            Ok(remotes)
        } else {
            Err(anyhow!("Failed to list rclone remotes"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn create_test_rclone() -> RcloneWrapper {
        let temp_dir = TempDir::new().unwrap();
        RcloneWrapper::new(
            RcloneConfig::default(),
            Some(temp_dir.path().to_path_buf())
        )
    }

    #[test]
    fn test_rclone_config_default() {
        let config = RcloneConfig::default();
        assert_eq!(config.log_level, "INFO");
        assert_eq!(config.stats_interval, "10s");
        assert!(!config.dry_run);
        assert!(!config.verbose);
        assert_eq!(config.transfers, Some(4));
        assert_eq!(config.checkers, Some(8));
        assert!(config.extra_flags.is_empty());
    }

    #[test]
    fn test_rclone_config_custom() {
        let config = RcloneConfig {
            log_level: "DEBUG".to_string(),
            stats_interval: "5s".to_string(),
            dry_run: true,
            verbose: true,
            transfers: Some(8),
            checkers: Some(16),
            extra_flags: vec!["--fast-list".to_string()],
        };

        assert_eq!(config.log_level, "DEBUG");
        assert!(config.dry_run);
        assert!(config.verbose);
        assert_eq!(config.transfers, Some(8));
        assert_eq!(config.extra_flags, vec!["--fast-list"]);
    }

    #[tokio::test]
    async fn test_parse_info_log() {
        let rclone = create_test_rclone();
        let mut files_transferred = 0;
        let mut files_checked = 0;
        let mut bytes_transferred = 0i64;
        let mut transfer_rate_mbps = 0.0f32;

        // Test com log de progresso
        let progress_entry = RcloneLogEntry {
            level: "INFO".to_string(),
            msg: "Transferred: 123.45 MiB / 500 MiB, 25%, 12.34 MiB/s, ETA 30s".to_string(),
            time: "2025-08-03T10:00:00Z".to_string(),
            extra: std::collections::HashMap::new(),
        };

        rclone.parse_info_log(
            &progress_entry,
            &mut files_transferred,
            &mut files_checked,
            &mut bytes_transferred,
            &mut transfer_rate_mbps,
        );

        assert_eq!(transfer_rate_mbps, 12.34);
    }

    #[tokio::test]
    async fn test_parse_info_log_with_stats() {
        let rclone = create_test_rclone();
        let mut files_transferred = 0;
        let mut files_checked = 0;
        let mut bytes_transferred = 0i64;
        let mut transfer_rate_mbps = 0.0f32;

        // Test com dados de stats no extra
        let mut extra = std::collections::HashMap::new();
        extra.insert("stats".to_string(), json!({
            "transfers": 42,
            "bytes": 1048576,
            "checks": 100
        }));

        let stats_entry = RcloneLogEntry {
            level: "INFO".to_string(),
            msg: "Stats update".to_string(),
            time: "2025-08-03T10:00:00Z".to_string(),
            extra,
        };

        rclone.parse_info_log(
            &stats_entry,
            &mut files_transferred,
            &mut files_checked,
            &mut bytes_transferred,
            &mut transfer_rate_mbps,
        );

        assert_eq!(files_transferred, 42);
        assert_eq!(bytes_transferred, 1048576);
        assert_eq!(files_checked, 100);
    }

    #[tokio::test]
    async fn test_parse_transfer_summary() {
        let rclone = create_test_rclone();
        let mut files_transferred = 0;
        let mut bytes_transferred = 0i64;

        let summary_entry = RcloneLogEntry {
            level: "NOTICE".to_string(),
            msg: "Transferred: 15 / 20, 75%".to_string(),
            time: "2025-08-03T10:00:00Z".to_string(),
            extra: std::collections::HashMap::new(),
        };

        rclone.parse_transfer_summary(
            &summary_entry,
            &mut files_transferred,
            &mut bytes_transferred,
        );

        assert_eq!(files_transferred, 15);
    }

    #[tokio::test]
    async fn test_mock_sync() {
        let rclone = create_test_rclone();
        let job_id = Uuid::new_v4();

        let expected_result = crate::models::RcloneExecutionResult {
            exit_code: 0,
            files_transferred: 10,
            files_checked: 15,
            files_deleted: 0,
            bytes_transferred: 2048,
            transfer_rate_mbps: 5.5,
            duration_seconds: 30,
            error_count: 0,
            errors: vec![],
            stdout: "Success".to_string(),
            stderr: "".to_string(),
        };

        let result = rclone.mock_sync(
            job_id,
            "/test/source",
            "/test/dest",
            expected_result.clone()
        ).await;

        assert!(result.is_ok());
        let actual_result = result.unwrap();
        assert_eq!(actual_result.exit_code, 0);
        assert_eq!(actual_result.files_transferred, 10);
        assert_eq!(actual_result.bytes_transferred, 2048);
    }

    #[test]
    fn test_rclone_log_entry_parsing() {
        let log_line = r#"{"level":"INFO","msg":"Transferred: 5 files","time":"2025-08-03T10:00:00Z"}"#;
        
        let result: Result<RcloneLogEntry, _> = serde_json::from_str(log_line);
        assert!(result.is_ok());
        
        let entry = result.unwrap();
        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.msg, "Transferred: 5 files");
        assert_eq!(entry.time, "2025-08-03T10:00:00Z");
    }

    #[test]
    fn test_invalid_log_entry() {
        let invalid_log = r#"{"level":"INFO","msg":"Missing time field"}"#;
        
        let result: Result<RcloneLogEntry, _> = serde_json::from_str(invalid_log);
        // Deve falhar porque falta time
        assert!(result.is_err());
    }
}