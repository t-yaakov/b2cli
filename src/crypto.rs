//! Módulo de criptografia para proteção de dados sensíveis
//!
//! Este módulo fornece:
//! - Criptografia AES-256-GCM para dados
//! - Derivação de chaves com Argon2
//! - Gerenciamento seguro de senhas e credenciais
//! - Serialização/deserialização segura de dados criptografados

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use argon2::{
    password_hash::{rand_core::RngCore, PasswordHasher, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Gerenciador central de criptografia do B2CLI
/// 
/// Responsável por:
/// - Gerenciar chaves de criptografia de forma segura
/// - Criptografar/descriptografar credenciais de cloud providers
/// - Proteger dados sensíveis em trânsito e em repouso
/// - Garantir que senhas nunca sejam armazenadas em texto plano
pub struct CryptoManager {
    /// Chave de criptografia derivada da senha mestra
    key: Arc<RwLock<Option<Key<Aes256Gcm>>>>,
    /// Salt para derivação de chave
    salt: String,
}

impl CryptoManager {
    /// Cria um novo gerenciador de criptografia
    /// 
    /// Gera automaticamente um salt aleatório para derivação de chaves
    pub fn new() -> Self {
        Self {
            key: Arc::new(RwLock::new(None)),
            salt: Self::generate_salt(),
        }
    }

    /// Gera um salt aleatório criptograficamente seguro
    /// 
    /// Usa o gerador de números aleatórios do sistema operacional
    fn generate_salt() -> String {
        let salt = SaltString::generate(&mut OsRng);
        salt.to_string()
    }

    /// Inicializa o gerenciador com uma senha mestra
    /// 
    /// A senha é usada para derivar a chave de criptografia usando Argon2.
    /// Esta chave será usada para criptografar todas as credenciais.
    /// 
    /// # Arguments
    /// 
    /// * `password` - Senha mestra para derivação da chave
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` - Se a inicialização foi bem-sucedida
    /// * `Err(e)` - Se houve erro na derivação da chave
    pub async fn init_with_password(&self, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Inicializando gerenciador de criptografia");
        
        // Derivar chave da senha usando Argon2
        let argon2 = Argon2::default();
        let salt = SaltString::from_b64(&self.salt)
            .map_err(|e| format!("Erro ao decodificar salt: {}", e))?;
        
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Erro ao derivar chave: {}", e))?
            .to_string();
        
        // Usar os primeiros 32 bytes do hash como chave AES
        let key_bytes = &password_hash.as_bytes()[..32];
        let key = Key::<Aes256Gcm>::from_slice(key_bytes);
        
        let mut stored_key = self.key.write().await;
        *stored_key = Some(*key);
        
        info!("Chave de criptografia derivada com sucesso");
        Ok(())
    }

    /// Criptografa um texto
    pub async fn encrypt(&self, plaintext: &str) -> Result<String, Box<dyn std::error::Error>> {
        let key_lock = self.key.read().await;
        let key = key_lock
            .as_ref()
            .ok_or("Criptografia não inicializada")?;
        
        let cipher = Aes256Gcm::new(key);
        
        // Gerar nonce aleatório
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Criptografar
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| format!("Erro ao criptografar: {}", e))?;
        
        // Combinar nonce + ciphertext
        let mut combined = Vec::new();
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        
        // Codificar em base64
        Ok(STANDARD.encode(combined))
    }

    /// Descriptografa um texto
    pub async fn decrypt(&self, ciphertext: &str) -> Result<String, Box<dyn std::error::Error>> {
        let key_lock = self.key.read().await;
        let key = key_lock
            .as_ref()
            .ok_or("Criptografia não inicializada")?;
        
        let cipher = Aes256Gcm::new(key);
        
        // Decodificar de base64
        let combined = STANDARD.decode(ciphertext)?;
        
        // Separar nonce e ciphertext
        if combined.len() < 12 {
            return Err("Ciphertext inválido".into());
        }
        
        let (nonce_bytes, ciphertext_bytes) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        // Descriptografar
        let plaintext = cipher
            .decrypt(nonce, ciphertext_bytes)
            .map_err(|e| format!("Erro ao descriptografar: {}", e))?;
        
        Ok(String::from_utf8(plaintext)?)
    }

    /// Criptografa um caminho de arquivo
    pub async fn encrypt_path(&self, path: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Para caminhos, podemos querer manter a estrutura parcialmente visível
        // Por exemplo: /home/user/documents/contract.pdf
        // Poderia virar: /home/user/[encrypted]/[encrypted].pdf
        
        // Por enquanto, criptografar o caminho completo
        self.encrypt(path).await
    }

    /// Descriptografa um caminho de arquivo
    pub async fn decrypt_path(&self, encrypted_path: &str) -> Result<String, Box<dyn std::error::Error>> {
        self.decrypt(encrypted_path).await
    }

    /// Verifica se a criptografia está ativa
    pub async fn is_enabled(&self) -> bool {
        self.key.read().await.is_some()
    }
}

/// Estrutura para armazenar configuração de criptografia
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoConfig {
    /// Se a criptografia está habilitada
    pub enabled: bool,
    
    /// Salt para derivação de chave (base64)
    pub salt: String,
    
    /// Algoritmo usado
    pub algorithm: String,
    
    /// Versão do esquema de criptografia
    pub version: u32,
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            salt: SaltString::generate(&mut OsRng).to_string(),
            algorithm: "AES-256-GCM".to_string(),
            version: 1,
        }
    }
}

/// Helper para criptografar dados sensíveis em structs
#[derive(Debug, Clone)]
pub struct EncryptedField {
    /// Valor criptografado
    pub ciphertext: String,
    /// Se está criptografado
    pub is_encrypted: bool,
}

impl EncryptedField {
    /// Cria um campo não criptografado
    pub fn plain(value: String) -> Self {
        Self {
            ciphertext: value,
            is_encrypted: false,
        }
    }

    /// Cria um campo criptografado
    pub fn encrypted(ciphertext: String) -> Self {
        Self {
            ciphertext,
            is_encrypted: true,
        }
    }

    /// Obtém o valor, descriptografando se necessário
    pub async fn get_value(&self, crypto: &CryptoManager) -> Result<String, Box<dyn std::error::Error>> {
        if self.is_encrypted {
            crypto.decrypt(&self.ciphertext).await
        } else {
            Ok(self.ciphertext.clone())
        }
    }

    /// Define o valor, criptografando se o crypto manager estiver ativo
    pub async fn set_value(
        &mut self,
        value: String,
        crypto: &CryptoManager,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if crypto.is_enabled().await {
            self.ciphertext = crypto.encrypt(&value).await?;
            self.is_encrypted = true;
        } else {
            self.ciphertext = value;
            self.is_encrypted = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encryption_decryption() {
        let crypto = CryptoManager::new();
        crypto.init_with_password("test_password_123").await.unwrap();

        let plaintext = "/home/user/documents/secret_file.pdf";
        let encrypted = crypto.encrypt(plaintext).await.unwrap();
        let decrypted = crypto.decrypt(&encrypted).await.unwrap();

        assert_eq!(plaintext, decrypted);
        assert_ne!(plaintext, encrypted);
    }

    #[tokio::test]
    async fn test_encrypted_field() {
        let crypto = CryptoManager::new();
        crypto.init_with_password("test_password_123").await.unwrap();

        let mut field = EncryptedField::plain("sensitive_data".to_string());
        assert!(!field.is_encrypted);

        field.set_value("new_sensitive_data".to_string(), &crypto).await.unwrap();
        assert!(field.is_encrypted);

        let value = field.get_value(&crypto).await.unwrap();
        assert_eq!(value, "new_sensitive_data");
    }
}