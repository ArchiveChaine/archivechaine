//! Module cryptographique pour ArchiveChain
//! 
//! Fournit les primitives cryptographiques essentielles :
//! - Fonctions de hachage (Blake3, SHA-3)
//! - Signatures numériques (Ed25519)
//! - Gestion des clés
//! - Arbres de Merkle

pub mod hash;
pub mod signature;
pub mod keys;

pub use hash::{Hash, HashAlgorithm, compute_hash, compute_blake3, compute_sha3, compute_combined_hash, Hashable};
pub use signature::{Signature, verify_signature, sign_data, Signable};
pub use keys::{PublicKey, PrivateKey, KeyPair, generate_keypair};

use crate::error::{CryptoError, Result};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_module_basic() {
        let data = b"test data";
        let hash = compute_blake3(data);
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_keypair_generation() {
        let keypair = generate_keypair().unwrap();
        assert!(keypair.public_key().is_valid());
        assert!(keypair.private_key().is_valid());
    }

    #[test]
    fn test_sign_verify_roundtrip() {
        let keypair = generate_keypair().unwrap();
        let data = b"message to sign";
        
        let signature = sign_data(data, keypair.private_key()).unwrap();
        let is_valid = verify_signature(data, &signature, keypair.public_key()).unwrap();
        
        assert!(is_valid);
    }
}