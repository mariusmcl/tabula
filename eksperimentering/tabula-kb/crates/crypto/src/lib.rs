//! Cryptographic primitives for the blockchain.
//!
//! - Ed25519 key generation, signing, and verification
//! - SHA-256 hashing utilities

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use rand::rngs::OsRng;

/// Ed25519 keypair for signing transactions.
pub struct Keypair {
    signing_key: SigningKey,
}

impl Keypair {
    /// Generate a new random keypair.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// Create keypair from a 32-byte seed (deterministic).
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        Self { signing_key }
    }

    /// Get the public key bytes.
    pub fn public_key(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// Sign a message, returning a 64-byte signature.
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let signature = self.signing_key.sign(message);
        signature.to_bytes()
    }

    /// Get the secret key bytes (use carefully!).
    pub fn secret_key(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }
}

impl Clone for Keypair {
    fn clone(&self) -> Self {
        Self::from_seed(&self.signing_key.to_bytes())
    }
}

/// Verify an Ed25519 signature.
pub fn verify(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    let Ok(verifying_key) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let sig = Signature::from_bytes(signature);
    verifying_key.verify(message, &sig).is_ok()
}

/// Compute SHA-256 hash.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Compute double SHA-256 hash (common in Bitcoin-style PoW).
pub fn double_sha256(data: &[u8]) -> [u8; 32] {
    sha256(&sha256(data))
}

/// Encode bytes as hex string.
pub fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Decode hex string to bytes.
pub fn hex_decode(s: &str) -> Result<Vec<u8>, HexError> {
    if s.len() % 2 != 0 {
        return Err(HexError::OddLength);
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| HexError::InvalidChar))
        .collect()
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum HexError {
    #[error("hex string has odd length")]
    OddLength,
    #[error("invalid hex character")]
    InvalidChar,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp1 = Keypair::generate();
        let kp2 = Keypair::generate();
        assert_ne!(kp1.public_key(), kp2.public_key());
    }

    #[test]
    fn test_keypair_from_seed() {
        let seed = [42u8; 32];
        let kp1 = Keypair::from_seed(&seed);
        let kp2 = Keypair::from_seed(&seed);
        assert_eq!(kp1.public_key(), kp2.public_key());
    }

    #[test]
    fn test_sign_and_verify() {
        let kp = Keypair::generate();
        let message = b"hello world";
        let signature = kp.sign(message);
        assert!(verify(&kp.public_key(), message, &signature));
    }

    #[test]
    fn test_verify_wrong_message() {
        let kp = Keypair::generate();
        let signature = kp.sign(b"hello");
        assert!(!verify(&kp.public_key(), b"world", &signature));
    }

    #[test]
    fn test_verify_wrong_key() {
        let kp1 = Keypair::generate();
        let kp2 = Keypair::generate();
        let signature = kp1.sign(b"hello");
        assert!(!verify(&kp2.public_key(), b"hello", &signature));
    }

    #[test]
    fn test_sha256() {
        let hash = sha256(b"hello");
        // Known SHA-256 of "hello"
        assert_eq!(
            hex_encode(&hash),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_double_sha256() {
        let hash = double_sha256(b"hello");
        // SHA-256(SHA-256("hello"))
        assert_eq!(
            hex_encode(&hash),
            "9595c9df90075148eb06860365df33584b75bff782a510c6cd4883a419833d50"
        );
    }

    #[test]
    fn test_hex_roundtrip() {
        let data = [0x12, 0x34, 0xab, 0xcd];
        let hex = hex_encode(&data);
        assert_eq!(hex, "1234abcd");
        let decoded = hex_decode(&hex).unwrap();
        assert_eq!(decoded, data);
    }
}
