pub mod encryption;
pub mod key_derivation;
pub mod secure_memory;

use thiserror::Error;

#[derive(Debug, Clone, Error)]
#[allow(dead_code)]
pub enum CryptoError {
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Decryption failed (wrong password?)")]
    DecryptionFailed,
    #[error("Key derivation failed")]
    KeyDerivationFailed,
    #[error("Invalid data format")]
    InvalidFormat,
}
