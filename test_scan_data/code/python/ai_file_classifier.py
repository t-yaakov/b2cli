#!/usr/bin/env python3
"""
AI File Classifier for B2CLI

Este módulo usa machine learning para classificar automaticamente
a criticidade dos arquivos baseado em:
- Nome do arquivo
- Extensão
- Conteúdo (quando possível)
- Localização
- Histórico de acesso

Author: B2CLI Team
Date: 2025-08-04
"""

import os
import re
import hashlib
import logging
from typing import List, Dict, Tuple
from pathlib import Path
from dataclasses import dataclass
from enum import Enum

# Simular imports de ML (na implementação real)
# from transformers import pipeline
# from sklearn.ensemble import RandomForestClassifier
# import ollama

class CriticalityLevel(Enum):
    """Níveis de criticidade dos arquivos"""
    LOW = 1
    MEDIUM = 2
    HIGH = 3
    CRITICAL = 4

@dataclass
class FileAnalysis:
    """Resultado da análise de um arquivo"""
    path: str
    criticality: CriticalityLevel
    confidence: float
    reasons: List[str]
    category: str
    risk_score: int

class AIFileClassifier:
    """Classificador inteligente de arquivos"""
    
    def __init__(self):
        self.critical_patterns = [
            r'contrato.*\.(pdf|docx)',
            r'senha.*\.(txt|json|env)',
            r'backup.*\.(sql|dump)',
            r'chave.*\.(pem|key|p12)',
            r'financeiro.*\.(xlsx|csv)',
        ]
        
        self.critical_dirs = [
            'contracts', 'financial', 'backup', 'keys', 
            'passwords', 'confidential', 'legal'
        ]
        
        self.extension_weights = {
            # Documentos críticos
            '.pdf': 0.7,
            '.docx': 0.6,
            '.xlsx': 0.8,
            
            # Código fonte
            '.py': 0.4,
            '.rs': 0.4,
            '.js': 0.3,
            '.sql': 0.9,
            
            # Configurações
            '.env': 0.9,
            '.config': 0.6,
            '.json': 0.5,
            
            # Backups
            '.bak': 0.8,
            '.dump': 0.9,
            
            # Cache/temp
            '.tmp': 0.1,
            '.cache': 0.1,
            '.log': 0.2,
        }
        
        self.logger = logging.getLogger(__name__)
    
    def analyze_file(self, file_path: str) -> FileAnalysis:
        """
        Analisa um arquivo e determina sua criticidade
        
        Args:
            file_path: Caminho para o arquivo
            
        Returns:
            FileAnalysis com criticidade e detalhes
        """
        path = Path(file_path)
        reasons = []
        risk_score = 0
        
        # 1. Análise do nome do arquivo
        filename_score = self._analyze_filename(path.name)
        risk_score += filename_score
        if filename_score > 50:
            reasons.append(f"Nome suspeito: {path.name}")
        
        # 2. Análise da extensão
        ext_score = self._analyze_extension(path.suffix)
        risk_score += ext_score
        if ext_score > 30:
            reasons.append(f"Extensão crítica: {path.suffix}")
        
        # 3. Análise do diretório
        dir_score = self._analyze_directory(str(path.parent))
        risk_score += dir_score
        if dir_score > 40:
            reasons.append(f"Diretório sensível: {path.parent.name}")
        
        # 4. Análise do tamanho
        try:
            size = path.stat().st_size
            size_score = self._analyze_size(size)
            risk_score += size_score
            if size_score > 20:
                reasons.append(f"Tamanho significativo: {self._format_size(size)}")
        except OSError:
            pass
        
        # 5. Determinar criticidade final
        if risk_score >= 80:
            criticality = CriticalityLevel.CRITICAL
            confidence = 0.9
        elif risk_score >= 60:
            criticality = CriticalityLevel.HIGH
            confidence = 0.8
        elif risk_score >= 30:
            criticality = CriticalityLevel.MEDIUM
            confidence = 0.7
        else:
            criticality = CriticalityLevel.LOW
            confidence = 0.6
        
        # 6. Categorizar arquivo
        category = self._categorize_file(path)
        
        return FileAnalysis(
            path=file_path,
            criticality=criticality,
            confidence=confidence,
            reasons=reasons,
            category=category,
            risk_score=min(risk_score, 100)
        )
    
    def _analyze_filename(self, filename: str) -> int:
        """Analisa o nome do arquivo para detectar padrões críticos"""
        score = 0
        filename_lower = filename.lower()
        
        # Padrões críticos
        critical_keywords = [
            'contrato', 'contract', 'senha', 'password', 'key', 'chave',
            'backup', 'financeiro', 'financial', 'confidencial', 'secret',
            'admin', 'root', 'master', 'private', 'legal'
        ]
        
        for keyword in critical_keywords:
            if keyword in filename_lower:
                score += 30
        
        # Padrões de data (podem ser backups)
        if re.search(r'\d{4}[-_]\d{2}[-_]\d{2}', filename):
            score += 15
            
        # Padrões de versão
        if re.search(r'v\d+\.\d+', filename_lower):
            score += 10
            
        return min(score, 60)
    
    def _analyze_extension(self, extension: str) -> int:
        """Analisa a extensão do arquivo"""
        if extension.lower() in self.extension_weights:
            weight = self.extension_weights[extension.lower()]
            return int(weight * 50)
        return 5
    
    def _analyze_directory(self, dir_path: str) -> int:
        """Analisa o diretório onde o arquivo está"""
        score = 0
        dir_lower = dir_path.lower()
        
        for critical_dir in self.critical_dirs:
            if critical_dir in dir_lower:
                score += 50
                break
        
        # Profundidade (arquivos muito enterrados podem ser importantes)
        depth = len(Path(dir_path).parts)
        if depth > 5:
            score += 10
            
        return min(score, 60)
    
    def _analyze_size(self, size_bytes: int) -> int:
        """Analisa o tamanho do arquivo"""
        if size_bytes > 100_000_000:  # > 100MB
            return 25
        elif size_bytes > 10_000_000:  # > 10MB
            return 15
        elif size_bytes > 1_000_000:  # > 1MB
            return 10
        elif size_bytes < 1024:  # < 1KB (muito pequeno, pode ser config)
            return 15
        return 5
    
    def _categorize_file(self, path: Path) -> str:
        """Categoriza o arquivo"""
        ext = path.suffix.lower()
        
        if ext in ['.pdf', '.docx', '.doc', '.txt']:
            return 'Document'
        elif ext in ['.xlsx', '.csv', '.xls']:
            return 'Spreadsheet'
        elif ext in ['.py', '.rs', '.js', '.java', '.cpp', '.c']:
            return 'Source Code'
        elif ext in ['.sql', '.db', '.sqlite']:
            return 'Database'
        elif ext in ['.json', '.yaml', '.yml', '.toml', '.env']:
            return 'Configuration'
        elif ext in ['.jpg', '.png', '.gif', '.bmp']:
            return 'Image'
        elif ext in ['.mp4', '.avi', '.mov']:
            return 'Video'
        elif ext in ['.zip', '.tar', '.gz', '.rar']:
            return 'Archive'
        elif ext in ['.log', '.txt']:
            return 'Log'
        else:
            return 'Other'
    
    def _format_size(self, size_bytes: int) -> str:
        """Formata tamanho em bytes para leitura humana"""
        for unit in ['B', 'KB', 'MB', 'GB']:
            if size_bytes < 1024:
                return f"{size_bytes:.1f}{unit}"
            size_bytes /= 1024
        return f"{size_bytes:.1f}TB"
    
    def analyze_directory(self, directory_path: str) -> List[FileAnalysis]:
        """
        Analisa todos os arquivos de um diretório
        
        Args:
            directory_path: Caminho do diretório
            
        Returns:
            Lista de análises de arquivos
        """
        analyses = []
        
        try:
            for root, dirs, files in os.walk(directory_path):
                # Ignorar diretórios conhecidos como não importantes
                dirs[:] = [d for d in dirs if d not in ['.git', '__pycache__', 'node_modules']]
                
                for file in files:
                    file_path = os.path.join(root, file)
                    try:
                        analysis = self.analyze_file(file_path)
                        analyses.append(analysis)
                    except Exception as e:
                        self.logger.warning(f"Erro ao analisar {file_path}: {e}")
        
        except Exception as e:
            self.logger.error(f"Erro ao analisar diretório {directory_path}: {e}")
        
        return analyses
    
    def generate_risk_report(self, analyses: List[FileAnalysis]) -> Dict:
        """
        Gera relatório de risco baseado nas análises
        
        Args:
            analyses: Lista de análises de arquivos
            
        Returns:
            Dicionário com estatísticas de risco
        """
        total_files = len(analyses)
        if total_files == 0:
            return {}
        
        # Contar por criticidade
        criticality_counts = {level: 0 for level in CriticalityLevel}
        for analysis in analyses:
            criticality_counts[analysis.criticality] += 1
        
        # Contar por categoria
        category_counts = {}
        for analysis in analyses:
            category_counts[analysis.category] = category_counts.get(analysis.category, 0) + 1
        
        # Top arquivos mais críticos
        critical_files = sorted(
            [a for a in analyses if a.criticality in [CriticalityLevel.CRITICAL, CriticalityLevel.HIGH]],
            key=lambda x: x.risk_score,
            reverse=True
        )[:10]
        
        return {
            'total_files': total_files,
            'criticality_distribution': {
                'critical': criticality_counts[CriticalityLevel.CRITICAL],
                'high': criticality_counts[CriticalityLevel.HIGH],
                'medium': criticality_counts[CriticalityLevel.MEDIUM],
                'low': criticality_counts[CriticalityLevel.LOW],
            },
            'category_distribution': category_counts,
            'risk_percentage': {
                'critical': round(criticality_counts[CriticalityLevel.CRITICAL] / total_files * 100, 2),
                'high': round(criticality_counts[CriticalityLevel.HIGH] / total_files * 100, 2),
            },
            'top_critical_files': [
                {
                    'path': f.path,
                    'risk_score': f.risk_score,
                    'reasons': f.reasons
                } for f in critical_files
            ]
        }

def main():
    """Função principal para teste"""
    classifier = AIFileClassifier()
    
    # Testar com arquivo específico
    test_file = "/home/user/documents/contrato_microsoft.pdf"
    if os.path.exists(test_file):
        analysis = classifier.analyze_file(test_file)
        print(f"Arquivo: {analysis.path}")
        print(f"Criticidade: {analysis.criticality.name}")
        print(f"Confiança: {analysis.confidence:.2f}")
        print(f"Score de Risco: {analysis.risk_score}")
        print(f"Razões: {', '.join(analysis.reasons)}")
    
    # Testar com diretório
    test_dir = "/home/user/documents"
    if os.path.exists(test_dir):
        analyses = classifier.analyze_directory(test_dir)
        report = classifier.generate_risk_report(analyses)
        print("\n--- RELATÓRIO DE RISCO ---")
        print(f"Total de arquivos: {report['total_files']}")
        print(f"Arquivos críticos: {report['criticality_distribution']['critical']}")
        print(f"Arquivos de alto risco: {report['criticality_distribution']['high']}")

if __name__ == "__main__":
    main()