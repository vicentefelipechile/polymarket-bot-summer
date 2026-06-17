//! Cryptographic primitives for secure configuration storage.
//!
//! Provides AES-256-GCM authenticated encryption with Argon2id password-based key
//! derivation for the `summer.bot` configuration file. This module only knows how to turn a
//! `SecureConfig` into encrypted bytes and back; the config type itself lives in
//! [`crate::config::secure_config`].

// =========================================================================================================
// Imports
// =========================================================================================================

use std::path::Path;

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::RngCore, PasswordHasher, SaltString},
    Argon2,
};
use zeroize::Zeroizing;

use crate::config::secure_config::SecureConfig;

// =========================================================================================================
// Constants
// =========================================================================================================

// Binary file format: [Salt (16)][Nonce (12)][Ciphertext + Auth Tag].
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

// =========================================================================================================
// Helpers
// =========================================================================================================

/// Derive a 256-bit encryption key from a password using Argon2id.
fn derive_key(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>> {
    if salt.len() != SALT_LEN {
        anyhow::bail!("Invalid salt length");
    }

    let argon2 = Argon2::default();
    let salt_string = SaltString::encode_b64(salt)
        .map_err(|e| anyhow::anyhow!("Failed to encode salt: {}", e))?;

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt_string)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

    // Extract the raw hash bytes (32 bytes for Argon2).
    let hash_bytes = password_hash.hash.context("Missing hash")?;
    let hash_slice = hash_bytes.as_bytes();

    if hash_slice.len() < 32 {
        anyhow::bail!("Hash too short");
    }

    let mut key = Zeroizing::new([0u8; 32]);
    key.copy_from_slice(&hash_slice[..32]);

    Ok(key)
}

// =========================================================================================================
// Encryption / Decryption
// =========================================================================================================

/// Encrypt configuration data with a password.
pub fn encrypt_config(config: &SecureConfig, password: &str) -> Result<Vec<u8>> {
    // Generate random salt.
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);

    // Derive encryption key from password.
    let key_bytes = derive_key(password, &salt)?;
    let key = Key::<Aes256Gcm>::from_slice(&*key_bytes);
    let cipher = Aes256Gcm::new(key);

    // Generate random nonce.
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Serialize configuration.
    let plaintext = bincode::serialize(config).context("Failed to serialize config")?;

    // Encrypt.
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    // Build output: [salt][nonce][ciphertext with auth tag].
    let mut output = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);

    Ok(output)
}

/// Decrypt configuration data with a password.
pub fn decrypt_config(data: &[u8], password: &str) -> Result<SecureConfig> {
    // Validate minimum length.
    if data.len() < SALT_LEN + NONCE_LEN {
        anyhow::bail!("Invalid encrypted data: too short");
    }

    // Extract components.
    let salt = &data[..SALT_LEN];
    let nonce_bytes = &data[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &data[SALT_LEN + NONCE_LEN..];

    // Derive key.
    let key_bytes = derive_key(password, salt)?;
    let key = Key::<Aes256Gcm>::from_slice(&*key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Decrypt.
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("Decryption failed: incorrect password or corrupted data"))?;

    // Deserialize.
    let config: SecureConfig =
        bincode::deserialize(&plaintext).context("Failed to deserialize config")?;

    Ok(config)
}

// =========================================================================================================
// File I/O
// =========================================================================================================

/// Save encrypted configuration to file.
pub fn save_config(config: &SecureConfig, path: &Path, password: &str) -> Result<()> {
    let encrypted = encrypt_config(config, password)?;
    std::fs::write(path, encrypted).context("Failed to write config file")?;
    Ok(())
}

/// Load encrypted configuration from file.
pub fn load_config(path: &Path, password: &str) -> Result<SecureConfig> {
    let encrypted = std::fs::read(path).context("Failed to read config file")?;
    decrypt_config(&encrypted, password)
}

/// Change the password for an encrypted configuration file.
pub fn change_password(path: &Path, old_password: &str, new_password: &str) -> Result<()> {
    let config = load_config(path, old_password)?;
    save_config(&config, path, new_password)?;
    Ok(())
}

// =========================================================================================================
// Tests
// =========================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::secure_config::AiPersonality;

    fn test_config() -> SecureConfig {
        SecureConfig {
            private_key: "0x1234567890abcdef".to_string(),
            max_order_size: 100.0,
            min_order_size: 1.0,
            volume_velocity_threshold: 1000.0,
            obi_threshold: 0.3,
            database_path: "./test.db".to_string(),
            rpc_url: Some("https://polygon-rpc.com".to_string()),
            gemini_api_key: Some("test_api_key".to_string()),
            ai_personality: AiPersonality::Summer,
            ai_enabled: true,
            ai_analysis_frequency_minutes: 30,
        }
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let config = test_config();
        let password = "test_password_123";

        let encrypted = encrypt_config(&config, password).unwrap();
        let decrypted = decrypt_config(&encrypted, password).unwrap();

        assert_eq!(config.private_key, decrypted.private_key);
        assert_eq!(config.max_order_size, decrypted.max_order_size);
        assert_eq!(config.ai_personality, decrypted.ai_personality);
    }

    #[test]
    fn test_wrong_password_fails() {
        let config = test_config();
        let password = "correct_password";
        let wrong_password = "wrong_password";

        let encrypted = encrypt_config(&config, password).unwrap();
        let result = decrypt_config(&encrypted, wrong_password);

        assert!(result.is_err());
    }

    #[test]
    fn test_different_salts() {
        let config = test_config();
        let password = "same_password";

        let encrypted1 = encrypt_config(&config, password).unwrap();
        let encrypted2 = encrypt_config(&config, password).unwrap();

        // Different salts mean different ciphertexts.
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt correctly.
        let decrypted1 = decrypt_config(&encrypted1, password).unwrap();
        let decrypted2 = decrypt_config(&encrypted2, password).unwrap();

        assert_eq!(decrypted1.private_key, decrypted2.private_key);
    }

    #[test]
    fn test_change_password() {
        let config = test_config();
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_summer.bot");

        let old_pass = "old_password";
        let new_pass = "new_password";

        // Save with old password.
        save_config(&config, &config_path, old_pass).unwrap();

        // Change password.
        change_password(&config_path, old_pass, new_pass).unwrap();

        // Old password should fail.
        assert!(load_config(&config_path, old_pass).is_err());

        // New password should work.
        let loaded = load_config(&config_path, new_pass).unwrap();
        assert_eq!(config.private_key, loaded.private_key);

        // Cleanup.
        std::fs::remove_file(config_path).ok();
    }
}
