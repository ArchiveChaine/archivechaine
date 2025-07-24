//! Corps de bloc pour ArchiveChain
//! 
//! Contient les transactions, archives, index de contenu et preuves de stockage

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::crypto::{Hash, HashAlgorithm, compute_hash, compute_combined_hash};
use crate::state::{MerkleTree, MerkleProof};
use crate::transaction::Transaction;
use crate::error::{BlockError, Result};
use super::archive_metadata::ArchiveBlock;

/// Index de contenu pour la recherche rapide
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentIndex {
    /// Index par mots-clés
    pub keyword_index: HashMap<String, Vec<Hash>>,
    
    /// Index par type de contenu
    pub content_type_index: HashMap<String, Vec<Hash>>,
    
    /// Index par domaine
    pub domain_index: HashMap<String, Vec<Hash>>,
    
    /// Index par langue
    pub language_index: HashMap<String, Vec<Hash>>,
    
    /// Index temporel (par année-mois)
    pub temporal_index: HashMap<String, Vec<Hash>>,
    
    /// Statistiques d'indexation
    pub stats: IndexStats,
}

/// Statistiques de l'index de contenu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Nombre total d'entrées indexées
    pub total_entries: u32,
    
    /// Nombre de mots-clés uniques
    pub unique_keywords: u32,
    
    /// Nombre de types de contenu
    pub content_types: u32,
    
    /// Nombre de domaines uniques
    pub unique_domains: u32,
    
    /// Nombre de langues détectées
    pub languages: u32,
}

impl ContentIndex {
    /// Crée un nouvel index vide
    pub fn new() -> Self {
        Self {
            keyword_index: HashMap::new(),
            content_type_index: HashMap::new(),
            domain_index: HashMap::new(),
            language_index: HashMap::new(),
            temporal_index: HashMap::new(),
            stats: IndexStats {
                total_entries: 0,
                unique_keywords: 0,
                content_types: 0,
                unique_domains: 0,
                languages: 0,
            },
        }
    }

    /// Ajoute une archive à l'index
    pub fn add_archive(&mut self, archive: &ArchiveBlock) {
        let archive_id = archive.archive_id.clone();

        // Index par mots-clés
        for keyword in &archive.metadata.keywords {
            self.keyword_index
                .entry(keyword.to_lowercase())
                .or_insert_with(Vec::new)
                .push(archive_id.clone());
        }

        // Index par type de contenu
        self.content_type_index
            .entry(archive.content_type.clone())
            .or_insert_with(Vec::new)
            .push(archive_id.clone());

        // Index par domaine (extrait de l'URL)
        if let Some(domain) = Self::extract_domain(&archive.original_url) {
            self.domain_index
                .entry(domain)
                .or_insert_with(Vec::new)
                .push(archive_id.clone());
        }

        // Index par langue
        if let Some(ref language) = archive.metadata.language {
            self.language_index
                .entry(language.clone())
                .or_insert_with(Vec::new)
                .push(archive_id.clone());
        }

        // Index temporel
        let temporal_key = format!(
            "{}-{:02}",
            archive.capture_timestamp.year(),
            archive.capture_timestamp.month()
        );
        self.temporal_index
            .entry(temporal_key)
            .or_insert_with(Vec::new)
            .push(archive_id);

        // Met à jour les statistiques
        self.update_stats();
    }

    /// Extrait le domaine d'une URL
    fn extract_domain(url: &str) -> Option<String> {
        if let Some(start) = url.find("://") {
            let after_protocol = &url[start + 3..];
            if let Some(end) = after_protocol.find('/') {
                Some(after_protocol[..end].to_lowercase())
            } else {
                Some(after_protocol.to_lowercase())
            }
        } else {
            None
        }
    }

    /// Met à jour les statistiques de l'index
    fn update_stats(&mut self) {
        self.stats.unique_keywords = self.keyword_index.len() as u32;
        self.stats.content_types = self.content_type_index.len() as u32;
        self.stats.unique_domains = self.domain_index.len() as u32;
        self.stats.languages = self.language_index.len() as u32;
        
        // Compte le nombre total d'entrées (approximation)
        self.stats.total_entries = self.keyword_index.values()
            .map(|v| v.len() as u32)
            .sum();
    }

