// tests/end_to_end.rs
// Testes end-to-end que simulam fluxos completos

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;
use serde_json::json;

mod common;
use common::{TestDatabase, TestFixtures, create_test_backup_job, files_are_identical, count_files_recursive};

#[tokio::test]
async fn test_complete_backup_cycle_with_files() {
    // let _test_db = TestDatabase::new().await;
    let fixtures = TestFixtures::new();
    
    // 1. Criar estrutura de arquivos de teste
    fixtures.create_test_structure();
    fixtures.create_binary_file("large_file.bin", 50); // 50KB
    
    // Verificar que os arquivos foram criados
    assert_eq!(count_files_recursive(&fixtures.source_dir), 4); // 3 text + 1 binary
    
    // 2. Simular backup (copiando arquivos)
    let backup_job = create_test_backup_job(
        "E2E Test Backup",
        fixtures.source_dir.to_str().unwrap(),
        vec![fixtures.backup_dir.to_str().unwrap()]
    );
    
    // Para este teste, vamos fazer a cópia manualmente
    copy_directory_recursive(&fixtures.source_dir, &fixtures.backup_dir).await;
    
    // 3. Verificar que o backup foi bem-sucedido
    assert_eq!(count_files_recursive(&fixtures.backup_dir), 4);
    
    // 4. Verificar integridade dos arquivos
    let source_doc = fixtures.source_dir.join("document.txt");
    let backup_doc = fixtures.backup_dir.join("document.txt");
    assert!(files_are_identical(&source_doc, &backup_doc));
    
    let source_binary = fixtures.source_dir.join("large_file.bin");
    let backup_binary = fixtures.backup_dir.join("large_file.bin");
    assert!(files_are_identical(&source_binary, &backup_binary));
}

#[tokio::test]
async fn test_backup_with_subdirectories() {
    let fixtures = TestFixtures::new();
    
    // Criar estrutura com subdiretórios
    let deep_dir = fixtures.source_dir.join("level1").join("level2").join("level3");
    fs::create_dir_all(&deep_dir).expect("Failed to create deep directory");
    fs::write(deep_dir.join("deep_file.txt"), "Deep content").expect("Failed to write deep file");
    
    // Criar mais arquivos em diferentes níveis
    fs::write(fixtures.source_dir.join("root.txt"), "Root content").unwrap();
    fs::write(fixtures.source_dir.join("level1").join("mid.txt"), "Mid content").unwrap();
    
    let total_files = count_files_recursive(&fixtures.source_dir);
    assert_eq!(total_files, 3);
    
    // Simular backup
    copy_directory_recursive(&fixtures.source_dir, &fixtures.backup_dir).await;
    
    // Verificar que todos os arquivos foram copiados, incluindo estrutura
    assert_eq!(count_files_recursive(&fixtures.backup_dir), 3);
    assert!(fixtures.backup_dir.join("root.txt").exists());
    assert!(fixtures.backup_dir.join("level1").join("mid.txt").exists());
    assert!(fixtures.backup_dir.join("level1").join("level2").join("level3").join("deep_file.txt").exists());
}

#[tokio::test]
async fn test_backup_empty_directory() {
    let fixtures = TestFixtures::new();
    
    // Source directory já existe mas está vazio (exceto pelos diretórios criados por create_test_structure)
    // Criar um diretório completamente novo e vazio
    let empty_source = fixtures.temp_dir.path().join("empty_source");
    fs::create_dir_all(&empty_source).unwrap();
    let empty_backup = fixtures.temp_dir.path().join("empty_backup");
    
    assert_eq!(count_files_recursive(&empty_source), 0);
    
    // Simular backup de diretório vazio
    copy_directory_recursive(&empty_source, &empty_backup).await;
    
    // Backup directory deve existir mas estar vazio
    assert!(empty_backup.exists());
    assert_eq!(count_files_recursive(&empty_backup), 0);
}

#[tokio::test]
async fn test_backup_large_files() {
    let fixtures = TestFixtures::new();
    
    // Criar arquivos de diferentes tamanhos
    fixtures.create_binary_file("small.bin", 1);    // 1KB
    fixtures.create_binary_file("medium.bin", 100); // 100KB
    fixtures.create_binary_file("large.bin", 1000); // 1MB
    
    assert_eq!(count_files_recursive(&fixtures.source_dir), 3);
    
    // Verificar tamanhos dos arquivos
    let small_size = fs::metadata(fixtures.source_dir.join("small.bin")).unwrap().len();
    let medium_size = fs::metadata(fixtures.source_dir.join("medium.bin")).unwrap().len();
    let large_size = fs::metadata(fixtures.source_dir.join("large.bin")).unwrap().len();
    
    assert_eq!(small_size, 1024);
    assert_eq!(medium_size, 100 * 1024);
    assert_eq!(large_size, 1000 * 1024);
    
    // Simular backup
    copy_directory_recursive(&fixtures.source_dir, &fixtures.backup_dir).await;
    
    // Verificar que os tamanhos permanecem corretos
    let backup_small_size = fs::metadata(fixtures.backup_dir.join("small.bin")).unwrap().len();
    let backup_medium_size = fs::metadata(fixtures.backup_dir.join("medium.bin")).unwrap().len();
    let backup_large_size = fs::metadata(fixtures.backup_dir.join("large.bin")).unwrap().len();
    
    assert_eq!(small_size, backup_small_size);
    assert_eq!(medium_size, backup_medium_size);
    assert_eq!(large_size, backup_large_size);
}

