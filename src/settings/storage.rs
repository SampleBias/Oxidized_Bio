//! Secure Settings Storage
//! 
//! Provides encrypted file-based storage for user settings.
//! Uses AES-256-GCM for encryption of sensitive data like API keys.

use super::{UserSettings, ProviderConfig};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::path::PathBuf;
use tokio::fs;
use tracing::{info, warn, error};

const SETTINGS_FILE: &str = "settings.json";
const ENCRYPTION_KEY_FILE: &str = ".settings_key";
const NONCE_SIZE: usize = 12;

/// Settings storage manager
pub struct SettingsStorage {
    settings_path: PathBuf,
    key_path: PathBuf,
}

impl SettingsStorage {
    /// Create a new settings storage manager
    pub fn new() -> Self {
        // Use XDG data directory or fallback to current directory
        let base_dir = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".local/share"))
                    .unwrap_or_else(|_| PathBuf::from("."))
            })
            .join("oxidized-bio");
        
        Self {
            settings_path: base_dir.join(SETTINGS_FILE),
            key_path: base_dir.join(ENCRYPTION_KEY_FILE),
        }
    }

    /// Create storage with custom path (useful for Docker/testing)
    pub fn with_path(base_dir: PathBuf) -> Self {
        Self {
            settings_path: base_dir.join(SETTINGS_FILE),
            key_path: base_dir.join(ENCRYPTION_KEY_FILE),
        }
    }

    /// Ensure the storage directory exists
    async fn ensure_dir(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.settings_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        Ok(())
    }

    /// Get or create the encryption key
    async fn get_or_create_key(&self) -> anyhow::Result<[u8; 32]> {
        self.ensure_dir().await?;
        
        if self.key_path.exists() {
            let key_data = fs::read(&self.key_path).await?;
            let key_bytes = BASE64.decode(&key_data)?;
            if key_bytes.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&key_bytes);
                return Ok(key);
            }
        }
        
        // Generate new key
        let key: [u8; 32] = rand::random();
        let key_b64 = BASE64.encode(&key);
        fs::write(&self.key_path, key_b64).await?;
        
        // Set restrictive permissions on the key file (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&self.key_path, perms)?;
        }
        
        info!("Generated new encryption key for settings");
        Ok(key)
    }

    /// Encrypt a string value
    fn encrypt(&self, plaintext: &str, key: &[u8; 32]) -> anyhow::Result<String> {
        let cipher = Aes256Gcm::new_from_slice(key)?;
        let nonce_bytes: [u8; NONCE_SIZE] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
        
        // Combine nonce + ciphertext and base64 encode
        let mut combined = nonce_bytes.to_vec();
        combined.extend(ciphertext);
        Ok(BASE64.encode(&combined))
    }

    /// Decrypt a string value
    fn decrypt(&self, encrypted: &str, key: &[u8; 32]) -> anyhow::Result<String> {
        let combined = BASE64.decode(encrypted)?;
        if combined.len() < NONCE_SIZE {
            return Err(anyhow::anyhow!("Invalid encrypted data"));
        }
        
        let (nonce_bytes, ciphertext) = combined.split_at(NONCE_SIZE);
        let cipher = Aes256Gcm::new_from_slice(key)?;
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;
        
        String::from_utf8(plaintext).map_err(Into::into)
    }

    /// Load settings from disk
    pub async fn load(&self) -> anyhow::Result<UserSettings> {
        if !self.settings_path.exists() {
            info!("No settings file found, using defaults");
            return Ok(UserSettings::default());
        }

        let key = self.get_or_create_key().await?;
        let content = fs::read_to_string(&self.settings_path).await?;
        let mut settings: UserSettings = serde_json::from_str(&content)?;
        
        // Decrypt API keys
        self.decrypt_provider_key(&mut settings.openai, &key)?;
        self.decrypt_provider_key(&mut settings.anthropic, &key)?;
        self.decrypt_provider_key(&mut settings.google, &key)?;
        self.decrypt_provider_key(&mut settings.openrouter, &key)?;
        self.decrypt_provider_key(&mut settings.groq, &key)?;
        
        info!("Loaded settings from {:?}", self.settings_path);
        Ok(settings)
    }

    /// Save settings to disk
    pub async fn save(&self, settings: &UserSettings) -> anyhow::Result<()> {
        self.ensure_dir().await?;
        let key = self.get_or_create_key().await?;
        
        // Clone settings and encrypt API keys
        let mut encrypted_settings = settings.clone();
        self.encrypt_provider_key(&mut encrypted_settings.openai, &key)?;
        self.encrypt_provider_key(&mut encrypted_settings.anthropic, &key)?;
        self.encrypt_provider_key(&mut encrypted_settings.google, &key)?;
        self.encrypt_provider_key(&mut encrypted_settings.openrouter, &key)?;
        self.encrypt_provider_key(&mut encrypted_settings.groq, &key)?;
        
        let content = serde_json::to_string_pretty(&encrypted_settings)?;
        fs::write(&self.settings_path, content).await?;
        
        info!("Saved settings to {:?}", self.settings_path);
        Ok(())
    }

    fn encrypt_provider_key(&self, config: &mut ProviderConfig, key: &[u8; 32]) -> anyhow::Result<()> {
        if let Some(api_key) = &config.api_key {
            if !api_key.is_empty() {
                config.api_key = Some(self.encrypt(api_key, key)?);
            }
        }
        Ok(())
    }

    fn decrypt_provider_key(&self, config: &mut ProviderConfig, key: &[u8; 32]) -> anyhow::Result<()> {
        if let Some(encrypted_key) = &config.api_key {
            if !encrypted_key.is_empty() {
                match self.decrypt(encrypted_key, key) {
                    Ok(decrypted) => config.api_key = Some(decrypted),
                    Err(e) => {
                        warn!("Failed to decrypt API key, it may be corrupted: {}", e);
                        config.api_key = None;
                    }
                }
            }
        }
        Ok(())
    }

    /// Get API key for a specific provider
    pub async fn get_api_key(&self, provider: &str) -> anyhow::Result<Option<String>> {
        let settings = self.load().await?;
        let key = match provider.to_lowercase().as_str() {
            "openai" => settings.openai.api_key,
            "anthropic" => settings.anthropic.api_key,
            "google" => settings.google.api_key,
            "openrouter" => settings.openrouter.api_key,
            "groq" => settings.groq.api_key,
            _ => None,
        };
        Ok(key)
    }
}

impl Default for SettingsStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_settings_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SettingsStorage::with_path(temp_dir.path().to_path_buf());
        
        let mut settings = UserSettings::default();
        settings.openai.api_key = Some("sk-test-key-12345".to_string());
        settings.anthropic.api_key = Some("sk-ant-key-67890".to_string());
        
        // Save
        storage.save(&settings).await.unwrap();
        
        // Load
        let loaded = storage.load().await.unwrap();
        
        assert_eq!(loaded.openai.api_key, Some("sk-test-key-12345".to_string()));
        assert_eq!(loaded.anthropic.api_key, Some("sk-ant-key-67890".to_string()));
    }

    #[tokio::test]
    async fn test_encryption() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SettingsStorage::with_path(temp_dir.path().to_path_buf());
        
        let key = storage.get_or_create_key().await.unwrap();
        let plaintext = "secret-api-key-12345";
        
        let encrypted = storage.encrypt(plaintext, &key).unwrap();
        assert_ne!(encrypted, plaintext);
        
        let decrypted = storage.decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
