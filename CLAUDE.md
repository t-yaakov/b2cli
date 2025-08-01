# CLAUDE.md - Instruções para Desenvolvimento

Este arquivo contém instruções específicas para assistentes de IA trabalhando no projeto B2CLI.

## Contexto do Projeto

B2CLI é uma plataforma de backup e gestão de dados construída em Rust que evolui de backups locais simples para uma solução inteligente de gestão de dados na nuvem.

### Arquitetura Atual

- **API REST** com Axum framework
- **PostgreSQL** como banco de dados principal
- **SQLx** para queries type-safe
- **Tracing** para logs estruturados
- **OpenAPI/Swagger** para documentação automática

## Estado Atual (Milestone 1 - CONCLUÍDO)

### ✅ Funcionalidades Implementadas

1. **API REST Completa**:
   - CRUD para backup jobs
   - Soft delete com campo `is_active`
   - Status tracking (PENDING, RUNNING, COMPLETED, FAILED)
   - Documentação automática via Swagger

2. **Sistema de Backup Local**:
   - Cópia de arquivos locais
   - Catalogação automática com metadados
   - Cálculo de checksum SHA256
   - Suporte a múltiplos destinos

3. **Sistema de Logs**:
   - Logs estruturados em JSON
   - Rotação diária automática
   - Níveis diferentes para console (INFO+) e arquivo (DEBUG+)

4. **Qualidade e Robustez**:
   - Tratamento de erros robusto
   - Health checks
   - Migrations de banco de dados
   - Soft delete para auditoria

### 🗂️ Estrutura de Arquivos

```
src/
├── main.rs           # Entry point e configuração do servidor
├── models.rs         # Structs e tipos de dados
├── db.rs            # Funções de acesso ao banco
├── backup_worker.rs  # Lógica principal de backup
├── logging.rs       # Configuração do sistema de logs
└── routes/          # HTTP handlers
    ├── mod.rs
    ├── health.rs
    ├── readiness.rs
    └── backups.rs
```

## Comandos Essenciais

### Desenvolvimento
```bash
# Executar com reload automático
cargo watch -x run

# Executar migrations
sqlx migrate run

# Verificar código
cargo clippy -- -D warnings
cargo fmt -- --check

# Executar testes
cargo test
```

### Logs
```bash
# Console com mais detalhes
RUST_LOG=debug cargo run

# Arquivo com trace completo
FILE_LOG=trace cargo run
```

## Banco de Dados

### Principais Tabelas

**backup_jobs**:
- `id` (UUID, PK)
- `name` (VARCHAR)
- `mappings` (JSONB) - formato: `{"origem": ["destino1", "destino2"]}`
- `status` (VARCHAR) - PENDING/RUNNING/COMPLETED/FAILED
- `is_active` (BOOLEAN) - para soft delete
- `created_at`, `updated_at`, `deleted_at` (TIMESTAMP)

**backed_up_files**:
- `id` (UUID, PK)
- `backup_job_id` (UUID, FK)
- `original_path`, `backed_up_path` (VARCHAR)
- `file_name`, `file_extension` (VARCHAR)
- `file_size` (BIGINT)
- `checksum` (VARCHAR) - SHA256
- `backed_up_at` (TIMESTAMP)

## Padrões de Código

### 1. Error Handling
Usar o tipo `AppError` personalizado que implementa `IntoResponse`:

```rust
pub enum AppError {
    SqlxError(sqlx::Error),
    IoError(std::io::Error),
    NotFound(String),
    InternalServerError(String),
}
```

### 2. Handlers HTTP
Sempre retornar `Result<impl IntoResponse, AppError>`:

```rust
pub async fn handler(
    State(state): State<AppState>,
    Json(payload): Json<RequestType>,
) -> Result<impl IntoResponse, AppError> {
    // lógica aqui
    Ok((StatusCode::OK, Json(response)))
}
```

### 3. Logs Estruturados
Usar tracing com campos estruturados:

```rust
tracing::info!(
    job_id = %job.id, 
    job_name = %job.name, 
    "Backup job completed successfully"
);
```

### 4. Migrations
Sempre criar migrations para mudanças no schema:

```sql
-- migrations/YYYYMMDDHHMMSS_description.sql
ALTER TABLE table_name ADD COLUMN new_column TYPE;
```

## Próximos Passos (Milestone 2)

### Prioridades
1. **Integração Rclone**: Substituir cópia local por rclone
2. **Cloud Storage**: Suporte a S3, B2, Google Drive, etc
3. **Configuration Management**: Sistema para gerenciar remotes
4. **Progress Tracking**: Acompanhar progresso de uploads

### Tarefas Pendentes
- [ ] Detectar instalação do rclone
- [ ] Wrapper para comandos rclone
- [ ] Parser de output para progresso
- [ ] Endpoint para configurar remotes
- [ ] Criptografia de credenciais

## Debugging

### Problemas Comuns

1. **Erro "Is a directory"**: 
   - Verificar se `strip_prefix` não retorna caminho vazio
   - Garantir que destinos sejam arquivos, não diretórios

2. **Migrations falhando**:
   - Verificar se DATABASE_URL está correto
   - Executar `sqlx migrate run` antes de compilar

3. **Logs não aparecendo**:
   - Verificar níveis de log (console vs arquivo)
   - Conferir se pasta `logs/` foi criada

### Environment Variables

```bash
DATABASE_URL=postgresql://user:pass@localhost/b2cli
RUST_LOG=info                # Console log level
FILE_LOG=debug               # File log level
```

## Testes

### Estrutura de Testes
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_function() {
        // setup
        // action
        // assertion
    }
}
```

### Executar Testes
```bash
# Todos os testes
cargo test

# Testes específicos
cargo test test_name

# Com output
cargo test -- --nocapture
```

## Documentação

### OpenAPI/Swagger
A documentação é gerada automaticamente via `utoipa`. Para adicionar novos endpoints:

1. Adicionar `#[utoipa::path(...)]` no handler
2. Incluir na struct `ApiDoc` no `main.rs`
3. Adicionar modelos na seção `components`

### Exemplo de Documentação
```rust
#[utoipa::path(
    post,
    path = "/backups",
    tag = "Backups",
    request_body = NewBackupJob,
    responses(
        (status = 201, description = "Created", body = BackupJob),
        (status = 500, description = "Error", body = ErrorResponse)
    )
)]
pub async fn create_backup(...) -> Result<impl IntoResponse, AppError> {
    // implementação
}
```

## Notas para IAs

### Quando Trabalhar no Projeto

1. **Sempre ler este arquivo primeiro** para entender o contexto
2. **Verificar o ROADMAP.md** para entender prioridades
3. **Usar o sistema de todos** para trackear progresso
4. **Executar testes** após mudanças significativas
5. **Atualizar documentação** quando necessário

### Padrões de Qualidade

- **Nunca quebrar a API** sem versioning
- **Sempre usar transactions** para operações críticas
- **Logs estruturados** para debug
- **Error handling** robusto
- **Testes** para novas funcionalidades

### Comandos Úteis para Debug

```bash
# Ver logs em tempo real
tail -f logs/b2cli.log | jq .

# Verificar banco
psql $DATABASE_URL -c "SELECT * FROM backup_jobs;"

# Status do projeto
git status
cargo check
```

---

**Última atualização**: Agosto 2025  
**Status**: Milestone 1 Completo ✅  
**Próximo**: Milestone 2 - Integração Rclone