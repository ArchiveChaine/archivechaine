//! Module de consensus Proof of Archive (PoA) pour ArchiveChain
//! 
//! Implémente un consensus innovant qui combine trois types de preuves :
//! - Proof of Storage : preuves que les nœuds stockent réellement les données
//! - Proof of Bandwidth : démonstration de la capacité à servir le contenu
//! - Proof of Longevity : bonus pour le stockage à long terme

pub mod proof_of_archive;
pub mod storage_proof;
pub mod bandwidth_proof;
pub mod longevity_proof;
pub mod leader_selection;
pub mod validator;
pub mod rewards;

pub use proof_of_archive::{ProofOfArchive, ConsensusScore, ConsensusConfig};
pub use storage_proof::{StorageProofManager, StorageChallenge, StorageChallengeResponse};
pub use bandwidth_proof::{BandwidthProofManager, BandwidthMetrics, BandwidthScore};
pub use longevity_proof::{LongevityProofManager, LongevityMetrics, LongevityBonus};
pub use leader_selection::{LeaderSelector, ValidatorInfo, LeaderElectionResult};
pub use validator::{ConsensusValidator, ValidationResult, ValidationError};
pub use rewards::{RewardCalculator, RewardDistribution, IncentiveTable};

use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::crypto::{Hash, PublicKey};
use crate::error::Result;

/// Identifiant unique d'un nœud du réseau
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Hash);

impl NodeId {
    /// Crée un NodeId à partir d'une clé publique
    pub fn from_public_key(public_key: &PublicKey) -> Self {
        Self(Hash::from_bytes(public_key.as_bytes()).unwrap_or_else(|_| Hash::zero()))
    }

    /// Retourne le hash sous-jacent
    pub fn hash(&self) -> &Hash {
        &self.0
    }
}

impl From<Hash> for NodeId {
    fn from(hash: Hash) -> Self {
        Self(hash)
    }
}

impl From<PublicKey> for NodeId {
    fn from(public_key: PublicKey) -> Self {
        Self::from_public_key(&public_key)
    }
}

/// Configuration du consensus PoA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Poids du score de stockage (défaut: 0.5)
    pub storage_weight: f64,
    /// Poids du score de bande passante (défaut: 0.3)
    pub bandwidth_weight: f64,
    /// Poids du score de longévité (défaut: 0.2)
    pub longevity_weight: f64,
    /// Minimum requis pour valider (en bytes)
    pub min_storage_proof: u64,
    /// Fréquence des défis de stockage
    pub challenge_frequency: Duration,
    /// Nombre de validateurs par round
    pub validators_per_round: usize,
    /// Temps maximum pour répondre à un défi
    pub challenge_timeout: Duration,
    /// Seuil minimum de bande passante (bytes/sec)
    pub min_bandwidth_threshold: u64,
    /// Durée minimum pour les bonus de longévité
    pub min_longevity_duration: Duration,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            storage_weight: 0.5,
            bandwidth_weight: 0.3,
            longevity_weight: 0.2,
            min_storage_proof: 1024 * 1024, // 1 MB minimum
            challenge_frequency: Duration::from_secs(300), // 5 minutes
            validators_per_round: 21,
            challenge_timeout: Duration::from_secs(30),
            min_bandwidth_threshold: 1024 * 1024, // 1 MB/s minimum
            min_longevity_duration: Duration::from_secs(3600 * 24), // 1 jour
        }
    }
}

impl ConsensusConfig {
    /// Valide la configuration
    pub fn validate(&self) -> Result<()> {
        // Vérifie que les poids totalisent 1.0
        let total_weight = self.storage_weight + self.bandwidth_weight + self.longevity_weight;
        if (total_weight - 1.0).abs() > 0.01 {
            return Err(crate::error::CoreError::Validation {
                message: format!("Les poids du consensus doivent totaliser 1.0, trouvé: {}", total_weight)
            });
        }

        // Vérifie que tous les poids sont positifs
        if self.storage_weight < 0.0 || self.bandwidth_weight < 0.0 || self.longevity_weight < 0.0 {
            return Err(crate::error::CoreError::Validation {
                message: "Tous les poids doivent être positifs".to_string()
            });
        }

        // Vérifie les seuils minimums
        if self.min_storage_proof == 0 {
            return Err(crate::error::CoreError::Validation {
                message: "Le seuil minimum de stockage doit être supérieur à 0".to_string()
            });
        }

        if self.validators_per_round == 0 {
            return Err(crate::error::CoreError::Validation {
                message: "Le nombre de validateurs par round doit être supérieur à 0".to_string()
            });
        }

        Ok(())
    }

