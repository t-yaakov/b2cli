# B2CLI Development Roadmap

## 🎯 O que é o B2CLI

B2CLI é uma ferramenta de backup que garante que seus backups **realmente funcionam** quando você precisar restaurar. Diferente de outras ferramentas que apenas "fazem backup", nós **testamos o restore automaticamente**.

---

## 📊 Status Atual - Agosto 2025

### ✅ **O que já funciona hoje**

- **API REST completa** - Todos os endpoints funcionais
- **Backup para nuvem** - Integração com rclone (40+ provedores)  
- **Agendamento robusto** - Cron expressions com tokio-cron-scheduler
- **Logs detalhados** - Métricas de transferência, duração, erros
- **Sistema de arquivamento** - Hot/Warm storage automático
- **Cloud Providers** ✨ **NOVO** - Gestão de B2, IDrive e2, Wasabi, Scaleway
- **Documentação interativa** - Swagger UI + Redoc

### 🏗️ **Arquitetura atual**
```
[REST API] → [PostgreSQL] → [Rclone Worker] → [Cloud Storage]
     ↓            ↓              ↓             ↗
[Scheduler]  [Execution Logs]  [Metrics]  [Cloud Providers]
                                              ↓
                                    [Connectivity Tests]
```

### 📦 **Como usar hoje**
```bash
# 1. Configurar provedor cloud
curl -X POST localhost:3000/providers \
  -d '{"name": "B2 Backup", "provider_type": "backblaze_b2", 
       "bucket": "meu-backup", "access_key": "...", "secret_key": "..."}'

# 2. Testar conectividade
curl -X POST localhost:3000/providers/{id}/test

# 3. Criar backup job
curl -X POST localhost:3000/backups \
  -d '{"name": "Docs", "mappings": {"/home/docs": ["gdrive:backup"]}}'

# 4. Criar agendamento  
curl -X POST localhost:3000/backups/{id}/schedule \
  -d '{"name": "Daily", "cron_expression": "0 0 2 * * *"}'

# 5. Executar manualmente
curl -X POST localhost:3000/backups/{id}/run
```

---

## 🚀 Próximas Versões

### **v0.1.5 - Test Suite Complete** ✅ (CONCLUÍDO)

**Funcionalidades Implementadas:**
- ✅ **21 testes automatizados** funcionando
- ✅ **Unit tests** para backup_worker e rclone
- ✅ **End-to-end tests** para operações de arquivo
- ✅ **Test fixtures** e mocks para desenvolvimento
- ✅ **Documentação completa** de testes (TESTING_GUIDE.md)

### **v0.1.6 - Cloud Providers** ✅ (CONCLUÍDO - Agosto 2025)

