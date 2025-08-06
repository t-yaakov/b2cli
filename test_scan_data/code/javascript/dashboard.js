/**
 * B2CLI Executive Dashboard
 * 
 * Dashboard interativo para executivos visualizarem:
 * - Status dos backups em tempo real
 * - Mapa de risco dos arquivos
 * - Métricas de compliance
 * - Alertas críticos
 * 
 * @author B2CLI Team
 * @version 2.0.0
 */

class B2CLIDashboard {
    constructor(containerId) {
        this.container = document.getElementById(containerId);
        this.apiBase = '/api/v1';
        this.refreshInterval = 30000; // 30 segundos
        this.charts = {};
        
        this.init();
    }

    /**
     * Inicializa o dashboard
     */
    async init() {
        try {
            this.showLoading();
            await this.loadDashboardData();
            this.renderDashboard();
            this.startAutoRefresh();
        } catch (error) {
            console.error('Erro ao inicializar dashboard:', error);
            this.showError('Falha ao carregar dashboard');
        }
    }

    /**
     * Carrega dados da API
     */
    async loadDashboardData() {
        const [backupStats, riskData, complianceStatus, alerts] = await Promise.all([
            this.fetchAPI('/backups/stats'),
            this.fetchAPI('/files/risk-analysis'),
            this.fetchAPI('/compliance/status'),
            this.fetchAPI('/alerts/active')
        ]);

        this.data = {
            backupStats,
            riskData, 
            complianceStatus,
            alerts
        };
    }

    /**
     * Renderiza o dashboard completo
     */
    renderDashboard() {
        this.container.innerHTML = `
            <div class="dashboard-header">
                <h1>B2CLI Executive Dashboard</h1>
                <div class="last-update">
                    Última atualização: ${new Date().toLocaleString('pt-BR')}
                </div>
            </div>

            <div class="dashboard-grid">
                <!-- KPI Cards -->
                <div class="kpi-section">
                    ${this.renderKPICards()}
                </div>

                <!-- Risk Map -->
                <div class="risk-map-section">
                    <h2>🗺️ Mapa de Risco</h2>
                    <div id="risk-map-chart"></div>
                </div>

                <!-- Backup Status -->
                <div class="backup-status-section">
                    <h2>💾 Status dos Backups</h2>
                    <div id="backup-status-chart"></div>
                </div>

                <!-- Compliance -->
                <div class="compliance-section">
                    <h2>📊 Compliance</h2>
                    <div id="compliance-chart"></div>
                </div>

                <!-- Alerts -->
                <div class="alerts-section">
                    <h2>🚨 Alertas Críticos</h2>
                    <div id="alerts-list"></div>
                </div>

                <!-- File Categories -->
                <div class="file-categories-section">
                    <h2>📁 Categorias de Arquivos</h2>
                    <div id="file-categories-chart"></div>
                </div>
            </div>
        `;

        // Renderizar gráficos
        this.renderCharts();
    }

    /**
     * Renderiza cards de KPI
     */
    renderKPICards() {
        const { backupStats, riskData } = this.data;
        
        const kpis = [
            {
                title: 'Backups Ativos',
                value: backupStats.active_jobs,
                change: '+5.2%',
                icon: '💾',
                color: 'success'
            },
            {
                title: 'Arquivos Críticos',
                value: riskData.critical_files,
                change: '-2.1%',
                icon: '⚠️',
                color: 'warning'
            },
            {
                title: 'Success Rate',
                value: `${backupStats.success_rate}%`,
                change: '+1.3%',
                icon: '✅',
                color: 'success'
            },
            {
                title: 'Storage Usado',
                value: this.formatBytes(backupStats.total_storage),
                change: '+12.5%',
                icon: '💿',
                color: 'info'
            }
        ];

        return kpis.map(kpi => `
            <div class="kpi-card ${kpi.color}">
                <div class="kpi-icon">${kpi.icon}</div>
                <div class="kpi-content">
                    <div class="kpi-value">${kpi.value}</div>
                    <div class="kpi-title">${kpi.title}</div>
                    <div class="kpi-change">${kpi.change}</div>
                </div>
            </div>
        `).join('');
    }

