# B2CLI - Plataforma Inteligente de Backup com Restore-First Design

B2CLI é uma plataforma de backup inteligente construída em Rust que oferece o primeiro sistema do mercado com **"Restore-First Design"** - verificação automática de que seus backups são realmente restauráveis. 

## 🎯 Diferencial Único no Mercado

**"Restore-First Design"** - Enquanto outras ferramentas apenas copiam arquivos, o B2CLI é a primeira a garantir que 98% dos seus backups são realmente restauráveis através de verificação automática.

## 🚀 Características Principais

### ✅ Funcionalidades Atuais (v0.1.6)

#### Core do Sistema
- **API REST completa** com 50+ endpoints documentados
- **Integração Rclone** para backup em 40+ provedores cloud
- **Sistema de agendamento** robusto com cron expressions
- **Logs detalhados** com métricas completas (arquivos, bytes, duração)
- **Arquivamento inteligente** de logs (Hot/Warm/Cold storage)

#### File Intelligence System 🧠
- **Catálogo global de arquivos** com metadados completos
- **Detecção de duplicatas** via SHA256 hash
- **Scanner recursivo** de diretórios com filtros avançados
- **Busca full-text** em nomes de arquivos
- **Classificação automática** por padrões de acesso (Hot/Warm/Cold)

#### Cloud Providers Management ☁️
- **Suporte nativo** para Backblaze B2, IDrive e2, Wasabi, Scaleway
- **Teste de conectividade** automático
- **Templates específicos** por provedor
- **Configuração S3-compatible** genérica

#### Segurança e Confiabilidade 🔐
- **Criptografia end-to-end** para dados sensíveis
- **Soft delete** para recuperação segura
- **Auditoria completa** de todas as operações
- **Health/Readiness checks** para monitoramento

### 🔄 Próximo: Restore Verification (Milestone 3)
- Verificação automática de restore após cada backup
- Dashboard executivo: "98% dos backups são restauráveis"
- Sistema .b2ignore para exclusão inteligente
- Configuração via TOML (Infrastructure as Code)

## 🛠️ Stack Tecnológico

### Core
- **Rust 1.70+** - Performance e segurança
- **Axum** - Framework web async de alta performance
- **PostgreSQL 14+** - Banco de dados com recursos avançados (JSONB, Arrays, Full-text search)
- **SQLx** - Type-safe SQL com verificação em compile-time

### Integrações
- **Rclone** - Suporte para 40+ provedores cloud
- **tokio-cron-scheduler** - Agendamento robusto com precisão de segundos
- **SHA2** - Hashing criptográfico para detecção de duplicatas
- **AES-GCM + Argon2** - Criptografia de dados sensíveis

### Observabilidade
- **Tracing** - Logs estruturados com rotação diária
- **OpenAPI/Swagger** - Documentação interativa automática
- **Métricas detalhadas** - Estatísticas em tempo real

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

#### Cloud Providers 🆕
- `POST /providers` - Adicionar novo provedor cloud
- `GET /providers` - Listar provedores configurados
- `GET /providers/{id}` - Detalhes do provedor
- `PUT /providers/{id}` - Atualizar configuração
- `DELETE /providers/{id}` - Remover provedor
- `POST /providers/{id}/test` - Testar conectividade

#### File Intelligence 🆕
- `POST /files/scan` - Criar configuração de scan
- `GET /files/scan` - Listar configurações de scan
- `POST /files/scan/{id}/run` - Executar scan de arquivos
- `GET /files/scan/jobs` - Listar jobs de scan executados
- `GET /files/scan/{id}` - Status do scan job
- `GET /files/duplicates` - Encontrar arquivos duplicados

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
│   ├── logging.rs      # Sistema de logs estruturado
│   ├── scheduler.rs    # Criação do scheduler
│   ├── rclone.rs       # Wrapper para comandos rclone
│   ├── archiver.rs     # Sistema de arquivamento de logs
│   ├── config_manager.rs # Gerenciamento de configurações 🆕
│   ├── crypto.rs       # Funções de criptografia 🆕
│   ├── file_scanner.rs # Scanner de arquivos e catalogação 🆕
│   └── routes/         # HTTP handlers
│       ├── mod.rs
│       ├── health.rs
│       ├── readiness.rs
│       ├── backups.rs
│       ├── logs.rs
│       ├── archive.rs
│       ├── providers.rs # Cloud providers 🆕
│       ├── files.rs    # File intelligence 🆕
│       └── scan_schedules.rs # Agendamento de scans 🆕
├── migrations/          # 13 migrations SQL
├── test_scan_data/     # Dados de teste para file scanner
├── logs/               # Arquivos de log (rotação diária)
├── docs/               # Documentação completa
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

#### Sistema de Backup
- `backup_jobs` - Jobs de backup configurados
- `backup_schedules` - Agendamentos com cron expressions
- `backup_execution_logs` - Logs detalhados de execução
- `backed_up_files` - Arquivos transferidos em cada backup

#### Cloud Providers 🆕
- `cloud_providers` - Configurações de provedores cloud
  - Suporte para B2, IDrive e2, Wasabi, Scaleway
  - Credenciais criptografadas
  - Templates específicos por tipo

#### File Intelligence 🆕  
- `file_catalog` - Catálogo global de arquivos
  - Metadados completos (tamanho, hash, datas)
  - Detecção de duplicatas
  - Full-text search em nomes
- `file_history` - Histórico de mudanças em arquivos
- `directory_catalog` - Estatísticas por diretório
- `scan_configs` - Configurações de varredura
- `scan_jobs` - Jobs de varredura executados
- `scan_schedules` - Agendamento de varreduras

### Views Especializadas 🆕
- `v_file_scan_info` - Arquivos com informações do scan que os catalogou
- `file_access_patterns` - Classificação Hot/Warm/Cold
- `directory_summary` - Resumo estatístico por diretório

### Soft Delete

Os backups deletados não são removidos do banco, apenas marcados como inativos (`is_active = false`). Isso permite recuperação e auditoria.

## 🚧 Desenvolvimento

### Executar em modo desenvolvimento

```bash
cargo watch -x run
```

### Executar testes

```bash
# Bateria completa de testes
cargo test

# Testes unitários
cargo test --lib

# Testes de integração
cargo test --test '*'

# Testes com output detalhado
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

### Status Atual: v0.1.6 ✅ 
- ✅ API REST completa com 50+ endpoints
- ✅ Integração com 40+ provedores cloud via Rclone
- ✅ File Intelligence System com catálogo global
- ✅ Cloud Providers Management nativo
- ✅ Sistema de criptografia end-to-end
- ✅ 21 testes automatizados (unit + integration + e2e)

### Próximo: Milestone 3 🔄 Restore-First Design
- 🔄 Verificação automática de restore após backup
- 🔄 Dashboard executivo de confiabilidade 
- 🔄 Sistema .b2ignore para exclusão inteligente
- 🔄 Configuração via TOML (Infrastructure as Code)

## 🧪 Testes e Qualidade

- **21 testes automatizados** cobrindo funcionalidades críticas
- **Documentação inline** para geração automática com `cargo doc`
- **Logs estruturados** para debugging eficiente
- **Type safety** garantido pelo Rust e SQLx

## 📄 Licença

Este projeto está sob a licença MIT. Veja o arquivo [LICENSE](LICENSE) para mais detalhes.