//! Configuration domain: the `SecureConfig` model and its encrypted persistence.

// =========================================================================================================
// Submodules
// =========================================================================================================

pub mod crypto;
pub mod secure_config;

// =========================================================================================================
// Re-exports
// =========================================================================================================

pub use crypto::{change_password, decrypt_config, encrypt_config, load_config, save_config};
pub use secure_config::{AiPersonality, SecureConfig};
