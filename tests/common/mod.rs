// tests/common/mod.rs
// Helpers compartilhados entre testes

use sqlx::{PgPool, Row};
use std::sync::atomic::{AtomicU32, Ordering};
use tempfile::TempDir;
use std::path::PathBuf;
use std::fs;
use uuid::Uuid;
use serde_json::json;
use b2cli::{models::BackupJob, AppState};
use axum::Router;
use std::sync::Arc;
use tokio_cron_scheduler::JobScheduler;

// Contador para garantir DBs únicos
static DB_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Setup de banco de dados para testes
pub struct TestDatabase {
    pub pool: PgPool,
    pub db_name: String,
}

impl TestDatabase {
    pub async fn new() -> Self {
        let db_id = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_name = format!("b2cli_test_{}", db_id);
        
        // Conectar ao postgres para criar o DB de teste
        let admin_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/postgres".to_string());
        
        let admin_pool = PgPool::connect(&admin_url).await
            .expect("Failed to connect to postgres");
        
        // Criar database de teste
        sqlx::query(&format!("CREATE DATABASE {}", db_name))
            .execute(&admin_pool)
            .await
            .expect("Failed to create test database");
        
        admin_pool.close().await;
        
        // Conectar ao DB de teste
        let test_url = admin_url.replace("/postgres", &format!("/{}", db_name));
        let pool = PgPool::connect(&test_url).await
            .expect("Failed to connect to test database");
        
        // Rodar migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");
        
        Self { pool, db_name }
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        // Cleanup será feito async em production
        // Por enquanto deixamos o DB para debug
    }
}

/// Fixtures para arquivos de teste
pub struct TestFixtures {
    pub temp_dir: TempDir,
    pub source_dir: PathBuf,
    pub backup_dir: PathBuf,
}

impl TestFixtures {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let source_dir = temp_dir.path().join("source");
        let backup_dir = temp_dir.path().join("backup");
        
        fs::create_dir_all(&source_dir).expect("Failed to create source dir");
        fs::create_dir_all(&backup_dir).expect("Failed to create backup dir");
        
        Self {
            temp_dir,
            source_dir,
            backup_dir,
        }
    }
    
    /// Criar arquivo de teste com conteúdo conhecido
    pub fn create_test_file(&self, name: &str, content: &str) -> PathBuf {
        let file_path = self.source_dir.join(name);
        fs::write(&file_path, content).expect("Failed to write test file");
        file_path
    }
    
    /// Criar estrutura de diretórios de teste
    pub fn create_test_structure(&self) {
        self.create_test_file("document.txt", "Important document content");
        self.create_test_file("config.json", r#"{"key": "value", "number": 42}"#);
        
        let subdir = self.source_dir.join("subdir");
        fs::create_dir_all(&subdir).expect("Failed to create subdir");
        fs::write(subdir.join("nested.md"), "# Nested File\nContent here")
            .expect("Failed to write nested file");
    }
    
    /// Criar arquivo binário de teste
    pub fn create_binary_file(&self, name: &str, size_kb: usize) -> PathBuf {
        let file_path = self.source_dir.join(name);
        let content = vec![0x42; size_kb * 1024]; // Preencher com bytes 0x42
        fs::write(&file_path, content).expect("Failed to write binary file");
        file_path
    }
}

/// Mock do RcloneWrapper para testes
pub struct MockRclone {
    pub should_fail: bool,
    pub files_transferred: i32,
    pub bytes_transferred: i64,
    pub duration_seconds: i32,
}

impl Default for MockRclone {
    fn default() -> Self {
        Self {
            should_fail: false,
            files_transferred: 5,
            bytes_transferred: 1024 * 1024, // 1MB
            duration_seconds: 10,
        }
    }
}

impl MockRclone {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }
    
    pub fn with_files(mut self, count: i32) -> Self {
        self.files_transferred = count;
        self
    }
}

/// Criar BackupJob para testes
pub fn create_test_backup_job(name: &str, source: &str, destinations: Vec<&str>) -> BackupJob {
    let mappings = json!({
        source: destinations
    });
    
    BackupJob {
        id: Uuid::new_v4(),
        name: name.to_string(),
        mappings,
        status: "PENDING".to_string(),
        is_active: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        deleted_at: None,
    }
}

/// Verificar se dois arquivos são idênticos
pub fn files_are_identical(path1: &PathBuf, path2: &PathBuf) -> bool {
    if !path1.exists() || !path2.exists() {
        return false;
    }
    
    let content1 = fs::read(path1).unwrap_or_default();
    let content2 = fs::read(path2).unwrap_or_default();
    
    content1 == content2
}

/// Contar arquivos em um diretório recursivamente
pub fn count_files_recursive(dir: &PathBuf) -> usize {
    if !dir.exists() {
        return 0;
    }
    
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                1
            } else if path.is_dir() {
                count_files_recursive(&path)
            } else {
                0
            }
        })
        .sum()
}

/// Cria um scheduler para testes
pub async fn create_test_scheduler() -> JobScheduler {
    JobScheduler::new().await.expect("Failed to create test scheduler")
}

/// Cria uma aplicação Axum para testes com todas as rotas
pub fn create_test_app(app_state: AppState) -> Router {
    use axum::routing::{get, post};
    use b2cli::routes::providers::*;
    
    Router::new()
        .route("/providers", get(list_providers).post(create_provider))
        .route("/providers/types", get(list_provider_types))
        .route(
            "/providers/{id}",
            get(get_provider)
                .put(update_provider)
                .delete(delete_provider),
        )
        .route("/providers/{id}/test", post(test_provider_connectivity))
        .with_state(app_state)
}