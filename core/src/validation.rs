//! Module de validation pour ArchiveChain
//! 
//! Fournit des validateurs pour blocs, transactions et archives

use crate::crypto::{Hash, HashAlgorithm, verify_signature};
use crate::block::{Block, ArchiveBlock};
use crate::transaction::Transaction;
use crate::error::{CoreError, Result};
use chrono::Utc;

/// Validateur principal pour la blockchain
#[derive(Debug)]
pub struct BlockchainValidator {
    /// Configuration de validation
    pub config: ValidationConfig,
}

/// Configuration des règles de validation
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Algorithme de hachage à utiliser
    pub hash_algorithm: HashAlgorithm,
    /// Taille maximum d'un bloc (bytes)
    pub max_block_size: usize,
    /// Nombre maximum de transactions par bloc
    pub max_transactions_per_block: usize,
    /// Nombre maximum d'archives par bloc
    pub max_archives_per_block: usize,
    /// Frais minimum par transaction
    pub min_transaction_fee: u64,
    /// Taille maximum d'une archive (bytes)
    pub max_archive_size: u64,
    /// Tolerance de timestamp (secondes dans le futur)
    pub timestamp_tolerance: i64,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            hash_algorithm: HashAlgorithm::Blake3,
            max_block_size: 1024 * 1024 * 4, // 4MB
            max_transactions_per_block: 1000,
            max_archives_per_block: 100,
            min_transaction_fee: 1,
            max_archive_size: 1024 * 1024 * 100, // 100MB
            timestamp_tolerance: 300, // 5 minutes
        }
    }
}

impl BlockchainValidator {
    /// Crée un nouveau validateur
    pub fn new(config: ValidationConfig) -> Self {
        Self { config }
    }

