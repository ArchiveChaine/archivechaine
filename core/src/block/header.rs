//! En-tête de bloc pour ArchiveChain

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::crypto::{Hash, HashAlgorithm, compute_hash};
use crate::error::{BlockError, Result};

/// En-tête d'un bloc ArchiveChain
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Hauteur du bloc dans la chaîne
    pub height: u64,
    
    /// Hash du bloc précédent
    pub previous_hash: Hash,
    
    /// Hash de ce bloc (calculé)
    pub block_hash: Hash,
    
    /// Racine de l'arbre de Merkle des transactions et archives
    pub merkle_root: Hash,
    
    /// Timestamp de création du bloc
    pub timestamp: DateTime<Utc>,
    
    /// Difficulté du consensus (pour le mining/validation)
    pub difficulty: u64,
    
    /// Nonce utilisé pour le Proof of Work/Archive
    pub nonce: u64,
    
    /// Version du protocole blockchain
    pub version: u32,
    
    /// Taille du bloc en bytes
    pub size: u32,
    
    /// Nombre de transactions dans le bloc
    pub transaction_count: u32,
    
    /// Nombre d'archives dans le bloc
    pub archive_count: u32,
}

impl BlockHeader {
    /// Crée un nouvel en-tête de bloc
    pub fn new(
        height: u64,
        previous_hash: Hash,
        merkle_root: Hash,
        timestamp: DateTime<Utc>,
        difficulty: u64,
        nonce: u64,
    ) -> Self {
        Self {
            height,
            previous_hash,
            block_hash: Hash::zero(), // Sera calculé après
            merkle_root,
            timestamp,
            difficulty,
            nonce,
            version: 1, // Version actuelle du protocole
            size: 0,    // Sera calculé après
            transaction_count: 0,
            archive_count: 0,
        }
    }

    /// Calcule le hash de l'en-tête
    pub fn calculate_hash(&self, algorithm: HashAlgorithm) -> Hash {
        // Sérialise les champs principaux pour le hachage
        let data = self.serialize_for_hash();
        compute_hash(&data, algorithm)
    }

    /// Sérialise l'en-tête pour le calcul de hash
    fn serialize_for_hash(&self) -> Vec<u8> {
        let mut data = Vec::new();
        
        // Ajoute tous les champs sauf block_hash (qui sera le résultat)
        data.extend_from_slice(&self.height.to_le_bytes());
        data.extend_from_slice(self.previous_hash.as_bytes());
        data.extend_from_slice(self.merkle_root.as_bytes());
        data.extend_from_slice(&self.timestamp.timestamp().to_le_bytes());
        data.extend_from_slice(&self.timestamp.timestamp_subsec_nanos().to_le_bytes());
        data.extend_from_slice(&self.difficulty.to_le_bytes());
        data.extend_from_slice(&self.nonce.to_le_bytes());
        data.extend_from_slice(&self.version.to_le_bytes());
        
        data
    }

    /// Vérifie que l'en-tête est valide
    pub fn is_valid(&self, algorithm: HashAlgorithm) -> Result<bool> {
        // Vérifie que le hash calculé correspond au hash stocké
        let calculated_hash = self.calculate_hash(algorithm);
        if calculated_hash != self.block_hash {
            return Ok(false);
        }

        // Vérifie que le timestamp n'est pas dans le futur
        let now = Utc::now();
        if self.timestamp > now {
            return Ok(false);
        }

        // Vérifie que la hauteur est cohérente (> 0 sauf pour le bloc genesis)
        if self.height == 0 && !self.previous_hash.is_zero() {
            return Ok(false);
        }

        // Vérifie que les compteurs sont cohérents
        if self.transaction_count > 1_000_000 || self.archive_count > 1_000_000 {
            return Ok(false);
        }

        Ok(true)
    }

    /// Vérifie si c'est le bloc genesis
    pub fn is_genesis(&self) -> bool {
        self.height == 0 && self.previous_hash.is_zero()
    }

    /// Met à jour les compteurs de l'en-tête
    pub fn update_counts(&mut self, transaction_count: u32, archive_count: u32) {
        self.transaction_count = transaction_count;
        self.archive_count = archive_count;
    }

    /// Met à jour la taille du bloc
    pub fn update_size(&mut self, size: u32) {
        self.size = size;
    }

    /// Vérifie la difficulté du bloc (pour le consensus)
    pub fn meets_difficulty_target(&self, target: &Hash) -> bool {
        // Compare le hash du bloc avec la cible de difficulté
        // Le hash doit être inférieur à la cible
        self.block_hash.as_bytes() < target.as_bytes()
    }

    /// Calcule la difficulté basée sur le hash
    pub fn calculate_difficulty(&self) -> u64 {
        // Calcule une métrique de difficulté basée sur le nombre de zéros en tête
        let hash_bytes = self.block_hash.as_bytes();
        let mut difficulty = 0u64;
        
        for &byte in hash_bytes {
            if byte == 0 {
                difficulty += 8;
            } else {
                difficulty += byte.leading_zeros() as u64;
                break;
            }
        }
        
        difficulty
    }

