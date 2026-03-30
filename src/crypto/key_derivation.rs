use argon2::{Algorithm, Argon2, Params, Version};
use rand::rngs::OsRng;
use rand::RngCore;

use super::CryptoError;
use crate::models::DerivedKey;

/// Derives a 32-byte key from a password and salt using Argon2id.
pub fn derive_key(password: &[u8], salt: &[u8; 16]) -> Result<DerivedKey, CryptoError> {
    // 64MB memory, 3 iterations — strong protection against offline brute-force
    let params = Params::new(65536, 3, 1, Some(32)).map_err(|_| CryptoError::KeyDerivationFailed)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key_bytes = [0u8; 32];
    argon2
        .hash_password_into(password, salt, &mut key_bytes)
        .map_err(|_| CryptoError::KeyDerivationFailed)?;

    Ok(DerivedKey { key_bytes })
}

/// Generates a random 16-byte salt.
pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    salt
}
