//! Module des blocs pour ArchiveChain
//! 
//! Contient les structures de données principales pour les blocs de la blockchain,
//! incluant les métadonnées d'archives et les preuves de stockage

pub mod header;
pub mod body;
pub mod archive_metadata;

pub use header::BlockHeader;
pub use body::{BlockBody, ContentIndex, StorageProof};
pub use archive_metadata::{ArchiveMetadata, CompressionType, ArchiveBlock};

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::crypto::{Hash, HashAlgorithm, compute_combined_hash};
use crate::error::{BlockError, Result};
use crate::transaction::Transaction;

/// Structure principale d'un bloc ArchiveChain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// En-tête du bloc
    pub header: BlockHeader,
    /// Corps du bloc contenant les transactions et archives
    pub body: BlockBody,
}

impl Block {
    /// Crée un nouveau bloc
    pub fn new(
        header: BlockHeader,
        body: BlockBody,
    ) -> Self {
        Self { header, body }
    }

    /// Calcule le hash du bloc complet
    pub fn calculate_hash(&self, algorithm: HashAlgorithm) -> Hash {
        let header_hash = self.header.calculate_hash(algorithm);
        let body_hash = self.body.calculate_hash(algorithm);
        
        compute_combined_hash(
            &[header_hash.as_bytes(), body_hash.as_bytes()],
            algorithm
        )
    }

    /// Vérifie l'intégrité du bloc
    pub fn verify_integrity(&self, algorithm: HashAlgorithm) -> Result<bool> {
        // Vérifie que le hash de l'en-tête correspond
        let calculated_hash = self.header.calculate_hash(algorithm);
        if calculated_hash != self.header.block_hash {
            return Ok(false);
        }

        // Vérifie l'intégrité du corps
        if !self.body.verify_integrity(algorithm)? {
            return Ok(false);
        }

        // Vérifie que le merkle root correspond
        let body_merkle_root = self.body.calculate_merkle_root(algorithm);
        if body_merkle_root != self.header.merkle_root {
            return Ok(false);
        }

        Ok(true)
    }

    /// Obtient la hauteur du bloc
    pub fn height(&self) -> u64 {
        self.header.height
    }