    /// Obtient une représentation hexadécimale du hash
    pub fn hash_hex(&self) -> String {
        self.block_hash.to_hex()
    }

    /// Obtient l'âge du bloc en secondes
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.timestamp).num_seconds()
    }
}

/// Builder pour créer des en-têtes de bloc
#[derive(Debug)]
pub struct BlockHeaderBuilder {
    height: u64,
    previous_hash: Hash,
    merkle_root: Option<Hash>,
    timestamp: Option<DateTime<Utc>>,
    difficulty: Option<u64>,
    nonce: Option<u64>,
    version: Option<u32>,
}

impl BlockHeaderBuilder {
    /// Crée un nouveau builder
    pub fn new(height: u64, previous_hash: Hash) -> Self {
        Self {
            height,
            previous_hash,
            merkle_root: None,
            timestamp: None,
            difficulty: None,
            nonce: None,
            version: None,
        }
    }

    /// Définit la racine de Merkle
    pub fn merkle_root(mut self, merkle_root: Hash) -> Self {
        self.merkle_root = Some(merkle_root);
        self
    }

    /// Définit le timestamp
    pub fn timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Définit la difficulté
    pub fn difficulty(mut self, difficulty: u64) -> Self {
        self.difficulty = Some(difficulty);
        self
    }

    /// Définit le nonce
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Définit la version
    pub fn version(mut self, version: u32) -> Self {
        self.version = Some(version);
        self
    }

    /// Construit l'en-tête
    pub fn build(self) -> Result<BlockHeader> {
        let merkle_root = self.merkle_root.unwrap_or_else(Hash::zero);
        let timestamp = self.timestamp.unwrap_or_else(Utc::now);
        let difficulty = self.difficulty.unwrap_or(0);
        let nonce = self.nonce.unwrap_or(0);
        
        let mut header = BlockHeader::new(
            self.height,
            self.previous_hash,
            merkle_root,
            timestamp,
            difficulty,
            nonce,
        );

        if let Some(version) = self.version {
            header.version = version;
        }

        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{Hash, HashAlgorithm};

    fn create_test_header() -> BlockHeader {
        BlockHeader::new(
            1,
            Hash::zero(),
            Hash::zero(),
            Utc::now(),
            1000,
            12345,
        )
    }

    #[test]
    fn test_header_creation() {
        let header = create_test_header();
        assert_eq!(header.height, 1);
        assert_eq!(header.difficulty, 1000);
        assert_eq!(header.nonce, 12345);
        assert_eq!(header.version, 1);
    }

    #[test]
    fn test_header_hash_calculation() {
        let mut header = create_test_header();
        let hash = header.calculate_hash(HashAlgorithm::Blake3);
        header.block_hash = hash.clone();
        
        assert_eq!(header.block_hash, hash);
        assert!(!hash.is_zero());
    }

    #[test]
    fn test_header_validation() {
        let mut header = create_test_header();
        let hash = header.calculate_hash(HashAlgorithm::Blake3);
        header.block_hash = hash;
        
        assert!(header.is_valid(HashAlgorithm::Blake3).unwrap());
    }

    #[test]
    fn test_genesis_block() {
        let genesis = BlockHeader::new(
            0,
            Hash::zero(),
            Hash::zero(),
            Utc::now(),
            0,
            0,
        );
        
        assert!(genesis.is_genesis());
    }

    #[test]
    fn test_header_builder() {
        let header = BlockHeaderBuilder::new(5, Hash::zero())
            .difficulty(2000)
            .nonce(54321)
            .version(2)
            .build()
            .unwrap();
        
        assert_eq!(header.height, 5);
        assert_eq!(header.difficulty, 2000);
        assert_eq!(header.nonce, 54321);
        assert_eq!(header.version, 2);
    }

    #[test]
    fn test_difficulty_calculation() {
        // Crée un hash avec des zéros en tête pour tester
        let mut hash_bytes = [0u8; 32];
        hash_bytes[0] = 0x0F; // 4 zéros en tête
        
        let mut header = create_test_header();
        header.block_hash = Hash::new(hash_bytes);
        
        let difficulty = header.calculate_difficulty();
        assert_eq!(difficulty, 4);
    }

    #[test]
    fn test_age_calculation() {
        let past_time = Utc::now() - chrono::Duration::seconds(60);
        let header = BlockHeader::new(
            1,
            Hash::zero(),
            Hash::zero(),
            past_time,
            1000,
            0,
        );
        
        let age = header.age_seconds();
        assert!(age >= 60);
    }

    #[test]
    fn test_update_functions() {
        let mut header = create_test_header();
        
        header.update_counts(10, 5);
        assert_eq!(header.transaction_count, 10);
        assert_eq!(header.archive_count, 5);
        
        header.update_size(1024);
        assert_eq!(header.size, 1024);
    }
}