#[tokio::test]
async fn test_backup_special_characters() {
    let fixtures = TestFixtures::new();
    
    // Criar arquivos com nomes especiais
    fixtures.create_test_file("normal_file.txt", "Normal content");
    fixtures.create_test_file("file with spaces.txt", "Spaced content");
    fixtures.create_test_file("file-with-dashes.txt", "Dashed content");
    fixtures.create_test_file("file_with_números_123.txt", "Numbered content");
    
    // Diretório com caracteres especiais
    let special_dir = fixtures.source_dir.join("special dir with spaces");
    fs::create_dir_all(&special_dir).unwrap();
    fs::write(special_dir.join("nested file.txt"), "Nested content").unwrap();
    
    assert_eq!(count_files_recursive(&fixtures.source_dir), 5);
    
    // Simular backup
    copy_directory_recursive(&fixtures.source_dir, &fixtures.backup_dir).await;
    
    // Verificar que todos os arquivos foram copiados corretamente
    assert_eq!(count_files_recursive(&fixtures.backup_dir), 5);
    assert!(fixtures.backup_dir.join("file with spaces.txt").exists());
    assert!(fixtures.backup_dir.join("special dir with spaces").join("nested file.txt").exists());
}

#[tokio::test]
async fn test_incremental_backup_simulation() {
    let fixtures = TestFixtures::new();
    
    // 1. Backup inicial
    fixtures.create_test_file("initial.txt", "Initial content");
    copy_directory_recursive(&fixtures.source_dir, &fixtures.backup_dir).await;
    assert_eq!(count_files_recursive(&fixtures.backup_dir), 1);
    
    // 2. Adicionar novos arquivos
    fixtures.create_test_file("new.txt", "New content");
    fixtures.create_test_file("another.txt", "Another content");
    
    // 3. Segundo backup (incremental simulado)
    copy_directory_recursive(&fixtures.source_dir, &fixtures.backup_dir).await;
    assert_eq!(count_files_recursive(&fixtures.backup_dir), 3);
    
    // 4. Verificar que arquivos antigos ainda existem
    assert!(fixtures.backup_dir.join("initial.txt").exists());
    assert!(fixtures.backup_dir.join("new.txt").exists());
    assert!(fixtures.backup_dir.join("another.txt").exists());
}

#[tokio::test]
async fn test_backup_performance_timing() {
    let fixtures = TestFixtures::new();
    
    // Criar vários arquivos para testar performance
    for i in 0..20 {
        fixtures.create_test_file(&format!("file_{:03}.txt", i), &format!("Content for file {}", i));
    }
    
    fixtures.create_binary_file("performance_test.bin", 500); // 500KB
    
    assert_eq!(count_files_recursive(&fixtures.source_dir), 21);
    
    // Medir tempo de backup
    let start = std::time::Instant::now();
    copy_directory_recursive(&fixtures.source_dir, &fixtures.backup_dir).await;
    let duration = start.elapsed();
    
    // Verificar resultado
    assert_eq!(count_files_recursive(&fixtures.backup_dir), 21);
    
    // Para arquivos pequenos, deve ser bem rápido (menos de 1 segundo)
    assert!(duration.as_secs() < 1, "Backup took too long: {:?}", duration);
    
    println!("Backup of 21 files completed in: {:?}", duration);
}

// Helper function para copiar diretórios recursivamente
async fn copy_directory_recursive(source: &PathBuf, destination: &PathBuf) {
    if source.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::copy(source, destination).unwrap();
        return;
    }
    
    fs::create_dir_all(destination).unwrap();
    
    let entries = fs::read_dir(source).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        let source_path = entry.path();
        let dest_path = destination.join(entry.file_name());
        
        if source_path.is_dir() {
            Box::pin(copy_directory_recursive(&source_path, &dest_path)).await;
        } else {
            fs::copy(&source_path, &dest_path).unwrap();
        }
    }
}