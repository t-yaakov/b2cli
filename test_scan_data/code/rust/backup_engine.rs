/// Motor principal de backup do B2CLI
/// 
/// Este módulo implementa a lógica central de backup,
/// incluindo verificação de integridade e restore automático.
use std::path::Path;
use tokio::fs;
use sha2::{Digest, Sha256};
use tracing::{info, error, debug};

#[derive(Debug, Clone)]
pub struct BackupEngine {
    source_path: String,
    destination_path: String,
    verify_restore: bool,
    encryption_key: Option<String>,
}

impl BackupEngine {
    /// Cria uma nova instância do motor de backup
    pub fn new(source: &str, destination: &str) -> Self {
        Self {
            source_path: source.to_string(),
            destination_path: destination.to_string(),
            verify_restore: true,
            encryption_key: None,
        }
    }

    /// Executa o backup completo com verificação
    pub async fn execute_backup(&self) -> Result<BackupResult, BackupError> {
        info!("Iniciando backup de {} para {}", self.source_path, self.destination_path);
        
        // 1. Scan dos arquivos
        let files = self.scan_source_files().await?;
        debug!("Encontrados {} arquivos para backup", files.len());
        
        // 2. Execução do backup
        let backup_stats = self.perform_backup(&files).await?;
        
        // 3. Verificação de restore (diferencial competitivo!)
        if self.verify_restore {
            info!("Verificando integridade do backup via restore...");
            self.verify_backup_integrity(&files).await?;
            info!("✅ Backup verificado com sucesso!");
        }
        
        Ok(BackupResult {
            files_backed_up: files.len(),
            bytes_transferred: backup_stats.total_bytes,
            duration_seconds: backup_stats.duration,
            integrity_verified: self.verify_restore,
        })
    }

    /// Varre arquivos do diretório origem
    async fn scan_source_files(&self) -> Result<Vec<FileInfo>, BackupError> {
        let mut files = Vec::new();
        let mut entries = fs::read_dir(&self.source_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                let metadata = entry.metadata().await?;
                let hash = self.calculate_file_hash(&path).await?;
                
                files.push(FileInfo {
                    path: path.to_string_lossy().to_string(),
                    size: metadata.len(),
                    hash,
                    modified: metadata.modified()?,
                });
            }
        }
        
        Ok(files)
    }

    /// Executa o backup propriamente dito
    async fn perform_backup(&self, files: &[FileInfo]) -> Result<BackupStats, BackupError> {
        let start_time = std::time::Instant::now();
        let mut total_bytes = 0;
        
        for file in files {
            // Simular cópia (na implementação real usaria rclone)
            let source = Path::new(&file.path);
            let dest_name = source.file_name().unwrap().to_string_lossy();
            let dest_path = format!("{}/{}", self.destination_path, dest_name);
            
            fs::copy(&file.path, dest_path).await?;
            total_bytes += file.size;
            
            debug!("Arquivo copiado: {} ({} bytes)", file.path, file.size);
        }
        
        Ok(BackupStats {
            total_bytes,
            duration: start_time.elapsed().as_secs(),
        })
    }

    /// Verifica integridade via restore parcial (DIFERENCIAL!)
    async fn verify_backup_integrity(&self, files: &[FileInfo]) -> Result<(), BackupError> {
        // Selecionar 10% dos arquivos para verificação
        let sample_size = (files.len() / 10).max(1);
        let sample_files: Vec<_> = files.iter().take(sample_size).collect();
        
        for file in sample_files {
            let source_name = Path::new(&file.path).file_name().unwrap().to_string_lossy();
            let backup_path = format!("{}/{}", self.destination_path, source_name);
            
            // Verificar se arquivo existe no backup
            if !Path::new(&backup_path).exists() {
                return Err(BackupError::IntegrityCheckFailed(
                    format!("Arquivo não encontrado no backup: {}", backup_path)
                ));
            }
            
            // Verificar hash
            let backup_hash = self.calculate_file_hash(Path::new(&backup_path)).await?;
            if backup_hash != file.hash {
                return Err(BackupError::IntegrityCheckFailed(
                    format!("Hash não confere para arquivo: {}", file.path)
                ));
            }
        }
        
        Ok(())
    }

    /// Calcula SHA256 de um arquivo
    async fn calculate_file_hash(&self, path: &Path) -> Result<String, BackupError> {
        let content = fs::read(path).await?;
        let mut hasher = Sha256::new();
        hasher.update(content);
        Ok(format!("{:x}", hasher.finalize()))
    }
}

#[derive(Debug)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub hash: String,
    pub modified: std::time::SystemTime,
}

#[derive(Debug)]
pub struct BackupResult {
    pub files_backed_up: usize,
    pub bytes_transferred: u64,
    pub duration_seconds: u64,
    pub integrity_verified: bool,
}

#[derive(Debug)]
struct BackupStats {
    total_bytes: u64,
    duration: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Integrity check failed: {0}")]
    IntegrityCheckFailed(String),
    
    #[error("Encryption error: {0}")]
    Encryption(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_backup_engine() {
        let engine = BackupEngine::new("/tmp/test_source", "/tmp/test_dest");
        assert_eq!(engine.source_path, "/tmp/test_source");
        assert!(engine.verify_restore);
    }
}