    /// Valide un bloc complet
    pub fn validate_block(&self, block: &Block, previous_block: Option<&Block>) -> Result<ValidationResult> {
        let mut errors = Vec::new();

        // Validation de base du bloc
        match self.validate_block_structure(block) {
            Ok(false) => errors.push("Structure de bloc invalide".to_string()),
            Err(e) => errors.push(format!("Erreur de validation de structure: {}", e)),
            _ => {}
        }

        // Validation de l'intégrité
        match block.verify_integrity(self.config.hash_algorithm) {
            Ok(false) => errors.push("Intégrité du bloc échouée".to_string()),
            Err(e) => errors.push(format!("Erreur de vérification d'intégrité: {}", e)),
            _ => {}
        }

        // Validation du chaînage
        if let Some(prev) = previous_block {
            if block.previous_hash() != prev.hash() {
                errors.push("Hash du bloc précédent incorrect".to_string());
            }
            if block.height() != prev.height() + 1 {
                errors.push("Hauteur de bloc incorrecte".to_string());
            }
        } else if block.height() != 0 {
            errors.push("Bloc non-genesis sans bloc précédent".to_string());
        }

        // Validation des transactions
        for (i, transaction) in block.transactions().iter().enumerate() {
            match self.validate_transaction(transaction) {
                Ok(ValidationResult { is_valid: false, errors: tx_errors }) => {
                    for error in tx_errors {
                        errors.push(format!("Transaction {}: {}", i, error));
                    }
                }
                Err(e) => errors.push(format!("Erreur de validation transaction {}: {}", i, e)),
                _ => {}
            }
        }

        // Validation des archives
        for (i, archive) in block.archives().iter().enumerate() {
            match self.validate_archive(archive) {
                Ok(ValidationResult { is_valid: false, errors: arch_errors }) => {
                    for error in arch_errors {
                        errors.push(format!("Archive {}: {}", i, error));
                    }
                }
                Err(e) => errors.push(format!("Erreur de validation archive {}: {}", i, e)),
                _ => {}
            }
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
        })
    }

    /// Valide la structure de base d'un bloc
    fn validate_block_structure(&self, block: &Block) -> Result<bool> {
        // Vérifie la taille du bloc
        if block.size_bytes() > self.config.max_block_size {
            return Ok(false);
        }

        // Vérifie le nombre de transactions
        if block.transaction_count() > self.config.max_transactions_per_block {
            return Ok(false);
        }

        // Vérifie le nombre d'archives
        if block.archive_count() > self.config.max_archives_per_block {
            return Ok(false);
        }

        // Vérifie le timestamp
        let now = Utc::now();
        let tolerance = chrono::Duration::seconds(self.config.timestamp_tolerance);
        if block.timestamp() > now + tolerance {
            return Ok(false);
        }

        Ok(true)
    }

    /// Valide une transaction
    pub fn validate_transaction(&self, transaction: &Transaction) -> Result<ValidationResult> {
        let mut errors = Vec::new();

        // Validation de base
        match transaction.is_valid() {
            Ok(false) => errors.push("Transaction de base invalide".to_string()),
            Err(e) => errors.push(format!("Erreur de validation de base: {}", e)),
            _ => {}
        }

        // Vérifie les frais minimum
        if transaction.fee < self.config.min_transaction_fee {
            errors.push(format!(
                "Frais insuffisants: {} < {}",
                transaction.fee, self.config.min_transaction_fee
            ));
        }

        // Vérifie le timestamp
        let now = Utc::now();
        let tolerance = chrono::Duration::seconds(self.config.timestamp_tolerance);
        if transaction.timestamp > now + tolerance {
            errors.push("Timestamp dans le futur".to_string());
        }

        // Vérifie que les montants sont cohérents
        if transaction.total_output_amount() == 0 && !transaction.is_coinbase() {
            errors.push("Montant de sortie zéro pour transaction non-coinbase".to_string());
        }

        // Validation des signatures (basique - nécessiterait l'état pour validation complète)
        if transaction.signature.is_zero() && !transaction.is_coinbase() {
            errors.push("Signature manquante".to_string());
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
        })
    }

    /// Valide une archive
    pub fn validate_archive(&self, archive: &ArchiveBlock) -> Result<ValidationResult> {
        let mut errors = Vec::new();

        // Validation de base
        match archive.is_valid() {
            Ok(false) => errors.push("Archive de base invalide".to_string()),
            Err(e) => errors.push(format!("Erreur de validation d'archive: {}", e)),
            _ => {}
        }

        // Vérifie la taille de l'archive
        if archive.size_original > self.config.max_archive_size {
            errors.push(format!(
                "Archive trop grande: {} > {}",
                archive.size_original, self.config.max_archive_size
            ));
        }

        // Vérifie l'URL
        if archive.original_url.is_empty() {
            errors.push("URL vide".to_string());
        } else if !self.is_valid_url(&archive.original_url) {
            errors.push("Format d'URL invalide".to_string());
        }

        // Vérifie la cohérence de compression
        if archive.size_compressed > archive.size_original {
            errors.push("Taille compressée supérieure à l'original".to_string());
        }

        // Vérifie le timestamp
        let now = Utc::now();
        let tolerance = chrono::Duration::seconds(self.config.timestamp_tolerance);
        if archive.capture_timestamp > now + tolerance {
            errors.push("Timestamp de capture dans le futur".to_string());
        }

        // Vérifie l'intégrité de l'archive
        if !archive.verify_integrity() {
            errors.push("Vérification d'intégrité échouée".to_string());
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
        })
    }

    /// Valide le format d'une URL (basique)
    fn is_valid_url(&self, url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    /// Valide une chaîne de blocs complète
    pub fn validate_chain(&self, blocks: &[Block]) -> Result<ValidationResult> {
        let mut errors = Vec::new();

        if blocks.is_empty() {
            return Ok(ValidationResult {
                is_valid: true,
                errors: Vec::new(),
            });
        }

        // Valide le bloc genesis
        let genesis = &blocks[0];
        if genesis.height() != 0 {
            errors.push("Premier bloc n'est pas genesis".to_string());
        }
        if !genesis.previous_hash().is_zero() {
            errors.push("Bloc genesis a un hash précédent non-zéro".to_string());
        }

        // Valide chaque bloc et son chaînage
        for (i, block) in blocks.iter().enumerate() {
            let previous_block = if i > 0 { Some(&blocks[i - 1]) } else { None };
            
            match self.validate_block(block, previous_block) {
                Ok(ValidationResult { is_valid: false, errors: block_errors }) => {
                    for error in block_errors {
                        errors.push(format!("Bloc {}: {}", i, error));
                    }
                }
                Err(e) => errors.push(format!("Erreur de validation bloc {}: {}", i, e)),
                _ => {}
            }

            // Vérifie la séquence de hauteurs
            if block.height() != i as u64 {
                errors.push(format!("Hauteur incorrecte bloc {}: {} != {}", i, block.height(), i));
            }
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
        })
    }
}

