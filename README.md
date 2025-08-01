# B2CLI - Plataforma de Backup e Gestão de Dados

B2CLI é uma plataforma de backup inteligente que evolui de uma simples ferramenta de cópia para um sistema completo de gestão de dados. Construído com Rust, oferece performance, segurança e flexibilidade para pequenas empresas e uso pessoal.

## 🚀 Características Principais

- **API REST completa** para gerenciamento de backups
- **Backup local** de arquivos e diretórios
- **Sistema de logs estruturado** com rotação diária
- **Soft delete** para segurança dos dados
- **Catalogação automática** de arquivos com metadados
- **Documentação interativa** via Swagger UI e Redoc

## 🛠️ Tecnologias

- **Rust** - Linguagem principal
- **Axum** - Framework web async
- **PostgreSQL** - Banco de dados
- **SQLx** - ORM com verificação em tempo de compilação
- **Tracing** - Sistema de logs estruturado
- **OpenAPI** - Documentação automática da API

## 📋 Pré-requisitos

- **Rust 1.70+** e Cargo - [Instruções de Instalação](https://www.rust-lang.org/tools/install)
- **PostgreSQL 14+** - Banco de dados
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

- `POST /backups` - Criar novo job de backup
- `GET /backups` - Listar jobs de backup ativos
- `GET /backups/{id}` - Obter detalhes de um job
- `DELETE /backups/{id}` - Deletar um job (soft delete)
- `POST /backups/{id}/run` - Executar um backup

### Exemplo de Criação de Backup

```json
POST /backups
{
  "name": "Backup de Documentos",
  "mappings": {
    "/home/user/Documents": [
      "/mnt/backup/docs",
      "/mnt/external/backup"
    ]
  }
}
```

## 📁 Estrutura do Projeto

```
b2cli/
├── src/
│   ├── main.rs           # Entrada principal
│   ├── routes/           # Handlers HTTP
│   ├── models.rs         # Modelos de dados
│   ├── db.rs            # Funções de banco
│   ├── backup_worker.rs  # Lógica de backup
│   └── logging.rs       # Configuração de logs
├── migrations/          # Migrations SQL
├── logs/               # Arquivos de log (gerado automaticamente)
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
  
- `backed_up_files` - Catálogo de arquivos copiados
  - `backup_job_id` - Referência ao job
  - `original_path` - Caminho original
  - `backed_up_path` - Caminho de destino
  - `file_size` - Tamanho em bytes
  - `checksum` - SHA256 do arquivo

### Soft Delete

Os backups deletados não são removidos do banco, apenas marcados como inativos (`is_active = false`). Isso permite recuperação e auditoria.

## 🚧 Desenvolvimento

### Executar em modo desenvolvimento

```bash
cargo watch -x run
```

### Executar testes

```bash
cargo test
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

## 🚀 Roadmap

Veja [ROADMAP.md](ROADMAP.md) para o progresso detalhado e próximos passos, incluindo:

- Suporte a provedores de nuvem (S3, B2, etc)
- Criptografia de arquivos
- Versionamento de backups
- Interface web
- Agendamento automático

## 🤝 Contribuindo

1. Fork o projeto
2. Crie sua feature branch (`git checkout -b feature/amazing-feature`)
3. Commit suas mudanças (`git commit -m 'Add amazing feature'`)
4. Push para a branch (`git push origin feature/amazing-feature`)
5. Abra um Pull Request

## 📄 Licença

Este projeto está sob a licença MIT. Veja o arquivo [LICENSE](LICENSE) para mais detalhes.

## 📞 Suporte

- Issues: [GitHub Issues](https://github.com/seu-usuario/b2cli/issues)
- Documentação: [Wiki](https://github.com/seu-usuario/b2cli/wiki)