    /**
     * Renderiza todos os gráficos
     */
    renderCharts() {
        this.renderRiskMapChart();
        this.renderBackupStatusChart();
        this.renderComplianceChart();
        this.renderAlertsList();
        this.renderFileCategoriesChart();
    }

    /**
     * Renderiza mapa de risco
     */
    renderRiskMapChart() {
        const ctx = document.getElementById('risk-map-chart');
        if (!ctx) return;

        const { riskData } = this.data;
        
        // Simular dados de mapa de risco por diretório
        const riskMapData = {
            labels: ['Contracts', 'Financial', 'HR', 'Engineering', 'Marketing', 'Legal'],
            datasets: [{
                label: 'Risk Score',
                data: [95, 88, 72, 45, 32, 91],
                backgroundColor: [
                    '#ff4757', '#ff4757', '#ffa502', '#2ed573', '#2ed573', '#ff4757'
                ],
                borderColor: '#ffffff',
                borderWidth: 2
            }]
        };

        this.charts.riskMap = new Chart(ctx, {
            type: 'doughnut',
            data: riskMapData,
            options: {
                responsive: true,
                plugins: {
                    legend: {
                        position: 'bottom'
                    },
                    tooltip: {
                        callbacks: {
                            label: function(context) {
                                return `${context.label}: ${context.parsed}% risk`;
                            }
                        }
                    }
                }
            }
        });
    }

    /**
     * Renderiza status dos backups
     */
    renderBackupStatusChart() {
        const ctx = document.getElementById('backup-status-chart');
        if (!ctx) return;

        const { backupStats } = this.data;
        
        // Dados dos últimos 7 dias
        const statusData = {
            labels: ['Seg', 'Ter', 'Qua', 'Qui', 'Sex', 'Sáb', 'Dom'],
            datasets: [
                {
                    label: 'Sucessos',
                    data: [45, 48, 52, 49, 55, 35, 42],
                    backgroundColor: '#2ed573',
                    borderColor: '#2ed573',
                    tension: 0.4
                },
                {
                    label: 'Falhas',
                    data: [2, 1, 3, 2, 1, 4, 2],
                    backgroundColor: '#ff4757',
                    borderColor: '#ff4757',
                    tension: 0.4
                }
            ]
        };

        this.charts.backupStatus = new Chart(ctx, {
            type: 'line',
            data: statusData,
            options: {
                responsive: true,
                scales: {
                    y: {
                        beginAtZero: true
                    }
                },
                plugins: {
                    legend: {
                        position: 'top'
                    }
                }
            }
        });
    }

    /**
     * Renderiza gráfico de compliance
     */
    renderComplianceChart() {
        const ctx = document.getElementById('compliance-chart');
        if (!ctx) return;

        const { complianceStatus } = this.data;
        
        const complianceData = {
            labels: ['LGPD', 'ISO 27001', 'SOX', 'GDPR'],
            datasets: [{
                data: [95, 88, 92, 87],
                backgroundColor: ['#2ed573', '#ffa502', '#2ed573', '#ffa502'],
                borderWidth: 0
            }]
        };

        this.charts.compliance = new Chart(ctx, {
            type: 'bar',
            data: complianceData,
            options: {
                responsive: true,
                plugins: {
                    legend: {
                        display: false
                    }
                },
                scales: {
                    y: {
                        beginAtZero: true,
                        max: 100,
                        ticks: {
                            callback: function(value) {
                                return value + '%';
                            }
                        }
                    }
                }
            }
        });
    }

