//! Fonctions de hachage pour ArchiveChain
//! 
//! Implémente Blake3 (performance) et SHA-3 (standard) pour différents cas d'usage

use serde::{Deserialize, Serialize};
use std::fmt;
use crate::error::{CryptoError, Result};

/// Taille standard d'un hash en bytes
pub const HASH_SIZE: usize = 32;

/// Représentation d'un hash de 256 bits
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash([u8; HASH_SIZE]);

/// Algorithmes de hachage supportés
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashAlgorithm {
    /// Blake3 - Rapide, cryptographiquement sûr
    Blake3,
    /// SHA-3 256 - Standard NIST
    Sha3,
}

impl Hash {
    /// Crée un nouveau hash à partir d'un array de bytes
    pub fn new(data: [u8; HASH_SIZE]) -> Self {
        Self(data)
    }

    /// Crée un hash à partir d'un slice de bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != HASH_SIZE {
            return Err(CryptoError::InvalidHashLength {
                expected: HASH_SIZE,
                actual: bytes.len(),
            }.into());
        }
        let mut array = [0u8; HASH_SIZE];
        array.copy_from_slice(bytes);
        Ok(Self(array))
    }

    /// Crée un hash à partir d'un array de bytes (version infaillible)
    pub fn from_bytes_array(bytes: [u8; HASH_SIZE]) -> Self {
        Self(bytes)
    }

    /// Crée un hash à partir d'une string hexadécimale
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes = hex::decode(hex_str)?;
        Self::from_bytes(&bytes)
    }

    /// Retourne les bytes du hash
    pub fn as_bytes(&self) -> &[u8; HASH_SIZE] {
        &self.0
    }

    /// Retourne une représentation hexadécimale
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Hash vide (utilisé pour les tests et cas spéciaux)
    pub fn zero() -> Self {
        Self([0u8; HASH_SIZE])
    }

    /// Vérifie si le hash est vide
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; HASH_SIZE]
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Calcule un hash Blake3 des données
pub fn compute_blake3(data: &[u8]) -> Hash {
    let hash_bytes = blake3::hash(data);
    Hash::new(*hash_bytes.as_bytes())
}

/// Calcule un hash SHA-3 256 des données
pub fn compute_sha3(data: &[u8]) -> Hash {
    use sha3::{Digest, Sha3_256};
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    let result = hasher.finalize();
    Hash::new(result.into())
}

/// Calcule un hash selon l'algorithme spécifié
pub fn compute_hash(data: &[u8], algorithm: HashAlgorithm) -> Hash {
    match algorithm {
        HashAlgorithm::Blake3 => compute_blake3(data),
        HashAlgorithm::Sha3 => compute_sha3(data),
    }
}

/// Calcule un hash de plusieurs éléments concaténés
pub fn compute_combined_hash(elements: &[&[u8]], algorithm: HashAlgorithm) -> Hash {
    let combined: Vec<u8> = elements.iter().flat_map(|&e| e.iter()).cloned().collect();
    compute_hash(&combined, algorithm)
}

/// Hash double pour une sécurité renforcée
pub fn compute_double_hash(data: &[u8], algorithm: HashAlgorithm) -> Hash {
    let first_hash = compute_hash(data, algorithm);
    compute_hash(first_hash.as_bytes(), algorithm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_creation() {
        let data = [1u8; HASH_SIZE];
        let hash = Hash::new(data);
        assert_eq!(hash.as_bytes(), &data);
    }

    #[test]
    fn test_hash_from_bytes() {
        let bytes = vec![0u8; HASH_SIZE];
        let hash = Hash::from_bytes(&bytes).unwrap();
        assert!(hash.is_zero());
    }

    #[test]
    fn test_hash_from_invalid_bytes() {
        let bytes = vec![0u8; 16]; // Wrong size
        let result = Hash::from_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_hex_roundtrip() {
        let original = Hash::new([0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                                  0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                                  0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                                  0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]);
        let hex = original.to_hex();
        let recovered = Hash::from_hex(&hex).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_blake3_hash() {
        let data = b"test data for hashing";
        let hash = compute_blake3(data);
        assert!(!hash.is_zero());
        assert_eq!(hash.as_bytes().len(), HASH_SIZE);
    }

    #[test]
    fn test_sha3_hash() {
        let data = b"test data for hashing";
        let hash = compute_sha3(data);
        assert!(!hash.is_zero());
        assert_eq!(hash.as_bytes().len(), HASH_SIZE);
    }

    #[test]
    fn test_hash_consistency() {
        let data = b"consistent test data";
        let hash1 = compute_blake3(data);
        let hash2 = compute_blake3(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_algorithms() {
        let data = b"test data";
        let blake3_hash = compute_blake3(data);
        let sha3_hash = compute_sha3(data);
        // Les hashes doivent être différents avec des algorithmes différents
        assert_ne!(blake3_hash, sha3_hash);
    }

    #[test]
    fn test_combined_hash() {
        let elements = [b"part1".as_slice(), b"part2".as_slice(), b"part3".as_slice()];
        let hash = compute_combined_hash(&elements, HashAlgorithm::Blake3);
        assert!(!hash.is_zero());
    }

    #[test]
    fn test_double_hash() {
        let data = b"data to double hash";
        let single = compute_blake3(data);
        let double = compute_double_hash(data, HashAlgorithm::Blake3);
        assert_ne!(single, double);
    }
}

/// Trait pour les types qui peuvent être hashés
pub trait Hashable {
    /// Calcule le hash de l'objet
    fn hash(&self) -> Hash;
    
    /// Calcule le hash avec un algorithme spécifique
    fn hash_with_algorithm(&self, algorithm: HashAlgorithm) -> Hash;
}

/// Implémentation par défaut pour les types qui implémentent AsRef<[u8]>
impl<T: AsRef<[u8]>> Hashable for T {
    fn hash(&self) -> Hash {
        compute_blake3(self.as_ref())
    }
    
    fn hash_with_algorithm(&self, algorithm: HashAlgorithm) -> Hash {
        compute_hash(self.as_ref(), algorithm)
    }
}