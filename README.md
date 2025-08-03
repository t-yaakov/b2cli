# B2CLI - Plataforma de Backup e Gestão de Dados

B2CLI é uma plataforma de backup inteligente que evolui de uma simples ferramenta de cópia para um sistema completo de gestão de dados. Construído com Rust, oferece performance, segurança e flexibilidade para pequenas empresas e uso pessoal.

## 🚀 Características Principais

### ✅ Funcionalidades Atuais (Milestone 2 Concluído)

- **API REST completa** para gerenciamento de backups
- **Integração Rclone** para backup em nuvem (40+ provedores)
- **Sistema de agendamento** com cron expressions
- **Logs detalhados** de execução com métricas (arquivos, bytes, duração)
- **Arquivamento inteligente** de logs (Hot/Warm/Cold storage)
- **Soft delete** para segurança dos dados
- **Cleanup automático** de arquivos temporários
- **Documentação interativa** via Swagger UI e Redoc

### 🔄 Próximo: Restore-First Design (Milestone 3)
- Verificação automática de restore após backup
- Dashboard de confiabilidade: "98% dos seus backups são restauráveis"
- Sistema .b2ignore para exclusão de arquivos
- Configuração via TOML (Infrastructure as Code)

## 🛠️ Tecnologias

- **Rust** - Linguagem principal
- **Axum** - Framework web async
- **PostgreSQL** - Banco de dados
- **SQLx** - ORM com verificação em tempo de compilação
- **Rclone** - Backup para 40+ provedores cloud
- **tokio-cron-scheduler** - Agendamento robusto
- **Tracing** - Sistema de logs estruturado
- **OpenAPI** - Documentação automática da API

## 📋 Pré-requisitos