/// Résultat de validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Indique si la validation a réussi
    pub is_valid: bool,
    /// Liste des erreurs trouvées
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// Crée un résultat de validation réussi
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
        }
    }

    /// Crée un résultat de validation échoué avec une erreur
    pub fn invalid(error: String) -> Self {
        Self {
            is_valid: false,
            errors: vec![error],
        }
    }

    /// Crée un résultat de validation échoué avec plusieurs erreurs
    pub fn invalid_multiple(errors: Vec<String>) -> Self {
        Self {
            is_valid: !errors.is_empty(),
            errors,
        }
    }
}

impl Default for BlockchainValidator {
    fn default() -> Self {
        Self::new(ValidationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{BlockBuilder, ArchiveBlockBuilder, CompressionType};
    use crate::transaction::{TransactionBuilder, TransactionType, TransactionOutput};
    use crate::crypto::{Hash, HashAlgorithm, generate_keypair};
    use crate::block::archive_metadata::{ArchiveMetadata, ContentFlags};
    use std::collections::HashMap;

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

        ArchiveBlockBuilder::new(
            "https://example.com".to_string(),
            "text/html".to_string(),
            CompressionType::None,
            1000,
            1000,
            Hash::zero(),
        )
        .metadata(metadata)
        .build()
    }

    #[test]
    fn test_block_validation() {
        let validator = BlockchainValidator::default();
        
        let block = BlockBuilder::new(0, Hash::zero(), HashAlgorithm::Blake3)
            .build()
            .unwrap();
        
        let result = validator.validate_block(&block, None).unwrap();
        assert!(result.is_valid, "Erreurs: {:?}", result.errors);
    }

    #[test]
    fn test_transaction_validation() {
        let validator = BlockchainValidator::default();
        let keypair = generate_keypair().unwrap();
        
        let output = TransactionOutput {
            amount: 1000,
            recipient: keypair.public_key().clone(),
            lock_script: Vec::new(),
        };
        
        let transaction = TransactionBuilder::new(TransactionType::Archive)
            .add_output(output)
            .fee(10)
            .build();
        
        let result = validator.validate_transaction(&transaction).unwrap();
        assert!(result.is_valid, "Erreurs: {:?}", result.errors);
    }

    #[test]
    fn test_archive_validation() {
        let validator = BlockchainValidator::default();
        let archive = create_test_archive();
        
        let result = validator.validate_archive(&archive).unwrap();
        assert!(result.is_valid, "Erreurs: {:?}", result.errors);
    }

    #[test]
    fn test_invalid_url() {
        let validator = BlockchainValidator::default();
        assert!(!validator.is_valid_url("not-an-url"));
        assert!(validator.is_valid_url("https://example.com"));
        assert!(validator.is_valid_url("http://test.org"));
    }

    #[test]
    fn test_validation_result() {
        let valid = ValidationResult::valid();
        assert!(valid.is_valid);
        assert!(valid.errors.is_empty());
        
        let invalid = ValidationResult::invalid("Test error".to_string());
        assert!(!invalid.is_valid);
        assert_eq!(invalid.errors.len(), 1);
    }
}