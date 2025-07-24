//! Gestion des clés cryptographiques pour ArchiveChain
//! 
//! Utilise Ed25519 pour les signatures numériques

use serde::{Deserialize, Serialize};
use ed25519_dalek::{Signer, Verifier, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use std::fmt;
use crate::error::{CryptoError, Result};

/// Taille d'une clé publique Ed25519 en bytes
pub const PUBLIC_KEY_SIZE: usize = 32;

/// Taille d'une clé privée Ed25519 en bytes
pub const PRIVATE_KEY_SIZE: usize = 32;

/// Clé publique Ed25519
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKey {
    key: VerifyingKey,
}

/// Clé privée Ed25519
#[derive(Clone, Serialize, Deserialize)]
pub struct PrivateKey {
    key: SigningKey,
}

/// Paire de clés (publique + privée)
#[derive(Clone)]
pub struct KeyPair {
    private_key: PrivateKey,
    public_key: PublicKey,
}

impl PublicKey {
    /// Crée une clé publique à partir de bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != PUBLIC_KEY_SIZE {
            return Err(CryptoError::InvalidPublicKey.into());
        }
        
        let mut array = [0u8; PUBLIC_KEY_SIZE];
        array.copy_from_slice(bytes);
        
        let key = VerifyingKey::from_bytes(&array)
            .map_err(|_| CryptoError::InvalidPublicKey)?;
            
        Ok(Self { key })
    }

    /// Crée une clé publique à partir d'une string hexadécimale
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes = hex::decode(hex_str)?;
        Self::from_bytes(&bytes)
    }

    /// Retourne les bytes de la clé publique
    pub fn as_bytes(&self) -> &[u8; PUBLIC_KEY_SIZE] {
        self.key.as_bytes()
    }

    /// Retourne une représentation hexadécimale
    pub fn to_hex(&self) -> String {
        hex::encode(self.as_bytes())
    }

    /// Vérifie si la clé publique est valide
    pub fn is_valid(&self) -> bool {
        // Une clé publique Ed25519 est toujours valide si elle a été créée avec succès
        true
    }

    /// Obtient la clé interne pour la vérification
    pub(crate) fn inner(&self) -> &VerifyingKey {
        &self.key
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl PrivateKey {
    /// Crée une clé privée à partir de bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != PRIVATE_KEY_SIZE {
            return Err(CryptoError::InvalidPrivateKey.into());
        }
        
        let mut array = [0u8; PRIVATE_KEY_SIZE];
        array.copy_from_slice(bytes);
        
        let key = SigningKey::from_bytes(&array);
        Ok(Self { key })
    }

    /// Crée une clé privée à partir d'une string hexadécimale
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes = hex::decode(hex_str)?;
        Self::from_bytes(&bytes)
    }

    /// Retourne les bytes de la clé privée
    pub fn as_bytes(&self) -> &[u8; PRIVATE_KEY_SIZE] {
        self.key.as_bytes()
    }

    /// Retourne une représentation hexadécimale
    pub fn to_hex(&self) -> String {
        hex::encode(self.as_bytes())
    }

    /// Vérifie si la clé privée est valide
    pub fn is_valid(&self) -> bool {
        // Une clé privée Ed25519 est toujours valide si elle a été créée avec succès
        true
    }

    /// Obtient la clé publique correspondante
    pub fn public_key(&self) -> PublicKey {
        let verifying_key = self.key.verifying_key();
        PublicKey { key: verifying_key }
    }

    /// Obtient la clé interne pour la signature
    pub(crate) fn inner(&self) -> &SigningKey {
        &self.key
    }
}

// Debug pour PrivateKey ne doit pas révéler la clé
impl fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrivateKey")
            .field("key", &"<hidden>")
            .finish()
    }
}

impl KeyPair {
    /// Crée une nouvelle paire de clés
    pub fn new(private_key: PrivateKey, public_key: PublicKey) -> Self {
        Self {
            private_key,
            public_key,
        }
    }

    /// Obtient la clé privée
    pub fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }

    /// Obtient la clé publique
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    /// Sépare la paire en clés individuelles
    pub fn split(self) -> (PrivateKey, PublicKey) {
        (self.private_key, self.public_key)
    }
}

impl fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyPair")
            .field("private_key", &"<hidden>")
            .field("public_key", &self.public_key)
            .finish()
    }
}

/// Génère une nouvelle paire de clés aléatoire
pub fn generate_keypair() -> Result<KeyPair> {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    
    let private_key = PrivateKey { key: signing_key };
    let public_key = PublicKey { key: verifying_key };
    
    Ok(KeyPair::new(private_key, public_key))
}

/// Génère une paire de clés déterministe à partir d'une seed
pub fn generate_keypair_from_seed(seed: &[u8; 32]) -> Result<KeyPair> {
    let signing_key = SigningKey::from_bytes(seed);
    let verifying_key = signing_key.verifying_key();
    
    let private_key = PrivateKey { key: signing_key };
    let public_key = PublicKey { key: verifying_key };
    
    Ok(KeyPair::new(private_key, public_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = generate_keypair().unwrap();
        assert!(keypair.private_key().is_valid());
        assert!(keypair.public_key().is_valid());
    }

    #[test]
    fn test_keypair_from_seed() {
        let seed = [42u8; 32];
        let keypair1 = generate_keypair_from_seed(&seed).unwrap();
        let keypair2 = generate_keypair_from_seed(&seed).unwrap();
        
        // Les paires de clés générées avec la même seed doivent être identiques
        assert_eq!(keypair1.public_key(), keypair2.public_key());
    }

    #[test]
    fn test_public_key_derivation() {
        let keypair = generate_keypair().unwrap();
        let derived_public = keypair.private_key().public_key();
        assert_eq!(*keypair.public_key(), derived_public);
    }

    #[test]
    fn test_public_key_hex_roundtrip() {
        let keypair = generate_keypair().unwrap();
        let hex = keypair.public_key().to_hex();
        let recovered = PublicKey::from_hex(&hex).unwrap();
        assert_eq!(*keypair.public_key(), recovered);
    }

    #[test]
    fn test_private_key_hex_roundtrip() {
        let keypair = generate_keypair().unwrap();
        let hex = keypair.private_key().to_hex();
        let recovered = PrivateKey::from_hex(&hex).unwrap();
        assert_eq!(keypair.private_key().as_bytes(), recovered.as_bytes());
    }

    #[test]
    fn test_invalid_public_key_bytes() {
        let invalid_bytes = vec![0u8; 16]; // Wrong size
        let result = PublicKey::from_bytes(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_private_key_bytes() {
        let invalid_bytes = vec![0u8; 16]; // Wrong size
        let result = PrivateKey::from_bytes(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_keypair_split() {
        let keypair = generate_keypair().unwrap();
        let public_key_orig = keypair.public_key().clone();
        let (private_key, public_key) = keypair.split();
        
        assert_eq!(public_key_orig, public_key);
        assert_eq!(private_key.public_key(), public_key);
    }
}