- **Rust 1.70+** e Cargo - [Instruções de Instalação](https://www.rust-lang.org/tools/install)
- **PostgreSQL 14+** - Banco de dados
- **Rclone** - Para backup em nuvem ([Download](https://rclone.org/downloads/))
- **Docker** (opcional) - Para executar o PostgreSQL via container

## 🔧 Instalação

### 1. Clone o repositório

```bash
git clone https://github.com/seu-usuario/b2cli.git
cd b2cli
```

### 2. Instale as ferramentas de desenvolvimento

```bash
cargo install sqlx-cli
```

### 3. Configure o ambiente

```bash
cp .env.example .env
# Edite .env com suas configurações de banco de dados
```

### 4. Prepare o banco de dados

#### Opção A: Usando Docker (Recomendado)

```bash
docker-compose up -d
```

#### Opção B: Usando PostgreSQL existente

Configure a `DATABASE_URL` no arquivo `.env` para apontar para seu banco.

### 5. Execute as migrations

```bash
sqlx migrate run
```

### 6. Compile e execute

```bash
cargo run
```

## 🎯 Uso Rápido

### API Endpoints

A API estará disponível em `http://localhost:3000`

- **Swagger UI**: http://localhost:3000/swagger-ui
- **Redoc**: http://localhost:3000/redoc

### Principais Endpoints

#### Backup Jobs
- `POST /backups` - Criar nova tarefa de backup
- `GET /backups` - Listar tarefas de backup ativas
- `GET /backups/{id}` - Obter detalhes de uma tarefa
- `PUT /backups/{id}` - Atualizar uma tarefa
- `DELETE /backups/{id}` - Deletar uma tarefa (soft delete)
- `POST /backups/{id}/run` - Executar um backup manualmente

#### Schedules (Agendamento)
- `POST /backups/{id}/schedule` - Criar agendamento para um backup
- `GET /backups/{id}/schedule` - Obter agendamento do backup
- `PUT /backups/{id}/schedule` - Atualizar agendamento
- `DELETE /backups/{id}/schedule` - Remover agendamento
- `GET /schedules` - Listar todos os agendamentos

#### Logs de Execução
- `GET /logs` - Listar logs de execução
- `GET /logs/{id}` - Obter detalhes de um log
- `GET /backups/{id}/logs` - Logs de execução de um backup específico
- `GET /logs/stats` - Estatísticas dos logs

#### Sistema de Arquivamento
- `GET /archive/status` - Status do sistema de arquivamento
- `GET /archive/policy` - Política de retenção atual
- `PUT /archive/policy` - Atualizar política de retenção

### Exemplo de Criação de Backup

```json
POST /backups
{
  "name": "Backup para Google Drive",
  "mappings": {
    "/home/user/Documents": ["gdrive:backups/docs"],
    "/home/user/Projects": ["gdrive:backups/projects"]
  }
}
```

### Exemplo de Agendamento

```json
POST /backups/{id}/schedule
{
  "name": "Backup semanal - Domingo 10h",
  "cron_expression": "0 0 10 * * 0",
  "enabled": true
}
```

**Nota**: Cron expressions usam 6 campos: `segundo minuto hora dia mês dia_semana`

## 📁 Estrutura do Projeto

```
b2cli/
├── src/
│   ├── main.rs           # Entry point e configuração do servidor
│   ├── lib.rs           # Módulos e error handling
│   ├── models.rs        # Structs e tipos de dados
│   ├── db.rs           # Funções de acesso ao banco
│   ├── backup_worker.rs # Lógica principal de backup
│   ├── logging.rs      # Configuração do sistema de logs
│   ├── scheduler.rs    # Criação do scheduler
│   ├── rclone.rs       # Wrapper para comandos rclone
│   ├── archiver.rs     # Sistema de arquivamento de logs
│   └── routes/         # HTTP handlers
│       ├── mod.rs
│       ├── health.rs
│       ├── readiness.rs
│       ├── backups.rs
│       ├── logs.rs
│       └── archive.rs
├── migrations/          # Migrations SQL
├── logs/               # Arquivos de log (gerado automaticamente)
├── docs/               # Documentação do projeto
└── Cargo.toml          # Configuração do projeto
```

## 🔍 Sistema de Logs

O B2CLI possui um sistema de logs em dois níveis:

- **Console**: Logs INFO e acima (menos verboso)
- **Arquivo**: Todos os logs em formato JSON (`logs/b2cli.log`)

### Configuração de Logs

```bash
# Nível de log do console (padrão: info)
RUST_LOG=debug cargo run

# Nível de log do arquivo (padrão: debug)
FILE_LOG=trace cargo run
```

### Rotação de Logs

Os logs são rotacionados diariamente automaticamente. Arquivos antigos são mantidos com o padrão `b2cli.YYYY-MM-DD.log`.

## 🗄️ Banco de Dados

### Principais Tabelas

- `backup_jobs` - Jobs de backup configurados
  - `id` - UUID único
  - `name` - Nome do backup
  - `mappings` - JSON com origem -> destinos
  - `status` - PENDING, RUNNING, COMPLETED, FAILED
  - `is_active` - Soft delete flag

- `backup_schedules` - Agendamentos dos backups
  - `id` - UUID único
  - `backup_job_id` - Referência ao job
  - `name` - Nome do agendamento
  - `cron_expression` - Expressão cron (6 campos)
  - `enabled` - Se está ativo
  - `last_run`, `last_status` - Última execução
  
- `backup_execution_logs` - Logs detalhados de execução
  - `id` - UUID único
  - `backup_job_id` - Referência ao job
  - `schedule_id` - Referência ao schedule (nullable)
  - `status` - running/completed/failed
  - `files_transferred`, `bytes_transferred` - Métricas
  - `duration_seconds` - Tempo de execução
  - `rclone_command` - Comando executado
  - `triggered_by` - manual/scheduler

### Soft Delete

Os backups deletados não são removidos do banco, apenas marcados como inativos (`is_active = false`). Isso permite recuperação e auditoria.

## 🚧 Desenvolvimento

### Executar em modo desenvolvimento

```bash
cargo watch -x run
```

### Executar testes

```bash
# Bateria completa de testes (21 testes)
cargo test --lib --test end_to_end

# Verificar que tudo funciona
cargo test -- --nocapture

# Ver guia completo
cat docs/TESTING_GUIDE.md
```

### Verificar código

```bash
cargo clippy -- -D warnings
cargo fmt -- --check
```

### Compilar para produção

```bash
cargo build --release
```

## 🔐 Segurança

- Todas as senhas e dados sensíveis devem estar no `.env`
- Os backups suportarão criptografia end-to-end (em desenvolvimento)
- Logs não contêm informações sensíveis
- API preparada para autenticação (em desenvolvimento)

## 📊 Monitoramento

- Logs estruturados em JSON facilitam integração com ferramentas de monitoramento
- Health check disponível em `/health`
- Readiness check em `/readiness` verifica conexão com banco
- Sistema de arquivamento com políticas Hot/Warm/Cold
- Métricas detalhadas de execução em `/logs/stats`

## 🚀 Roadmap

Veja [ROADMAP.md](ROADMAP.md) para o progresso detalhado e próximos passos.

### Status Atual: Milestone 2 ✅ Concluído
- ✅ Integração Rclone para backup em nuvem  
- ✅ Sistema de agendamento robusto
- ✅ Logs detalhados com métricas
- ✅ Sistema de arquivamento inteligente

### Próximo: Milestone 3 🔄 Restore-First Design
- 🔄 Verificação automática de restore após backup
- 🔄 Dashboard de confiabilidade 
- 🔄 Sistema .b2ignore para exclusão de arquivos
- 🔄 Configuração via TOML (Infrastructure as Code)

## 📄 Licença

Este projeto está sob a licença MIT. Veja o arquivo [LICENSE](LICENSE) para mais detalhes.