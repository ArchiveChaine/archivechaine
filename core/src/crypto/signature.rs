//! Module de signatures numériques pour ArchiveChain
//! 
//! Utilise Ed25519 pour signer et vérifier des données

use serde::{Deserialize, Serialize};
use ed25519_dalek::{Signer, Verifier};
use std::fmt;
use crate::error::{CryptoError, Result};
use super::keys::{PublicKey, PrivateKey};

/// Taille d'une signature Ed25519 en bytes
pub const SIGNATURE_SIZE: usize = 64;

/// Signature numérique Ed25519
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    bytes: [u8; SIGNATURE_SIZE],
}

impl Signature {
    /// Crée une signature à partir d'un array de bytes
    pub fn new(bytes: [u8; SIGNATURE_SIZE]) -> Self {
        Self { bytes }
    }

    /// Crée une signature à partir d'un slice de bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != SIGNATURE_SIZE {
            return Err(CryptoError::InvalidSignature.into());
        }
        
        let mut array = [0u8; SIGNATURE_SIZE];
        array.copy_from_slice(bytes);
        Ok(Self { bytes: array })
    }

    /// Crée une signature à partir d'une string hexadécimale
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes = hex::decode(hex_str)?;
        Self::from_bytes(&bytes)
    }

    /// Retourne les bytes de la signature
    pub fn as_bytes(&self) -> &[u8; SIGNATURE_SIZE] {
        &self.bytes
    }

    /// Retourne une représentation hexadécimale
    pub fn to_hex(&self) -> String {
        hex::encode(self.bytes)
    }

    /// Signature vide (utilisée pour les tests)
    pub fn zero() -> Self {
        Self {
            bytes: [0u8; SIGNATURE_SIZE],
        }
    }

    /// Vérifie si la signature est vide
    pub fn is_zero(&self) -> bool {
        self.bytes == [0u8; SIGNATURE_SIZE]
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

/// Signe des données avec une clé privée
pub fn sign_data(data: &[u8], private_key: &PrivateKey) -> Result<Signature> {
    let signature = private_key.inner().sign(data);
    Ok(Signature::new(signature.to_bytes()))
}

/// Vérifie une signature avec une clé publique
pub fn verify_signature(data: &[u8], signature: &Signature, public_key: &PublicKey) -> Result<bool> {
    let ed25519_signature = ed25519_dalek::Signature::from_bytes(signature.as_bytes());
    
    match public_key.inner().verify(data, &ed25519_signature) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Structure pour un message signé
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedMessage<T> {
    /// Le message original
    pub message: T,
    /// La signature du message sérialisé
    pub signature: Signature,
    /// La clé publique du signataire
    pub signer: PublicKey,
}

impl<T> SignedMessage<T> 
where
    T: Serialize,
{
    /// Crée un nouveau message signé
    pub fn new(message: T, private_key: &PrivateKey) -> Result<Self> {
        // Sérialise le message pour le signer
        let serialized = bincode::serialize(&message)
            .map_err(|e| CryptoError::RandomGeneration(e.to_string()))?;
        
        let signature = sign_data(&serialized, private_key)?;
        let signer = private_key.public_key();
        
        Ok(Self {
            message,
            signature,
            signer,
        })
    }

    /// Vérifie la signature du message
    pub fn verify(&self) -> Result<bool> {
        let serialized = bincode::serialize(&self.message)
            .map_err(|e| CryptoError::RandomGeneration(e.to_string()))?;
        
        verify_signature(&serialized, &self.signature, &self.signer)
    }

    /// Obtient le message s'il est valide
    pub fn into_message_if_valid(self) -> Result<T> {
        if self.verify()? {
            Ok(self.message)
        } else {
            Err(CryptoError::InvalidSignature.into())
        }
    }
}

/// Batch de vérification de signatures pour l'efficacité
pub struct SignatureBatch {
    items: Vec<(Vec<u8>, Signature, PublicKey)>,
}

impl SignatureBatch {
    /// Crée un nouveau batch vide
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
        }
    }

    /// Ajoute une signature à vérifier au batch
    pub fn add(&mut self, data: &[u8], signature: Signature, public_key: PublicKey) {
        self.items.push((data.to_vec(), signature, public_key));
    }

    /// Vérifie toutes les signatures du batch
    /// Retourne true seulement si toutes les signatures sont valides
    pub fn verify_all(&self) -> Result<bool> {
        for (data, signature, public_key) in &self.items {
            if !verify_signature(data, signature, public_key)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Vérifie les signatures individuellement
    /// Retourne un vecteur de résultats pour chaque signature
    pub fn verify_individual(&self) -> Result<Vec<bool>> {
        let mut results = Vec::with_capacity(self.items.len());
        
        for (data, signature, public_key) in &self.items {
            let is_valid = verify_signature(data, signature, public_key)?;
            results.push(is_valid);
        }
        
        Ok(results)
    }

    /// Retourne le nombre de signatures dans le batch
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Vérifie si le batch est vide
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for SignatureBatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::generate_keypair;

    #[test]
    fn test_signature_creation() {
        let bytes = [42u8; SIGNATURE_SIZE];
        let signature = Signature::new(bytes);
        assert_eq!(signature.as_bytes(), &bytes);
    }

    #[test]
    fn test_signature_from_bytes() {
        let bytes = vec![1u8; SIGNATURE_SIZE];
        let signature = Signature::from_bytes(&bytes).unwrap();
        assert_eq!(signature.as_bytes(), &bytes[..]);
    }

    #[test]
    fn test_signature_from_invalid_bytes() {
        let bytes = vec![0u8; 32]; // Wrong size
        let result = Signature::from_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_signature_hex_roundtrip() {
        let bytes = [0x12u8; SIGNATURE_SIZE];
        let signature = Signature::new(bytes);
        let hex = signature.to_hex();
        let recovered = Signature::from_hex(&hex).unwrap();
        assert_eq!(signature, recovered);
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = generate_keypair().unwrap();
        let data = b"test message to sign";
        
        let signature = sign_data(data, keypair.private_key()).unwrap();
        let is_valid = verify_signature(data, &signature, keypair.public_key()).unwrap();
        
        assert!(is_valid);
    }

    #[test]
    fn test_verify_invalid_signature() {
        let keypair1 = generate_keypair().unwrap();
        let keypair2 = generate_keypair().unwrap();
        let data = b"test message";
        
        let signature = sign_data(data, keypair1.private_key()).unwrap();
        let is_valid = verify_signature(data, &signature, keypair2.public_key()).unwrap();
        
        assert!(!is_valid);
    }

    #[test]
    fn test_verify_tampered_data() {
        let keypair = generate_keypair().unwrap();
        let original_data = b"original message";
        let tampered_data = b"tampered message";
        
        let signature = sign_data(original_data, keypair.private_key()).unwrap();
        let is_valid = verify_signature(tampered_data, &signature, keypair.public_key()).unwrap();
        
        assert!(!is_valid);
    }

    #[test]
    fn test_signed_message() {
        let keypair = generate_keypair().unwrap();
        let message = "Hello, ArchiveChain!";
        
        let signed_msg = SignedMessage::new(message, keypair.private_key()).unwrap();
        assert!(signed_msg.verify().unwrap());
        
        let recovered = signed_msg.into_message_if_valid().unwrap();
        assert_eq!(recovered, message);
    }

    #[test]
    fn test_signature_batch() {
        let keypair1 = generate_keypair().unwrap();
        let keypair2 = generate_keypair().unwrap();
        
        let data1 = b"message 1";
        let data2 = b"message 2";
        
        let sig1 = sign_data(data1, keypair1.private_key()).unwrap();
        let sig2 = sign_data(data2, keypair2.private_key()).unwrap();
        
        let mut batch = SignatureBatch::new();
        batch.add(data1, sig1, keypair1.public_key().clone());
        batch.add(data2, sig2, keypair2.public_key().clone());
        
        assert_eq!(batch.len(), 2);
        assert!(batch.verify_all().unwrap());
        
        let individual_results = batch.verify_individual().unwrap();
        assert_eq!(individual_results, vec![true, true]);
    }

    #[test]
    fn test_signature_batch_with_invalid() {
        let keypair1 = generate_keypair().unwrap();
        let keypair2 = generate_keypair().unwrap();
        
        let data1 = b"message 1";
        let data2 = b"message 2";
        
        let sig1 = sign_data(data1, keypair1.private_key()).unwrap();
        let sig2 = sign_data(data2, keypair1.private_key()).unwrap(); // Wrong key!
        
        let mut batch = SignatureBatch::new();
        batch.add(data1, sig1, keypair1.public_key().clone());
        batch.add(data2, sig2, keypair2.public_key().clone()); // Wrong public key
        
        assert!(!batch.verify_all().unwrap());
        
        let individual_results = batch.verify_individual().unwrap();
        assert_eq!(individual_results, vec![true, false]);
    }
}

/// Trait pour les types qui peuvent être signés
pub trait Signable {
    /// Signe l'objet avec une clé privée
    fn sign(&self, private_key: &PrivateKey) -> Result<Signature>;
    
    /// Vérifie la signature de l'objet
    fn verify_signature(&self, signature: &Signature, public_key: &PublicKey) -> Result<bool>;
}

/// Implémentation par défaut pour les types qui implémentent Serialize
impl<T: Serialize> Signable for T {
    fn sign(&self, private_key: &PrivateKey) -> Result<Signature> {
        let serialized = bincode::serialize(self)
            .map_err(|e| CryptoError::RandomGeneration(e.to_string()))?;
        sign_data(&serialized, private_key)
    }
    
    fn verify_signature(&self, signature: &Signature, public_key: &PublicKey) -> Result<bool> {
        let serialized = bincode::serialize(self)
            .map_err(|e| CryptoError::RandomGeneration(e.to_string()))?;
        verify_signature(&serialized, signature, public_key)
    }
}