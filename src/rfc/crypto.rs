//! Cryptographic utilities for RFC authentication
//!
//! This module provides:
//! - HMAC-SHA256 for request signing and verification
//! - RSA-OAEP for secure password exchange
//! - Constant-time comparison to prevent timing attacks
//!
//! # Security Notes
//! - HMAC keys should be at least 32 bytes of random data
//! - RSA keys are generated fresh for each password exchange (ephemeral)
//! - All comparisons use constant-time operations

use hmac::{Hmac, Mac};
use rand::rngs::OsRng;
use rsa::{
    pkcs8::{DecodePublicKey, EncodePublicKey, LineEnding},
    Oaep, RsaPrivateKey, RsaPublicKey,
};
use sha2::Sha256;
use thiserror::Error;

/// HMAC type alias for SHA-256
type HmacSha256 = Hmac<Sha256>;

/// RSA key size in bits
const RSA_KEY_SIZE: usize = 2048;

/// Cryptographic errors
#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("HMAC verification failed")]
    HmacVerificationFailed,

    #[error("Invalid HMAC key length")]
    InvalidKeyLength,

    #[error("RSA key generation failed: {0}")]
    RsaKeyGeneration(String),

    #[error("RSA encryption failed: {0}")]
    RsaEncryption(String),

    #[error("RSA decryption failed: {0}")]
    RsaDecryption(String),

    #[error("Invalid public key format: {0}")]
    InvalidPublicKey(String),

    #[error("Hex encoding/decoding error: {0}")]
    HexError(String),

    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(String),
}

impl From<hex::FromHexError> for CryptoError {
    fn from(e: hex::FromHexError) -> Self {
        CryptoError::HexError(e.to_string())
    }
}

impl From<std::string::FromUtf8Error> for CryptoError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        CryptoError::Utf8Error(e.to_string())
    }
}

// =============================================================================
// HMAC-SHA256 Functions
// =============================================================================

