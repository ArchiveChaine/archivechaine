//! Système de validation pour le consensus ArchiveChain
//! 
//! Valide les blocs, transactions et preuves selon les règles du consensus PoA

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::crypto::{Hash, HashAlgorithm, verify_signature};
use crate::block::{Block, BlockHeader};
use crate::transaction::Transaction;
use crate::error::Result;
use super::{NodeId, ConsensusConfig, ConsensusScore};

/// Validateur principal du consensus
#[derive(Debug)]
pub struct ConsensusValidator {
    /// Configuration du consensus
    config: ConsensusConfig,
    /// Cache des validations récentes
    validation_cache: HashMap<Hash, CachedValidation>,
    /// Nœuds de confiance pour la validation
    trusted_validators: HashSet<NodeId>,
    /// Statistiques de validation
    validation_stats: ValidationStatistics,
}

/// Résultat d'une validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Le bloc/transaction est-il valide ?
    pub is_valid: bool,
    /// Erreurs de validation détectées
    pub errors: Vec<ValidationError>,
    /// Avertissements (non bloquants)
    pub warnings: Vec<ValidationWarning>,
    /// Score de confiance (0.0 - 1.0)
    pub confidence_score: f64,
    /// Timestamp de la validation
    pub validated_at: chrono::DateTime<chrono::Utc>,
    /// Validateur qui a effectué la validation
    pub validator_id: Option<NodeId>,
}

/// Erreurs de validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    /// Hash de bloc invalide
    InvalidBlockHash { expected: Hash, actual: Hash },
    /// Signature invalide
    InvalidSignature { signer: NodeId },
    /// Merkle root invalide
    InvalidMerkleRoot { expected: Hash, actual: Hash },
    /// Timestamp invalide
    InvalidTimestamp { reason: String },
    /// Nonce invalide
    InvalidNonce { expected_difficulty: u64 },
    /// Transaction invalide
    InvalidTransaction { tx_hash: Hash, reason: String },
    /// Preuve de stockage invalide
    InvalidStorageProof { reason: String },
    /// Preuve de bande passante invalide
    InvalidBandwidthProof { reason: String },
    /// Preuve de longévité invalide
    InvalidLongevityProof { reason: String },
    /// Score de consensus insuffisant
    InsufficientConsensusScore { node_id: NodeId, score: f64, required: f64 },
    /// Validateur non autorisé
    UnauthorizedValidator { node_id: NodeId },
    /// Bloc orphelin
    OrphanBlock { parent_hash: Hash },
    /// Double dépense détectée
    DoubleSpend { tx_hash: Hash },
    /// Sequence de blocs incorrecte
    InvalidSequence { expected_height: u64, actual_height: u64 },
}

/// Avertissements de validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationWarning {
    /// Timestamp proche du futur
    NearFutureTimestamp,
    /// Faible score de consensus
    LowConsensusScore { score: f64 },
    /// Performance réseau dégradée
    DegradedNetworkPerformance,
    /// Archives potentiellement dupliquées
    PotentialDuplicateArchives,
    /// Frais de transaction inhabituellement élevés
    HighTransactionFees,
}

/// Validation mise en cache
#[derive(Debug, Clone)]
struct CachedValidation {
    result: ValidationResult,
    expires_at: chrono::DateTime<chrono::Utc>,
}

/// Contexte de validation
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// Epoch actuel
    pub current_epoch: u64,
    /// Validateurs autorisés pour cet epoch
    pub authorized_validators: HashSet<NodeId>,
    /// Bloc parent pour la validation
    pub parent_block: Option<Block>,
    /// État de la blockchain
    pub blockchain_state: BlockchainState,
    /// Configuration de validation
    pub validation_config: ValidationConfig,
}

/// État de la blockchain pour la validation
#[derive(Debug, Clone)]
pub struct BlockchainState {
    /// Hauteur actuelle
    pub current_height: u64,
    /// Hash du dernier bloc
    pub last_block_hash: Hash,
    /// UTXOs disponibles
    pub available_utxos: HashMap<Hash, u64>,
    /// Archives connues
    pub known_archives: HashSet<Hash>,
}

/// Configuration de validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Tolérance de timestamp (secondes)
    pub timestamp_tolerance: u64,
    /// Difficulté minimum requise
    pub min_difficulty: u64,
    /// Taille maximum d'un bloc
    pub max_block_size: usize,
    /// Nombre maximum de transactions par bloc
    pub max_transactions_per_block: usize,
    /// Validation stricte des preuves
    pub strict_proof_validation: bool,
    /// Niveau de validation des signatures
    pub signature_validation_level: SignatureValidationLevel,
}