    /// Recherche par mot-clé
    pub fn search_by_keyword(&self, keyword: &str) -> Vec<&Hash> {
        self.keyword_index
            .get(&keyword.to_lowercase())
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Recherche par type de contenu
    pub fn search_by_content_type(&self, content_type: &str) -> Vec<&Hash> {
        self.content_type_index
            .get(content_type)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Recherche par domaine
    pub fn search_by_domain(&self, domain: &str) -> Vec<&Hash> {
        self.domain_index
            .get(&domain.to_lowercase())
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }
}

impl Default for ContentIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Preuve de stockage cryptographique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageProof {
    /// Hash racine de l'arbre de preuves
    pub proof_root: Hash,
    
    /// Preuves individuelles pour chaque archive
    pub archive_proofs: HashMap<Hash, ArchiveStorageProof>,
    
    /// Timestamp de génération des preuves
    pub generated_at: chrono::DateTime<chrono::Utc>,
    
    /// Algorithme utilisé pour les preuves
    pub algorithm: HashAlgorithm,
    
    /// Métadonnées des preuves
    pub proof_metadata: ProofMetadata,
}

/// Preuve de stockage pour une archive individuelle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveStorageProof {
    /// Hash de l'archive
    pub archive_hash: Hash,
    
    /// Preuve de Merkle
    pub merkle_proof: MerkleProof,
    
    /// Défi de preuve de stockage
    pub challenge: StorageChallenge,
    
    /// Réponse au défi
    pub response: StorageChallengeResponse,
    
    /// Signature de la preuve
    pub proof_signature: Hash,
}

/// Défi pour prouver le stockage effectif
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageChallenge {
    /// Positions des bytes à vérifier
    pub positions: Vec<u64>,
    
    /// Taille des échantillons
    pub sample_size: u32,
    
    /// Nonce du défi
    pub nonce: u64,
    
    /// Timestamp du défi
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Réponse au défi de stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageChallengeResponse {
    /// Données aux positions demandées
    pub samples: Vec<u8>,
    
    /// Hash des échantillons
    pub sample_hash: Hash,
    
    /// Timestamp de la réponse
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Métadonnées des preuves de stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofMetadata {
    /// Nombre total d'archives prouvées
    pub total_archives: u32,
    
    /// Taille totale des données prouvées
    pub total_size: u64,
    
    /// Temps de génération des preuves (ms)
    pub generation_time_ms: u64,
    
    /// Version du protocole de preuve
    pub protocol_version: u32,
}

impl StorageProof {
    /// Crée une nouvelle preuve de stockage
    pub fn new() -> Self {
        Self {
            proof_root: Hash::zero(),
            archive_proofs: HashMap::new(),
            generated_at: chrono::Utc::now(),
            algorithm: HashAlgorithm::Blake3,
            proof_metadata: ProofMetadata {
                total_archives: 0,
                total_size: 0,
                generation_time_ms: 0,
                protocol_version: 1,
            },
        }
    }

    /// Ajoute une preuve d'archive
    pub fn add_archive_proof(&mut self, archive_hash: Hash, proof: ArchiveStorageProof) {
        self.archive_proofs.insert(archive_hash, proof);
        self.update_metadata();
    }

    /// Met à jour les métadonnées
    fn update_metadata(&mut self) {
        self.proof_metadata.total_archives = self.archive_proofs.len() as u32;
    }

    /// Vérifie toutes les preuves
    pub fn verify_all_proofs(&self) -> Result<bool> {
        for (archive_hash, proof) in &self.archive_proofs {
            if !self.verify_archive_proof(archive_hash, proof)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Vérifie une preuve d'archive individuelle
    pub fn verify_archive_proof(&self, archive_hash: &Hash, proof: &ArchiveStorageProof) -> Result<bool> {
        // Vérifie la preuve de Merkle
        if !proof.merkle_proof.verify(self.algorithm) {
            return Ok(false);
        }

        // Vérifie que le hash correspond
        if proof.archive_hash != *archive_hash {
            return Ok(false);
        }

        // Vérifie la réponse au défi
        let expected_hash = compute_hash(&proof.response.samples, self.algorithm);
        if expected_hash != proof.response.sample_hash {
            return Ok(false);
        }

        Ok(true)
    }

    /// Génère un défi de stockage aléatoire
    pub fn generate_challenge(archive_size: u64, sample_count: u32) -> StorageChallenge {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let mut positions = Vec::new();
        for _ in 0..sample_count {
            positions.push(rng.gen_range(0..archive_size));
        }
        positions.sort();

        StorageChallenge {
            positions,
            sample_size: sample_count,
            nonce: rng.gen(),
            timestamp: chrono::Utc::now(),
        }
    }
}

impl Default for StorageProof {
    fn default() -> Self {
        Self::new()
    }
}

/// Corps d'un bloc contenant transactions et archives
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockBody {
    /// Transactions dans ce bloc
    pub transactions: Vec<Transaction>,
    
    /// Archives dans ce bloc
    pub archives: Vec<ArchiveBlock>,
    
    /// Index de contenu pour la recherche
    pub content_index: ContentIndex,
    
    /// Preuves de stockage
    pub storage_proof: StorageProof,
}

impl BlockBody {
    /// Crée un nouveau corps de bloc
    pub fn new(
        transactions: Vec<Transaction>,
        archives: Vec<ArchiveBlock>,
        mut content_index: ContentIndex,
        storage_proof: StorageProof,
    ) -> Self {
        // Indexe toutes les archives
        for archive in &archives {
            content_index.add_archive(archive);
        }

        Self {
            transactions,
            archives,
            content_index,
            storage_proof,
        }
    }

    /// Calcule le hash du corps du bloc
    pub fn calculate_hash(&self, algorithm: HashAlgorithm) -> Hash {
        let mut data = Vec::new();
        
        // Hash des transactions
        for tx in &self.transactions {
            data.extend_from_slice(tx.hash().as_bytes());
        }
        
        // Hash des archives
        for archive in &self.archives {
            data.extend_from_slice(archive.archive_id.as_bytes());
        }
        
        compute_hash(&data, algorithm)
    }

    /// Calcule la racine de Merkle du corps
    pub fn calculate_merkle_root(&self, algorithm: HashAlgorithm) -> Hash {
        let mut hashes = Vec::new();
        
        // Ajoute les hashs des transactions
        for tx in &self.transactions {
            hashes.push(tx.hash().clone());
        }
        
        // Ajoute les hashs des archives
        for archive in &self.archives {
            hashes.push(archive.archive_id.clone());
        }
        
        if hashes.is_empty() {
            return Hash::zero();
        }
        
        let merkle_tree = MerkleTree::from_hashes(hashes, algorithm);
        merkle_tree.root_hash().cloned().unwrap_or_else(Hash::zero)
    }

    /// Vérifie l'intégrité du corps
    pub fn verify_integrity(&self, algorithm: HashAlgorithm) -> Result<bool> {
        // Vérifie que toutes les transactions sont valides
        for tx in &self.transactions {
            if !tx.is_valid()? {
                return Ok(false);
            }
        }

        // Vérifie que toutes les archives sont valides
        for archive in &self.archives {
            if !archive.is_valid()? {
                return Ok(false);
            }
        }

        // Vérifie les preuves de stockage
        if !self.storage_proof.verify_all_proofs()? {
            return Ok(false);
        }

        Ok(true)
    }

    /// Obtient le nombre total d'éléments
    pub fn total_items(&self) -> usize {
        self.transactions.len() + self.archives.len()
    }

    /// Calcule la taille totale des archives
    pub fn total_archive_size(&self) -> u64 {
        self.archives.iter()
            .map(|a| a.size_original)
            .sum()
    }

    /// Calcule la taille compressée totale
    pub fn total_compressed_size(&self) -> u64 {
        self.archives.iter()
            .map(|a| a.size_compressed)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Hash;
    use crate::transaction::Transaction;
    use crate::block::archive_metadata::{ArchiveBlock, ArchiveMetadata, CompressionType, ContentFlags};
    use std::collections::HashMap;

    fn create_test_transaction() -> Transaction {
        // Cette fonction sera implémentée dans le module transaction
        Transaction::new(Hash::zero(), Hash::zero(), 0, Vec::new())
    }

    fn create_test_archive() -> ArchiveBlock {
        let metadata = ArchiveMetadata {
            title: Some("Test".to_string()),
            description: None,
            keywords: vec!["test".to_string()],
            content_type: "text/html".to_string(),
            language: Some("en".to_string()),
            author: None,
            published_at: None,
            custom_metadata: HashMap::new(),
            external_links_count: 0,
            resource_count: 0,
            quality_score: 50,
            content_flags: ContentFlags::default(),
        };

        ArchiveBlock::new(
            "https://example.com".to_string(),
            "text/html".to_string(),
            CompressionType::None,
            1000,
            1000,
            Hash::zero(),
            metadata,
        )
    }

    #[test]
    fn test_content_index() {
        let mut index = ContentIndex::new();
        let archive = create_test_archive();
        
        index.add_archive(&archive);
        
        assert_eq!(index.stats.unique_keywords, 1);
        assert_eq!(index.stats.content_types, 1);
        assert!(!index.search_by_keyword("test").is_empty());
    }

    #[test]
    fn test_domain_extraction() {
        assert_eq!(
            ContentIndex::extract_domain("https://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            ContentIndex::extract_domain("http://sub.domain.org/"),
            Some("sub.domain.org".to_string())
        );
    }

    #[test]
    fn test_storage_proof() {
        let mut proof = StorageProof::new();
        assert_eq!(proof.archive_proofs.len(), 0);
        
        let challenge = StorageProof::generate_challenge(1000, 5);
        assert_eq!(challenge.positions.len(), 5);
        assert!(challenge.positions.iter().all(|&pos| pos < 1000));
    }

    #[test]
    fn test_block_body() {
        let transactions = vec![];
        let archives = vec![create_test_archive()];
        let content_index = ContentIndex::new();
        let storage_proof = StorageProof::new();
        
        let body = BlockBody::new(transactions, archives, content_index, storage_proof);
        
        assert_eq!(body.transactions.len(), 0);
        assert_eq!(body.archives.len(), 1);
        assert_eq!(body.total_items(), 1);
    }

    #[test]
    fn test_merkle_root_calculation() {
        let transactions = vec![];
        let archives = vec![create_test_archive()];
        let content_index = ContentIndex::new();
        let storage_proof = StorageProof::new();
        
        let body = BlockBody::new(transactions, archives, content_index, storage_proof);
        let merkle_root = body.calculate_merkle_root(HashAlgorithm::Blake3);
        
        assert!(!merkle_root.is_zero());
    }
}