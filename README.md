# b2cli - Backup to Cloud CLI & API

Uma plataforma de backup e gestão de dados segura, inteligente e flexível para pequenas empresas e uso pessoal, utilizando o Rclone como motor de transferência.

## Pré-requisitos

Antes de começar, você precisará ter as seguintes ferramentas instaladas em seu sistema:

- **Rust e Cargo**: A linguagem e seu gerenciador de pacotes. [Instruções de Instalação](https://www.rust-lang.org/tools/install)
- **Rclone**: A ferramenta que move os arquivos. [Instruções de Instalação](https://rclone.org/install/)
- **Docker e Docker Compose**: Recomendado para a forma mais fácil de configurar o banco de dados. [Instruções de Instalação do Docker](https://docs.docker.com/engine/install/)

---

## Getting Started: Rodando a API Localmente

Siga estes passos para ter a API do b2cli rodando em sua máquina.

### 1. Instale as Ferramentas de Desenvolvimento

Este projeto usa `sqlx-cli` para gerenciar as migrações do banco de dados. Instale-o via Cargo (você só precisa fazer isso uma vez):

```bash
cargo install sqlx-cli
```

### 2. Configure o Ambiente

Crie seu arquivo de ambiente local (`.env`) a partir do exemplo fornecido.

```bash
cp .env.example .env
```

**Importante**: Se você for usar um banco de dados PostgreSQL que não seja o do Docker Compose, **edite o arquivo `.env` agora** e atualize a variável `DATABASE_URL` para apontar para o seu banco.

### 3. Prepare o Banco de Dados

Você tem duas opções: usar o Docker (recomendado) ou usar seu próprio PostgreSQL.

#### Opção A: Iniciar com Docker (Recomendado)

Este comando irá iniciar um contêiner PostgreSQL com toda a configuração necessária.

```bash
docker-compose up -d
```

#### Opção B: Usando um Banco de Dados Existente

Se você já tem um PostgreSQL rodando (localmente, na rede ou na nuvem), simplesmente garanta que ele esteja acessível e que a `DATABASE_URL` no seu arquivo `.env` esteja correta.

### 4. Rode as Migrações do Banco

Este comando, usando o `cargo sqlx`, irá criar as tabelas necessárias no banco de dados que você configurou.

```bash
cargo sqlx migrate run
```

### 5. Inicie a API

Finalmente, inicie o servidor da aplicação.

```bash
cargo run
```

A API estará disponível em `http://localhost:3000` e a documentação interativa do Swagger UI em `http://localhost:3000/swagger-ui`.