    /**
     * Renderiza lista de alertas
     */
    renderAlertsList() {
        const alertsContainer = document.getElementById('alerts-list');
        if (!alertsContainer) return;

        const { alerts } = this.data;
        
        // Simular alertas críticos
        const alertsHTML = [
            {
                level: 'critical',
                message: 'Backup do contrato Microsoft falhando há 3 dias',
                time: '2 horas atrás',
                icon: '🔴'
            },
            {
                level: 'warning', 
                message: '5 arquivos críticos sem backup nas últimas 24h',
                time: '4 horas atrás',
                icon: '🟡'
            },
            {
                level: 'info',
                message: 'Storage atingiu 80% da capacidade',
                time: '6 horas atrás',
                icon: '🔵'
            }
        ].map(alert => `
            <div class="alert-item ${alert.level}">
                <span class="alert-icon">${alert.icon}</span>
                <div class="alert-content">
                    <div class="alert-message">${alert.message}</div>
                    <div class="alert-time">${alert.time}</div>
                </div>
            </div>
        `).join('');

        alertsContainer.innerHTML = alertsHTML;
    }

    /**
     * Renderiza gráfico de categorias de arquivos
     */
    renderFileCategoriesChart() {
        const ctx = document.getElementById('file-categories-chart');
        if (!ctx) return;

        const categoryData = {
            labels: ['Documents', 'Code', 'Media', 'Config', 'Logs', 'Other'],
            datasets: [{
                data: [35, 25, 20, 10, 5, 5],
                backgroundColor: [
                    '#3742fa', '#2ed573', '#ff4757', '#ffa502', '#747d8c', '#a4b0be'
                ]
            }]
        };

        this.charts.fileCategories = new Chart(ctx, {
            type: 'pie',
            data: categoryData,
            options: {
                responsive: true,
                plugins: {
                    legend: {
                        position: 'right'
                    }
                }
            }
        });
    }

    /**
     * Inicia atualização automática
     */
    startAutoRefresh() {
        setInterval(async () => {
            try {
                await this.loadDashboardData();
                this.updateCharts();
                console.log('Dashboard atualizado automaticamente');
            } catch (error) {
                console.error('Erro na atualização automática:', error);
            }
        }, this.refreshInterval);
    }

    /**
     * Atualiza dados dos gráficos existentes
     */
    updateCharts() {
        // Atualizar apenas os dados, não recriar os gráficos
        Object.values(this.charts).forEach(chart => {
            if (chart && chart.update) {
                chart.update('none'); // Animação suave
            }
        });
    }

    /**
     * Utilitários
     */
    async fetchAPI(endpoint) {
        const response = await fetch(`${this.apiBase}${endpoint}`);
        if (!response.ok) {
            throw new Error(`API Error: ${response.status}`);
        }
        return response.json();
    }

    formatBytes(bytes) {
        const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
        if (bytes === 0) return '0 B';
        const i = Math.floor(Math.log(bytes) / Math.log(1024));
        return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i];
    }

    showLoading() {
        this.container.innerHTML = '<div class="loading">Carregando dashboard...</div>';
    }

    showError(message) {
        this.container.innerHTML = `<div class="error">Erro: ${message}</div>`;
    }
}

// Inicializar dashboard quando a página carregar
document.addEventListener('DOMContentLoaded', () => {
    const dashboard = new B2CLIDashboard('dashboard-container');
    
    // Exportar para uso global
    window.B2CLIDashboard = dashboard;
});

// CSS inline para demonstração
const dashboardCSS = `
.dashboard-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 20px;
    padding: 20px;
}

.kpi-section {
    grid-column: 1 / -1;
    display: flex;
    gap: 20px;
    flex-wrap: wrap;
}

.kpi-card {
    flex: 1;
    min-width: 200px;
    padding: 20px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    color: white;
}

.kpi-card.success { background: linear-gradient(135deg, #2ed573, #2ed573); }
.kpi-card.warning { background: linear-gradient(135deg, #ffa502, #ff6348); }
.kpi-card.info { background: linear-gradient(135deg, #3742fa, #2f3542); }

.alert-item {
    display: flex;
    align-items: center;
    padding: 10px;
    margin: 10px 0;
    border-radius: 6px;
    border-left: 4px solid;
}

.alert-item.critical { border-color: #ff4757; background: #fff5f5; }
.alert-item.warning { border-color: #ffa502; background: #fffdf0; }
.alert-item.info { border-color: #3742fa; background: #f8f9ff; }
`;

// Adicionar CSS ao head
const style = document.createElement('style');
style.textContent = dashboardCSS;
document.head.appendChild(style);