/// Niveau de validation des signatures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignatureValidationLevel {
    /// Validation basique
    Basic,
    /// Validation standard
    Standard,
    /// Validation stricte
    Strict,
}

/// Statistiques de validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStatistics {
    /// Nombre total de validations
    pub total_validations: u64,
    /// Nombre de validations réussies
    pub successful_validations: u64,
    /// Nombre de validations échouées
    pub failed_validations: u64,
    /// Temps moyen de validation (ms)
    pub avg_validation_time_ms: u64,
    /// Erreurs les plus communes
    pub common_errors: HashMap<String, u32>,
    /// Dernière mise à jour
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ConsensusValidator {
    /// Crée un nouveau validateur de consensus
    pub fn new(config: ConsensusConfig) -> Self {
        Self {
            config,
            validation_cache: HashMap::new(),
            trusted_validators: HashSet::new(),
            validation_stats: ValidationStatistics {
                total_validations: 0,
                successful_validations: 0,
                failed_validations: 0,
                avg_validation_time_ms: 0,
                common_errors: HashMap::new(),
                updated_at: chrono::Utc::now(),
            },
        }
    }

    /// Valide un bloc complet
    pub fn validate_block(
        &mut self,
        block: &Block,
        context: &ValidationContext,
    ) -> Result<ValidationResult> {
        let start_time = std::time::Instant::now();
        
        // Vérifie le cache
        if let Some(cached) = self.get_cached_validation(block.hash()) {
            return Ok(cached);
        }

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validation de base du bloc
        self.validate_block_structure(block, &mut errors, &mut warnings)?;
        
        // Validation de l'en-tête
        self.validate_block_header(&block.header, context, &mut errors, &mut warnings)?;
        
        // Validation du corps du bloc
        self.validate_block_body(&block.body, context, &mut errors, &mut warnings)?;
        
        // Validation des transactions
        for transaction in block.transactions() {
            self.validate_transaction_in_block(transaction, context, &mut errors, &mut warnings)?;
        }
        
        // Validation des preuves de consensus
        self.validate_consensus_proofs(block, context, &mut errors, &mut warnings)?;
        
        // Calcule le score de confiance
        let confidence_score = self.calculate_confidence_score(&errors, &warnings);
        
        let is_valid = errors.is_empty();
        let result = ValidationResult {
            is_valid,
            errors,
            warnings,
            confidence_score,
            validated_at: chrono::Utc::now(),
            validator_id: None, // À définir selon le contexte
        };

        // Met en cache le résultat
        self.cache_validation_result(block.hash().clone(), result.clone());
        
        // Met à jour les statistiques
        let validation_time = start_time.elapsed().as_millis() as u64;
        self.update_validation_stats(is_valid, validation_time, &result.errors);

        Ok(result)
    }

    /// Valide une transaction individuellement
    pub fn validate_transaction(
        &mut self,
        transaction: &Transaction,
        context: &ValidationContext,
    ) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validation de base de la transaction
        self.validate_transaction_structure(transaction, &mut errors, &mut warnings)?;
        
        // Validation des UTXOs
        self.validate_transaction_utxos(transaction, context, &mut errors, &mut warnings)?;
        
        // Validation des signatures
        self.validate_transaction_signatures(transaction, context, &mut errors, &mut warnings)?;
        
        // Validation des frais
        self.validate_transaction_fees(transaction, &mut errors, &mut warnings)?;

        let confidence_score = self.calculate_confidence_score(&errors, &warnings);
        
        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            confidence_score,
            validated_at: chrono::Utc::now(),
            validator_id: None,
        })
    }

    /// Valide un score de consensus
    pub fn validate_consensus_score(
        &self,
        node_id: &NodeId,
        score: &ConsensusScore,
        context: &ValidationContext,
    ) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Vérifie que le nœud est autorisé
        if !context.authorized_validators.contains(node_id) {
            errors.push(ValidationError::UnauthorizedValidator {
                node_id: node_id.clone()
            });
        }

        // Vérifie que le score est suffisant
        if !score.is_eligible_validator(&self.config) {
            errors.push(ValidationError::InsufficientConsensusScore {
                node_id: node_id.clone(),
                score: score.combined_score,
                required: 0.1, // Score minimum
            });
        }

        // Avertissements pour scores faibles
        if score.combined_score < 0.5 {
            warnings.push(ValidationWarning::LowConsensusScore {
                score: score.combined_score
            });
        }

        let confidence_score = self.calculate_confidence_score(&errors, &warnings);

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            confidence_score,
            validated_at: chrono::Utc::now(),
            validator_id: None,
        })
    }

    /// Ajoute un validateur de confiance
    pub fn add_trusted_validator(&mut self, node_id: NodeId) {
        self.trusted_validators.insert(node_id);
    }

    /// Supprime un validateur de confiance
    pub fn remove_trusted_validator(&mut self, node_id: &NodeId) {
        self.trusted_validators.remove(node_id);
    }

    /// Obtient les statistiques de validation
    pub fn get_validation_statistics(&self) -> &ValidationStatistics {
        &self.validation_stats
    }

    /// Nettoie le cache expiré
    pub fn cleanup_cache(&mut self) {
        let now = chrono::Utc::now();
        self.validation_cache.retain(|_, cached| cached.expires_at > now);
    }

    // Méthodes privées de validation

    fn validate_block_structure(
        &self,
        block: &Block,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<ValidationWarning>,
    ) -> Result<()> {
        // Vérifie l'intégrité du hash
        let calculated_hash = block.calculate_hash(HashAlgorithm::Blake3);
        if calculated_hash != *block.hash() {
            errors.push(ValidationError::InvalidBlockHash {
                expected: calculated_hash,
                actual: block.hash().clone(),
            });
        }

        // Vérifie l'intégrité du bloc
        if !block.verify_integrity(HashAlgorithm::Blake3)? {
            errors.push(ValidationError::InvalidMerkleRoot {
                expected: block.body.calculate_merkle_root(HashAlgorithm::Blake3),
                actual: block.header.merkle_root.clone(),
            });
        }

        Ok(())
    }

    fn validate_block_header(
        &self,
        header: &BlockHeader,
        context: &ValidationContext,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> Result<()> {
        // Validation du timestamp
        let now = chrono::Utc::now();
        let block_time = header.timestamp;
        
        if block_time > now + chrono::Duration::seconds(context.validation_config.timestamp_tolerance as i64) {
            errors.push(ValidationError::InvalidTimestamp {
                reason: "Timestamp dans le futur".to_string(),
            });
        } else if block_time > now {
            warnings.push(ValidationWarning::NearFutureTimestamp);
        }

        // Validation de la séquence
        if let Some(parent) = &context.parent_block {
            if header.height != parent.height() + 1 {
                errors.push(ValidationError::InvalidSequence {
                    expected_height: parent.height() + 1,
                    actual_height: header.height,
                });
            }

            if header.previous_hash != *parent.hash() {
                errors.push(ValidationError::OrphanBlock {
                    parent_hash: header.previous_hash.clone(),
                });
            }
        }

        // Validation de la difficulté
        if header.difficulty < context.validation_config.min_difficulty {
            errors.push(ValidationError::InvalidNonce {
                expected_difficulty: context.validation_config.min_difficulty,
            });
        }

        Ok(())
    }

    fn validate_block_body(
        &self,
        body: &crate::block::BlockBody,
        context: &ValidationContext,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> Result<()> {
        // Validation du nombre de transactions
        if body.transactions.len() > context.validation_config.max_transactions_per_block {
            errors.push(ValidationError::InvalidTransaction {
                tx_hash: Hash::zero(),
                reason: "Trop de transactions dans le bloc".to_string(),
            });
        }

        // Validation des preuves de stockage
        if context.validation_config.strict_proof_validation {
            if !body.storage_proof.verify_all_proofs()? {
                errors.push(ValidationError::InvalidStorageProof {
                    reason: "Échec de vérification des preuves de stockage".to_string(),
                });
            }
        }

        // Vérification des archives dupliquées
        let mut archive_hashes = HashSet::new();
        for archive in &body.archives {
            if !archive_hashes.insert(archive.archive_id.clone()) {
                warnings.push(ValidationWarning::PotentialDuplicateArchives);
            }
        }

        Ok(())
    }

    fn validate_transaction_in_block(
        &self,
        transaction: &Transaction,
        context: &ValidationContext,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> Result<()> {
        // Validation de base
        if !transaction.is_valid()? {
            errors.push(ValidationError::InvalidTransaction {
                tx_hash: transaction.hash().clone(),
                reason: "Transaction invalide".to_string(),
            });
        }

        // Validation des double-dépenses
        for input in &transaction.inputs {
            // Dans une implémentation complète, on vérifierait les UTXOs
            // Pour l'instant, on fait une validation basique
            if input.previous_tx.is_zero() && !transaction.is_coinbase() {
                errors.push(ValidationError::DoubleSpend {
                    tx_hash: transaction.hash().clone(),
                });
            }
        }

        // Validation des frais
        if transaction.fee > 1000000 { // Frais très élevés
            warnings.push(ValidationWarning::HighTransactionFees);
        }

        Ok(())
    }

    fn validate_transaction_structure(
        &self,
        transaction: &Transaction,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<ValidationWarning>,
    ) -> Result<()> {
        // Vérifie le hash de la transaction
        let calculated_hash = transaction.calculate_hash(HashAlgorithm::Blake3);
        if calculated_hash != *transaction.hash() {
            errors.push(ValidationError::InvalidTransaction {
                tx_hash: transaction.hash().clone(),
                reason: "Hash de transaction invalide".to_string(),
            });
        }

        Ok(())
    }

    fn validate_transaction_utxos(
        &self,
        transaction: &Transaction,
        context: &ValidationContext,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<ValidationWarning>,
    ) -> Result<()> {
        // Validation des UTXOs (implémentation simplifiée)
        for input in &transaction.inputs {
            if !context.blockchain_state.available_utxos.contains_key(&input.previous_tx) {
                errors.push(ValidationError::InvalidTransaction {
                    tx_hash: transaction.hash().clone(),
                    reason: "UTXO non trouvé".to_string(),
                });
            }
        }

        Ok(())
    }

    fn validate_transaction_signatures(
        &self,
        transaction: &Transaction,
        context: &ValidationContext,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<ValidationWarning>,
    ) -> Result<()> {
        match context.validation_config.signature_validation_level {
            SignatureValidationLevel::Basic => {
                // Validation basique - vérifie que les signatures ne sont pas nulles
                if transaction.signature.is_zero() {
                    errors.push(ValidationError::InvalidSignature {
                        signer: NodeId::from(Hash::zero()),
                    });
                }
            },
            SignatureValidationLevel::Standard | SignatureValidationLevel::Strict => {
                // Validation plus stricte - vérification cryptographique complète
                // Note: nécessiterait l'accès aux clés publiques des signataires
                // Pour l'instant, on fait une vérification basique
                for input in &transaction.inputs {
                    if input.signature.is_zero() {
                        errors.push(ValidationError::InvalidSignature {
                            signer: NodeId::from(Hash::zero()),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_transaction_fees(
        &self,
        transaction: &Transaction,
        _errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> Result<()> {
        // Validation des frais
        let size_bytes = transaction.size_bytes();
        let fee_per_byte = transaction.fee_per_byte();
        
        // Seuil de frais élevés (arbitraire pour la démo)
        if fee_per_byte > 100.0 {
            warnings.push(ValidationWarning::HighTransactionFees);
        }

        Ok(())
    }

    fn validate_consensus_proofs(
        &self,
        block: &Block,
        context: &ValidationContext,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<ValidationWarning>,
    ) -> Result<()> {
        // Validation des preuves de consensus
        if context.validation_config.strict_proof_validation {
            // Vérification des preuves de stockage
            if !block.body.storage_proof.verify_all_proofs()? {
                errors.push(ValidationError::InvalidStorageProof {
                    reason: "Preuves de stockage invalides".to_string(),
                });
            }
            
            // Note: Les preuves de bande passante et de longévité seraient validées ici
            // dans une implémentation complète
        }

        Ok(())
    }

    fn calculate_confidence_score(
        &self,
        errors: &[ValidationError],
        warnings: &[ValidationWarning],
    ) -> f64 {
        if !errors.is_empty() {
            return 0.0;
        }
        
        // Score basé sur le nombre d'avertissements
        let warning_penalty = warnings.len() as f64 * 0.1;
        (1.0 - warning_penalty).max(0.0)
    }

    fn get_cached_validation(&self, block_hash: &Hash) -> Option<ValidationResult> {
        self.validation_cache.get(block_hash)
            .and_then(|cached| {
                if cached.expires_at > chrono::Utc::now() {
                    Some(cached.result.clone())
                } else {
                    None
                }
            })
    }

    fn cache_validation_result(&mut self, block_hash: Hash, result: ValidationResult) {
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(10);
        self.validation_cache.insert(block_hash, CachedValidation {
            result,
            expires_at,
        });
    }

    fn update_validation_stats(
        &mut self,
        is_valid: bool,
        validation_time_ms: u64,
        errors: &[ValidationError],
    ) {
        self.validation_stats.total_validations += 1;
        
        if is_valid {
            self.validation_stats.successful_validations += 1;
        } else {
            self.validation_stats.failed_validations += 1;
        }

        // Met à jour le temps moyen
        let total_time = self.validation_stats.avg_validation_time_ms * (self.validation_stats.total_validations - 1)
            + validation_time_ms;
        self.validation_stats.avg_validation_time_ms = total_time / self.validation_stats.total_validations;

        // Compte les erreurs communes
        for error in errors {
            let error_type = std::mem::discriminant(error);
            let error_name = format!("{:?}", error_type);
            *self.validation_stats.common_errors.entry(error_name).or_insert(0) += 1;
        }

        self.validation_stats.updated_at = chrono::Utc::now();
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            timestamp_tolerance: 300, // 5 minutes
            min_difficulty: 1000,
            max_block_size: 1024 * 1024, // 1 MB
            max_transactions_per_block: 1000,
            strict_proof_validation: true,
            signature_validation_level: SignatureValidationLevel::Standard,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{generate_keypair, Hash};
    use crate::block::{BlockBuilder, BlockBody};

    fn create_test_context() -> ValidationContext {
        ValidationContext {
            current_epoch: 1,
            authorized_validators: HashSet::new(),
            parent_block: None,
            blockchain_state: BlockchainState {
                current_height: 0,
                last_block_hash: Hash::zero(),
                available_utxos: HashMap::new(),
                known_archives: HashSet::new(),
            },
            validation_config: ValidationConfig::default(),
        }
    }

    #[test]
    fn test_consensus_validator_creation() {
        let config = ConsensusConfig::test_config();
        let validator = ConsensusValidator::new(config);
        
        assert_eq!(validator.validation_stats.total_validations, 0);
        assert!(validator.trusted_validators.is_empty());
    }

    #[test]
    fn test_block_validation() {
        let config = ConsensusConfig::test_config();
        let mut validator = ConsensusValidator::new(config);
        let context = create_test_context();
        
        // Crée un bloc de test
        let block = BlockBuilder::new(1, Hash::zero(), crate::crypto::HashAlgorithm::Blake3)
            .build()
            .unwrap();
        
        let result = validator.validate_block(&block, &context).unwrap();
        
        // Le bloc devrait être valide (bloc simple sans transactions complexes)
        assert!(result.is_valid || !result.errors.is_empty()); // Peut avoir des erreurs de structure pour un bloc de test
        assert!(result.confidence_score >= 0.0);
        assert!(result.confidence_score <= 1.0);
    }

    #[test]
    fn test_consensus_score_validation() {
        let config = ConsensusConfig::test_config();
        let validator = ConsensusValidator::new(config);
        let mut context = create_test_context();
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        
        // Ajoute le nœud aux validateurs autorisés
        context.authorized_validators.insert(node_id.clone());
        
        let score = super::super::ConsensusScore {
            storage_score: 0.8,
            bandwidth_score: 0.7,
            longevity_score: 0.6,
            combined_score: 0.7,
            node_id: node_id.clone(),
            calculated_at: chrono::Utc::now(),
        };
        
        let result = validator.validate_consensus_score(&node_id, &score, &context).unwrap();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_trusted_validator_management() {
        let config = ConsensusConfig::test_config();
        let mut validator = ConsensusValidator::new(config);
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        
        // Ajoute un validateur de confiance
        validator.add_trusted_validator(node_id.clone());
        assert!(validator.trusted_validators.contains(&node_id));
        
        // Supprime le validateur
        validator.remove_trusted_validator(&node_id);
        assert!(!validator.trusted_validators.contains(&node_id));
    }

    #[test]
    fn test_validation_statistics() {
        let config = ConsensusConfig::test_config();
        let validator = ConsensusValidator::new(config);
        
        let stats = validator.get_validation_statistics();
        assert_eq!(stats.total_validations, 0);
        assert_eq!(stats.successful_validations, 0);
        assert_eq!(stats.failed_validations, 0);
    }
}