    /// Crée une configuration pour les tests
    pub fn test_config() -> Self {
        Self {
            storage_weight: 0.5,
            bandwidth_weight: 0.3,
            longevity_weight: 0.2,
            min_storage_proof: 1024, // 1 KB pour les tests
            challenge_frequency: Duration::from_secs(10),
            validators_per_round: 3,
            challenge_timeout: Duration::from_secs(5),
            min_bandwidth_threshold: 1024,
            min_longevity_duration: Duration::from_secs(60), // 1 minute
        }
    }
}

/// Trait principal pour les preuves de consensus
pub trait ConsensusProof {
    /// Type de métrique associé à cette preuve
    type Metrics;
    
    /// Calcule le score de cette preuve pour un nœud
    fn calculate_score(&self, node_id: &NodeId, metrics: &Self::Metrics) -> Result<f64>;
    
    /// Vérifie la validité de la preuve
    fn verify_proof(&self, node_id: &NodeId, proof_data: &[u8]) -> Result<bool>;
    
    /// Génère un défi pour ce type de preuve
    fn generate_challenge(&self, node_id: &NodeId) -> Result<Vec<u8>>;
}

/// Score de consensus combiné pour un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusScore {
    /// Score de stockage (0.0 - 1.0)
    pub storage_score: f64,
    /// Score de bande passante (0.0 - 1.0)
    pub bandwidth_score: f64,
    /// Score de longévité (0.0 - 1.0)
    pub longevity_score: f64,
    /// Score combiné final (0.0 - 1.0)
    pub combined_score: f64,
    /// Identifiant du nœud
    pub node_id: NodeId,
    /// Timestamp du calcul
    pub calculated_at: chrono::DateTime<chrono::Utc>,
}

impl ConsensusScore {
    /// Crée un nouveau score
    pub fn new(
        node_id: NodeId,
        storage_score: f64,
        bandwidth_score: f64,
        longevity_score: f64,
        config: &ConsensusConfig,
    ) -> Self {
        let combined_score = storage_score * config.storage_weight
            + bandwidth_score * config.bandwidth_weight
            + longevity_score * config.longevity_weight;

        Self {
            storage_score,
            bandwidth_score,
            longevity_score,
            combined_score,
            node_id,
            calculated_at: chrono::Utc::now(),
        }
    }

    /// Vérifie si le score est suffisant pour être validateur
    pub fn is_eligible_validator(&self, config: &ConsensusConfig) -> bool {
        self.combined_score > 0.1 && // Score minimum de 10%
        self.storage_score > 0.0 && // Doit avoir du stockage
        self.bandwidth_score > 0.0  // Doit avoir de la bande passante
    }

    /// Retourne une représentation normalisée du score (0-100)
    pub fn normalized_score(&self) -> u8 {
        (self.combined_score * 100.0).min(100.0) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{generate_keypair, Hash};

    #[test]
    fn test_consensus_config_default() {
        let config = ConsensusConfig::default();
        assert!(config.validate().is_ok());
        
        let total = config.storage_weight + config.bandwidth_weight + config.longevity_weight;
        assert!((total - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_consensus_config_validation() {
        let mut config = ConsensusConfig::default();
        config.storage_weight = 0.6; // Total > 1.0
        assert!(config.validate().is_err());

        config.storage_weight = -0.1; // Poids négatif
        config.bandwidth_weight = 0.6;
        config.longevity_weight = 0.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_node_id_creation() {
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        
        assert!(!node_id.hash().is_zero());
        
        let node_id2 = NodeId::from(keypair.public_key().clone());
        assert_eq!(node_id, node_id2);
    }

    #[test]
    fn test_consensus_score() {
        let config = ConsensusConfig::default();
        let node_id = NodeId::from(Hash::zero());
        
        let score = ConsensusScore::new(
            node_id.clone(),
            0.8, // Storage
            0.6, // Bandwidth
            0.4, // Longevity
            &config,
        );

        // Vérifie le calcul du score combiné
        let expected = 0.8 * 0.5 + 0.6 * 0.3 + 0.4 * 0.2;
        assert!((score.combined_score - expected).abs() < 0.01);
        
        assert!(score.is_eligible_validator(&config));
        assert_eq!(score.normalized_score(), (expected * 100.0) as u8);
    }

    #[test]
    fn test_consensus_score_eligibility() {
        let config = ConsensusConfig::default();
        let node_id = NodeId::from(Hash::zero());
        
        // Score trop faible
        let low_score = ConsensusScore::new(node_id.clone(), 0.1, 0.1, 0.1, &config);
        assert!(!low_score.is_eligible_validator(&config));
        
        // Pas de stockage
        let no_storage = ConsensusScore::new(node_id.clone(), 0.0, 0.8, 0.8, &config);
        assert!(!no_storage.is_eligible_validator(&config));
        
        // Score valide
        let valid_score = ConsensusScore::new(node_id, 0.5, 0.5, 0.5, &config);
        assert!(valid_score.is_eligible_validator(&config));
    }
}