**Funcionalidades Implementadas:**
- ✅ **CRUD completo** para cloud providers
- ✅ **Suporte multi-provedor**: Backblaze B2, IDrive e2, Wasabi, Scaleway
- ✅ **Teste de conectividade** com validação de credenciais
- ✅ **Templates de configuração** com exemplos práticos
- ✅ **APIs S3-compatible e B2 native**
- ✅ **Documentação Rust** completa (/// comments)
- ✅ **Validação específica** por tipo de provedor
- ✅ **Logs estruturados** sem poluir terminal

### **v0.2.0 - Rclone + Cloud Integration** (Próximo - 1-2 semanas)

**Objetivo:** Validar todo o sistema antes de implementar restore verification.

**Tarefas:**
- [ ] **Testar backup local → local** com arquivos reais
- [ ] **Testar backup local → rclone** (Google Drive, S3)
- [ ] **Validar agendamento** com diferentes cron expressions
- [ ] **Stress test** com 1000+ arquivos
- [ ] **Testar recuperação** após falhas simuladas
- [ ] **Validar logs** e métricas capturadas

### **v0.3.0 - Restore Verification** (2-4 semanas)

**Problema:** 50% dos backups falham na hora de restaurar, mas você só descobre quando precisa.

**Solução:** Verificação automática de restore após cada backup.

**Funcionalidades:**
- [ ] **Restore automático silencioso** após backup
- [ ] **Verificação de integridade** via checksum SHA256  
- [ ] **Métricas de confiabilidade** - "98% dos seus backups são restauráveis"
- [ ] **API de restore** - `POST /backups/{id}/restore`
- [ ] **Dashboard de status** - Visual do que está funcionando

**Entrega:** Você sabe que seus backups **realmente funcionam**.

---

### **v0.4.0 - Smart Configuration** (8 semanas)

**Problema:** Configurar backups é manual e propenso a erro.

**Solução:** Configuration as Code + exclusões inteligentes.

**Funcionalidades:**
- [ ] **Arquivos .b2ignore** - Sintaxe tipo gitignore
- [ ] **Templates por linguagem** - Python, Rust, Node.js, etc.
- [ ] **Configuração TOML** - `b2cli apply -f backup-config.toml`
- [ ] **Validação de config** antes de aplicar
- [ ] **Export/Import** de configurações

**Entrega:** Configuração versionada, reproduzível e inteligente.

---

### **v0.5.0 - File Intelligence** (12 semanas)

**Problema:** "Onde está o arquivo contrato_microsoft.pdf?"

**Solução:** Catálogo global com busca full-text.

**Funcionalidades:**
- [ ] **Índice global** de todos os arquivos
- [ ] **API de busca** - `GET /files/search?q=contrato`
- [ ] **Detecção de duplicatas** - Mesmo arquivo em vários lugares
- [ ] **Mapa de localização** - Onde está cada arquivo
- [ ] **Arquivos em risco** - Sem backup há X dias

**Entrega:** Você sempre sabe onde estão seus arquivos.

---

### **v0.6.0 - Executive Dashboard** (16 semanas)

**Problema:** Gestores não sabem o status real dos backups.

**Solução:** Dashboard executivo com métricas de risco.

**Funcionalidades:**
- [ ] **Interface web** para visualização
- [ ] **Score de risco** por departamento/projeto
- [ ] **Alertas proativos** - Email/Slack para problemas
- [ ] **Relatórios de compliance** - LGPD, GDPR ready
- [ ] **SLA tracking** - Tempo de backup/restore

**Entrega:** Visibilidade executiva sobre riscos de dados.

---

## 🔧 Funcionalidades Técnicas Planejadas

### **Performance & Scalability**
- [ ] **Parallel transfers** - Múltiplos arquivos simultâneos
- [ ] **Resume transfers** - Continuar uploads interrompidos  
- [ ] **Bandwidth throttling** - Controle de velocidade
- [ ] **Deduplicação** - Arquivos idênticos não duplicados

### **Security & Compliance**
- [ ] **End-to-end encryption** - Arquivos criptografados na origem
- [ ] **API authentication** - JWT tokens + API keys
- [ ] **Audit logs** - Quem fez o quê quando
- [ ] **RBAC** - Role-based access control

### **Integrations**
- [ ] **Webhooks** - Notificações em tempo real
- [ ] **Prometheus metrics** - Monitoramento + alerting
- [ ] **CLI tool** - Interface de linha de comando
- [ ] **Docker images** - Deploy facilitado

---

## 🛠️ Como Contribuir

### **Setup de Desenvolvimento**
```bash
git clone https://github.com/user/b2cli
cd b2cli
cargo install sqlx-cli
sqlx migrate run
cargo run
```

### **Áreas que Precisam de Ajuda**
1. **Restore verification** - Core do diferencial
2. **File indexing** - Performance de busca
3. **Web dashboard** - Interface de usuário
4. **Documentation** - Guias e tutoriais

### **Processo**
1. Abra uma **issue** descrevendo o que quer implementar
2. Discuta a abordagem antes de começar
3. Faça **testes** para novas funcionalidades
4. Documente no **CLAUDE.md** se for mudança significativa

---

## 📈 Métricas de Sucesso

### **Técnicas**
- **Restore Success Rate:** >95% (atual: não medido)
- **Backup Completion Time:** <5min para 1GB
- **File Discovery Time:** <2s para qualquer arquivo

### **Usabilidade**  
- **Setup Time:** <10min do git clone ao primeiro backup
- **Config Complexity:** <10 linhas para caso típico
- **Learning Curve:** Usuário produtivo em <30min

---

## 🎯 Filosofia de Desenvolvimento

### **Restore-First Design**
Toda funcionalidade de backup deve incluir teste de restore automático.

### **API-First**
Interface web é cliente da API, não o contrário.

### **Configuration as Code**
Configuração deve ser versionável e reproduzível.

### **Observable by Default**
Logs, métricas e traces em tudo.

---

**Status:** v0.1.5 ✅ | v0.2.0 iniciando testes 🧪  
**Próximo milestone:** Testar sistema completo (1 semana)  
**Diferencial:** Backup que realmente funciona quando você precisa