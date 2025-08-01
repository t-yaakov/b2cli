# B2CLI Development Roadmap

## Visão do Produto
Criar uma plataforma de backup e gestão de dados segura, inteligente e flexível para pequenas empresas e uso pessoal, que oferece controle total sobre a segurança e os custos de armazenamento, utilizando o Rclone como motor de transferência.

---

### **Milestone 1: Core API & Local Backup Functionality** ✅

**Objetivo:** Estabelecer uma API REST robusta para backups locais, servindo como fundação para futuras integrações.

**Status:** CONCLUÍDO

- [x] **1.1: Configuração do Projeto & API REST:**
    - [x] Inicializar projeto Rust com Axum
    - [x] Configurar PostgreSQL ao invés de SQLite
    - [x] Implementar documentação automática (Swagger/OpenAPI)
- [x] **1.2: Integração com Banco de Dados:**
    - [x] Configurar SQLx com PostgreSQL
    - [x] Criar migrations para tabelas iniciais
    - [x] Implementar modelos de dados
- [x] **1.3: Endpoints Básicos da API:**
    - [x] POST /backups - Criar novo backup
    - [x] GET /backups - Listar backups
    - [x] GET /backups/{id} - Obter detalhes
    - [x] DELETE /backups/{id} - Soft delete
    - [x] POST /backups/{id}/run - Executar backup
- [x] **1.4: Sistema de Backup Local:**
    - [x] Implementar worker de backup assíncrono
    - [x] Catalogação automática de arquivos copiados
    - [x] Cálculo de checksum SHA256
    - [x] Suporte a múltiplos destinos por origem
- [x] **1.5: Sistema de Logs:**
    - [x] Logs estruturados com tracing
    - [x] Rotação diária de logs
    - [x] Níveis diferentes para console e arquivo
- [x] **1.6: Melhorias de Qualidade:**
    - [x] Soft delete com campo is_active
    - [x] Status de jobs (PENDING, RUNNING, COMPLETED, FAILED)
    - [x] Tratamento robusto de erros
    - [x] Health e readiness checks

---

### **Milestone 2: Integração com Rclone & Cloud Storage** 🚧

**Objetivo:** Integrar o Rclone como motor de transferência e adicionar suporte a provedores de nuvem.

**Status:** PRÓXIMO

- [ ] **2.1: Integração com Rclone:**
    - [ ] Detectar instalação do rclone no sistema
    - [ ] Wrapper Rust para comandos rclone
    - [ ] Parser de output do rclone para progresso
    - [ ] Tratamento de erros específicos do rclone
- [ ] **2.2: Configuração de Provedores:**
    - [ ] Endpoint para listar provedores disponíveis
    - [ ] Armazenar configurações de remotes no banco
    - [ ] Criptografia de credenciais sensíveis
    - [ ] Validação de configurações antes de salvar
- [ ] **2.3: Backup para Nuvem:**
    - [ ] Suporte a destinos S3 compatíveis
    - [ ] Suporte a Google Drive, OneDrive, Dropbox
    - [ ] Progress tracking durante upload
    - [ ] Retry automático em falhas
- [ ] **2.4: Funcionalidades Avançadas:**
    - [ ] Bandwidth limiting
    - [ ] Filtros de arquivos (include/exclude)
    - [ ] Dry-run mode
    - [ ] Sincronização bidirecional

---

### **Milestone 3: Recursos Avançados de Backup** 📋

**Objetivo:** Funcionalidades profissionais como criptografia, versionamento e agendamento.

**Status:** PLANEJADO

- [ ] **3.1: Criptografia End-to-End:**
    - [ ] Integração com rclone crypt
    - [ ] Geração e gerenciamento de chaves
    - [ ] Backup criptografado transparente
    - [ ] Recuperação de chaves perdidas
- [ ] **3.2: Versionamento de Arquivos:**
    - [ ] Sistema de versões com timestamps
    - [ ] Políticas de retenção configuráveis
    - [ ] Deduplicação inteligente
    - [ ] API para listar versões de arquivos
- [ ] **3.3: Agendamento e Automação:**
    - [ ] Scheduler interno com cron expressions
    - [ ] Execução automática de backups
    - [ ] Notificações por email/webhook
    - [ ] Monitoramento de falhas
- [ ] **3.4: Interface Web:**
    - [ ] Dashboard para visualização de jobs
    - [ ] Configuração via interface web
    - [ ] Gráficos de uso e estatísticas
    - [ ] Sistema de usuários e permissões

---

### **Milestone 4: Inteligência e Análise de Dados** 🔮

**Objetivo:** Transformar dados de backup em insights acionáveis.

**Status:** FUTURO

- [ ] **4.1: Análise Inteligente:**
    - [ ] Machine Learning para padrões de uso
    - [ ] Detecção de anomalias (possível ransomware)
    - [ ] Sugestões automáticas de otimização
    - [ ] Previsão de crescimento de dados
- [ ] **4.2: Lifecycle Management:**
    - [ ] Políticas automáticas de tiering (hot/cold)
    - [ ] Migração baseada em padrões de acesso
    - [ ] Otimização de custos de armazenamento
    - [ ] Cleanup automático de dados antigos
- [ ] **4.3: Relatórios e Dashboards:**
    - [ ] Analytics avançados de uso
    - [ ] Relatórios de compliance
    - [ ] Métricas de performance
    - [ ] Alertas proativos

### **Milestone 5: Produção e Escala** 🚀

**Objetivo:** Preparar para uso empresarial e alta disponibilidade.

**Status:** FUTURO

- [ ] **5.1: Infraestrutura:**
    - [ ] Containerização com Docker
    - [ ] Kubernetes deployment
    - [ ] High Availability setup
    - [ ] Load balancing
- [ ] **5.2: Monitoramento:**
    - [ ] Métricas Prometheus
    - [ ] Dashboards Grafana
    - [ ] Health checks avançados
    - [ ] Distributed tracing
- [ ] **5.3: Segurança Empresarial:**
    - [ ] OAuth2/OIDC integration
    - [ ] Audit logs completos
    - [ ] Compliance (GDPR, SOX)
    - [ ] Zero-trust architecture
- [ ] **5.4: Distribuição:**
    - [ ] Pacotes para todas as distribuições Linux
    - [ ] Instaladores Windows e macOS
    - [ ] Homebrew/Chocolatey packages
    - [ ] ARM64 support

---

## Estado Atual do Projeto

### ✅ Completado (Milestone 1)
- API REST funcional com documentação automática
- Backup local de arquivos com catalogação
- Sistema de logs estruturado
- Soft delete e gestão de status
- Testes básicos funcionando

### 🚧 Em Desenvolvimento
- Documentação atualizada
- Testes unitários expandidos

### 📋 Próximos Passos
1. Integração com Rclone
2. Suporte a provedores de nuvem
3. Sistema de configuração de remotes
4. Interface web básica

### 📊 Métricas do Projeto
- **Linguagem**: Rust
- **Framework**: Axum + SQLx
- **Banco**: PostgreSQL
- **Documentação**: OpenAPI/Swagger
- **Logs**: Tracing + JSON estruturado
- **Status**: MVP Funcional ✅