/// Hash data using HMAC-SHA256
///
/// # Arguments
/// * `data` - The data to hash
/// * `password` - The secret key for HMAC
///
/// # Returns
/// Hex-encoded HMAC digest
///
/// # Example
/// ```
/// use oxidized_bio::rfc::crypto::hash_data;
///
/// let hash = hash_data("important data", "secret_key");
/// assert_eq!(hash.len(), 64); // SHA256 produces 32 bytes = 64 hex chars
/// ```
pub fn hash_data(data: &str, password: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(password.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(data.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Verify HMAC-SHA256 hash using constant-time comparison
///
/// # Arguments
/// * `data` - The original data
/// * `hash` - The hex-encoded hash to verify
/// * `password` - The secret key used for HMAC
///
/// # Returns
/// `true` if the hash is valid, `false` otherwise
///
/// # Security
/// Uses constant-time comparison to prevent timing attacks
pub fn verify_data(data: &str, hash: &str, password: &str) -> bool {
    let expected = hash_data(data, password);
    constant_time_compare(expected.as_bytes(), hash.as_bytes())
}

/// Constant-time byte comparison
///
/// Compares two byte slices in constant time to prevent timing attacks.
/// Returns `false` if lengths differ (but still takes O(max(len1, len2)) time).
fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        // Still do a comparison to maintain constant time
        let max_len = std::cmp::max(a.len(), b.len());
        let mut _dummy = 0u8;  // Used to maintain constant time
        for i in 0..max_len {
            let byte_a = a.get(i).copied().unwrap_or(0);
            let byte_b = b.get(i).copied().unwrap_or(0);
            _dummy |= byte_a ^ byte_b;
        }
        // Always return false for different lengths
        return false;
    }

    let mut result = 0u8;
    for (byte_a, byte_b) in a.iter().zip(b.iter()) {
        result |= byte_a ^ byte_b;
    }
    result == 0
}

// =============================================================================
// RSA Functions
// =============================================================================

/// RSA key pair for password exchange
pub struct RsaKeyPair {
    pub private_key: RsaPrivateKey,
    pub public_key: RsaPublicKey,
}

/// Generate a new RSA key pair (2048-bit)
///
/// # Returns
/// A new RSA key pair suitable for password exchange
///
/// # Errors
/// Returns `CryptoError::RsaKeyGeneration` if key generation fails
pub fn generate_key_pair() -> Result<RsaKeyPair, CryptoError> {
    let private_key = RsaPrivateKey::new(&mut OsRng, RSA_KEY_SIZE)
        .map_err(|e| CryptoError::RsaKeyGeneration(e.to_string()))?;
    let public_key = RsaPublicKey::from(&private_key);
    Ok(RsaKeyPair {
        private_key,
        public_key,
    })
}

/// Export public key as PEM-encoded hex string
///
/// # Arguments
/// * `public_key` - The RSA public key to export
///
/// # Returns
/// Hex-encoded PEM string
pub fn export_public_key(public_key: &RsaPublicKey) -> Result<String, CryptoError> {
    let pem = public_key
        .to_public_key_pem(LineEnding::LF)
        .map_err(|e| CryptoError::InvalidPublicKey(e.to_string()))?;
    Ok(hex::encode(pem.as_bytes()))
}

/// Import public key from PEM-encoded hex string
///
/// # Arguments
/// * `hex_pem` - Hex-encoded PEM string
///
/// # Returns
/// The decoded RSA public key
pub fn import_public_key(hex_pem: &str) -> Result<RsaPublicKey, CryptoError> {
    let pem_bytes = hex::decode(hex_pem)?;
    let pem = String::from_utf8(pem_bytes)?;
    RsaPublicKey::from_public_key_pem(&pem)
        .map_err(|e| CryptoError::InvalidPublicKey(e.to_string()))
}

/// Encrypt data using RSA-OAEP with SHA-256
///
/// # Arguments
/// * `data` - The plaintext data to encrypt
/// * `public_key` - The RSA public key
///
/// # Returns
/// Hex-encoded ciphertext
///
/// # Note
/// Data length must be less than (key_size_bytes - 2 * hash_size - 2)
/// For 2048-bit key with SHA-256: max ~190 bytes
pub fn encrypt_data(data: &str, public_key: &RsaPublicKey) -> Result<String, CryptoError> {
    let padding = Oaep::new::<Sha256>();
    let encrypted = public_key
        .encrypt(&mut OsRng, padding, data.as_bytes())
        .map_err(|e| CryptoError::RsaEncryption(e.to_string()))?;
    Ok(hex::encode(encrypted))
}

/// Decrypt data using RSA-OAEP with SHA-256
///
/// # Arguments
/// * `encrypted_hex` - Hex-encoded ciphertext
/// * `private_key` - The RSA private key
///
/// # Returns
/// The decrypted plaintext
pub fn decrypt_data(
    encrypted_hex: &str,
    private_key: &RsaPrivateKey,
) -> Result<String, CryptoError> {
    let encrypted = hex::decode(encrypted_hex)?;
    let padding = Oaep::new::<Sha256>();
    let decrypted = private_key
        .decrypt(padding, &encrypted)
        .map_err(|e| CryptoError::RsaDecryption(e.to_string()))?;
    String::from_utf8(decrypted).map_err(|e| CryptoError::Utf8Error(e.to_string()))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_hash_verify() {
        let data = "test data for hashing";
        let password = "secret_password_123";
        
        let hash = hash_data(data, password);
        
        // Hash should be 64 hex characters (32 bytes)
        assert_eq!(hash.len(), 64);
        
        // Verification should succeed with correct password
        assert!(verify_data(data, &hash, password));
        
        // Verification should fail with wrong password
        assert!(!verify_data(data, &hash, "wrong_password"));
        
        // Verification should fail with modified data
        assert!(!verify_data("modified data", &hash, password));
    }

    #[test]
    fn test_hmac_deterministic() {
        let data = "same data";
        let password = "same_password";
        
        let hash1 = hash_data(data, password);
        let hash2 = hash_data(data, password);
        
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare(b"hello", b"hello"));
        assert!(!constant_time_compare(b"hello", b"world"));
        assert!(!constant_time_compare(b"hello", b"hello!"));
        assert!(!constant_time_compare(b"", b"a"));
        assert!(constant_time_compare(b"", b""));
    }

    #[test]
    fn test_rsa_key_generation() {
        let key_pair = generate_key_pair().expect("Key generation should succeed");
        
        // Verify we can export the public key
        let exported = export_public_key(&key_pair.public_key)
            .expect("Export should succeed");
        
        // Verify we can import it back
        let imported = import_public_key(&exported)
            .expect("Import should succeed");
        
        // Verify the imported key works
        let test_data = "test encryption";
        let encrypted = encrypt_data(test_data, &imported)
            .expect("Encryption should succeed");
        let decrypted = decrypt_data(&encrypted, &key_pair.private_key)
            .expect("Decryption should succeed");
        
        assert_eq!(test_data, decrypted);
    }

    #[test]
    fn test_rsa_encrypt_decrypt() {
        let key_pair = generate_key_pair().expect("Key generation should succeed");
        
        let original = "This is a secret password!";
        let encrypted = encrypt_data(original, &key_pair.public_key)
            .expect("Encryption should succeed");
        
        // Encrypted data should be different from original
        assert_ne!(original, encrypted);
        
        // Decryption should recover original
        let decrypted = decrypt_data(&encrypted, &key_pair.private_key)
            .expect("Decryption should succeed");
        
        assert_eq!(original, decrypted);
    }

    #[test]
    fn test_rsa_different_keys_fail() {
        let key_pair1 = generate_key_pair().expect("Key generation should succeed");
        let key_pair2 = generate_key_pair().expect("Key generation should succeed");
        
        let data = "secret data";
        let encrypted = encrypt_data(data, &key_pair1.public_key)
            .expect("Encryption should succeed");
        
        // Decryption with wrong key should fail
        let result = decrypt_data(&encrypted, &key_pair2.private_key);
        assert!(result.is_err());
    }
}
