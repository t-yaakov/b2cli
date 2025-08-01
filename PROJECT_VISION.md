# Visão do Projeto b2cli: Da Ferramenta Simples à Plataforma de Dados Inteligente

Este documento descreve a visão evolutiva do b2cli, delineando como cada fase de desenvolvimento se baseia na anterior para transformar uma simples ferramenta de cópia em uma plataforma de gestão de dados robusta e inteligente.

---

## Princípios Fundamentais

- **API-First**: Tudo é construído como uma API. Isso garante flexibilidade máxima para futuras interfaces (Web, Desktop, CLI).
- **Motor Robusto**: Utilizamos o `rclone` como o coração de todas as operações de transferência de dados, aproveitando sua performance e vasto suporte a provedores de nuvem.
- **A Catalogação é a Chave**: A funcionalidade mais importante do nosso sistema não é a cópia, mas a **catalogação**. Cada arquivo movido, em qualquer tipo de operação, tem seus metadados registrados em nosso banco de dados. Este catálogo é a base para toda a inteligência futura.

---

## A Evolução das Funcionalidades

### Fase 1: A Fundação - Cópia e Catálogo

O objetivo desta fase é criar um serviço de cópia de arquivos confiável e auditável. Não há distinção entre "cópia simples" e "backup"; tudo é um **Job de Backup**.

- **Recurso Principal**: Uma API RESTful para gerenciar `BackupJobs` (`POST`, `GET`, `DELETE`).
- **Execução Básica**: Implementar a rota `POST /backups/{id}/execute` que realiza uma operação `rclone copy` direta da origem para o destino.
- **Catalogação Universal**: Após cada execução, o sistema lê o manifesto da transferência e popula a tabela `cataloged_files` com os metadados de cada arquivo (caminho, tamanho, hash, data de modificação).
- **Resultado**: Uma API que pode mover arquivos de forma confiável e que **sabe onde cada arquivo está**.

### Fase 2: Segurança e Resiliência - O Backup Inteligente

Com a fundação estabelecida, adicionamos camadas de segurança e proteção de dados. Estas são apenas "flags" e parâmetros adicionados aos `BackupJobs` existentes.

- **Criptografia de Ponta a Ponta**: Adicionar suporte para o `rclone crypt`, permitindo que os dados sejam armazenados de forma ilegível no destino. O catálogo ainda registra os arquivos, mas a API gerencia o acesso ao "cofre" criptografado.
- **Versionamento de Arquivos**: Implementar o `--backup-dir` do rclone. Em vez de sobrescrever arquivos, as versões antigas são movidas para um diretório de arquivamento. O nosso catálogo será estendido para rastrear as diferentes versões de um mesmo arquivo.
- **Monitoramento de Jobs**: Criar um histórico de execuções para cada `BackupJob`, registrando o sucesso, falha, número de arquivos transferidos e duração.

### Fase 3: Automação e Otimização - A Plataforma Autônoma

O objetivo é reduzir a intervenção manual e otimizar os custos de armazenamento.

- **Agendamento de Jobs**: Implementar um scheduler interno que executa os `BackupJobs` automaticamente com base em uma programação (ex: diário, semanal).
- **Políticas de Ciclo de Vida**: Com base nos dados do nosso catálogo (ex: "arquivos não acessados há mais de 90 dias"), criar jobs especiais que movem dados de um armazenamento "quente" (caro e rápido) para um "frio" (barato e lento).
- **Controle de Performance**: Permitir a configuração de limites de banda e de transferências concorrentes para que os backups não impactem a performance da rede do usuário.

### Fase 4: Insights e Ação - A Gestão de Dados Proativa

Esta é a fase onde colhemos os frutos da nossa rigorosa catalogação. O sistema deixa de ser apenas reativo e se torna proativo.

- **Busca Avançada**: Implementar uma busca poderosa que permite ao usuário encontrar arquivos em todos os seus backups com base em nome, data, tamanho ou até mesmo tags.
- **Relatórios e Dashboards**: Fornecer visualizações sobre o uso do armazenamento, tipos de arquivos mais comuns, crescimento dos dados ao longo do tempo, etc.
- **Inteligência Artificial (IA)**:
    - **Análise de Segurança**: Analisar os metadados em busca de padrões suspeitos (ex: um grande número de arquivos sendo criptografados subitamente, indicando um possível ransomware).
    - **Sugestões de Otimização**: Sugerir proativamente ao usuário a criação de políticas de ciclo de vida com base nos padrões de acesso aos arquivos.
    - **Detecção de Anomalias**: Alertar sobre desvios no padrão normal de backups (ex: um backup que costumava ter 1GB de repente tem 10GB).