    /// Obtient le timestamp du bloc
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.header.timestamp
    }

    /// Obtient le hash du bloc précédent
    pub fn previous_hash(&self) -> &Hash {
        &self.header.previous_hash
    }

    /// Obtient le hash du bloc
    pub fn hash(&self) -> &Hash {
        &self.header.block_hash
    }

    /// Obtient le nombre de transactions
    pub fn transaction_count(&self) -> usize {
        self.body.transactions.len()
    }

    /// Obtient le nombre d'archives
    pub fn archive_count(&self) -> usize {
        self.body.archives.len()
    }

    /// Vérifie si le bloc est valide selon les règles de consensus
    pub fn is_valid(&self, algorithm: HashAlgorithm) -> Result<bool> {
        // Vérifications de base
        if !self.verify_integrity(algorithm)? {
            return Ok(false);
        }

        // Vérifie le timestamp (ne doit pas être dans le futur)
        let now = Utc::now();
        if self.header.timestamp > now {
            return Ok(false);
        }

        // Vérifie que toutes les transactions sont valides
        for transaction in &self.body.transactions {
            if !transaction.is_valid()? {
                return Ok(false);
            }
        }

        // Vérifie que toutes les archives ont des métadonnées valides
        for archive in &self.body.archives {
            if !archive.is_valid()? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Calcule la taille du bloc en bytes
    pub fn size_bytes(&self) -> usize {
        bincode::serialized_size(self).unwrap_or(0) as usize
    }

    /// Obtient toutes les transactions du bloc
    pub fn transactions(&self) -> &[Transaction] {
        &self.body.transactions
    }

    /// Obtient toutes les archives du bloc
    pub fn archives(&self) -> &[ArchiveBlock] {
        &self.body.archives
    }
}

/// Builder pour créer des blocs de manière fluide
#[derive(Debug)]
pub struct BlockBuilder {
    height: u64,
    previous_hash: Hash,
    timestamp: Option<DateTime<Utc>>,
    difficulty: u64,
    nonce: u64,
    transactions: Vec<Transaction>,
    archives: Vec<ArchiveBlock>,
    algorithm: HashAlgorithm,
}

impl BlockBuilder {
    /// Crée un nouveau builder
    pub fn new(height: u64, previous_hash: Hash, algorithm: HashAlgorithm) -> Self {
        Self {
            height,
            previous_hash,
            timestamp: None,
            difficulty: 0,
            nonce: 0,
            transactions: Vec::new(),
            archives: Vec::new(),
            algorithm,
        }
    }

    /// Définit le timestamp
    pub fn timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Définit la difficulté
    pub fn difficulty(mut self, difficulty: u64) -> Self {
        self.difficulty = difficulty;
        self
    }

    /// Définit le nonce
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = nonce;
        self
    }

    /// Ajoute une transaction
    pub fn add_transaction(mut self, transaction: Transaction) -> Self {
        self.transactions.push(transaction);
        self
    }

    /// Ajoute plusieurs transactions
    pub fn add_transactions(mut self, transactions: Vec<Transaction>) -> Self {
        self.transactions.extend(transactions);
        self
    }

    /// Ajoute une archive
    pub fn add_archive(mut self, archive: ArchiveBlock) -> Self {
        self.archives.push(archive);
        self
    }

    /// Ajoute plusieurs archives
    pub fn add_archives(mut self, archives: Vec<ArchiveBlock>) -> Self {
        self.archives.extend(archives);
        self
    }

    /// Construit le bloc final
    pub fn build(self) -> Result<Block> {
        let timestamp = self.timestamp.unwrap_or_else(Utc::now);
        
        // Crée le corps du bloc
        let body = BlockBody::new(
            self.transactions,
            self.archives,
            ContentIndex::new(),
            StorageProof::new(),
        );

        // Calcule le merkle root
        let merkle_root = body.calculate_merkle_root(self.algorithm);

        // Crée l'en-tête
        let mut header = BlockHeader::new(
            self.height,
            self.previous_hash,
            merkle_root,
            timestamp,
            self.difficulty,
            self.nonce,
        );

        // Calcule le hash du bloc
        let block_hash = header.calculate_hash(self.algorithm);
        header.block_hash = block_hash;

        Ok(Block::new(header, body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{Hash, HashAlgorithm};

    fn create_test_block() -> Block {
        BlockBuilder::new(1, Hash::zero(), HashAlgorithm::Blake3)
            .difficulty(1000)
            .nonce(12345)
            .build()
            .unwrap()
    }

    #[test]
    fn test_block_creation() {
        let block = create_test_block();
        assert_eq!(block.height(), 1);
        assert_eq!(block.transaction_count(), 0);
        assert_eq!(block.archive_count(), 0);
    }

    #[test]
    fn test_block_integrity() {
        let block = create_test_block();
        assert!(block.verify_integrity(HashAlgorithm::Blake3).unwrap());
    }

    #[test]
    fn test_block_validity() {
        let block = create_test_block();
        assert!(block.is_valid(HashAlgorithm::Blake3).unwrap());
    }

    #[test]
    fn test_block_builder() {
        let block = BlockBuilder::new(5, Hash::zero(), HashAlgorithm::Blake3)
            .difficulty(2000)
            .nonce(54321)
            .build()
            .unwrap();
        
        assert_eq!(block.height(), 5);
        assert_eq!(block.header.difficulty, 2000);
        assert_eq!(block.header.nonce, 54321);
    }

    #[test]
    fn test_block_hash_consistency() {
        let block = create_test_block();
        let hash1 = block.calculate_hash(HashAlgorithm::Blake3);
        let hash2 = block.calculate_hash(HashAlgorithm::Blake3);
        assert_eq!(hash1, hash2);
    }
}