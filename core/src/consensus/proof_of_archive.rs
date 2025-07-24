//! Algorithme principal du consensus Proof of Archive (PoA)
//! 
//! Combine les trois types de preuves pour produire un score de consensus unifié

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::crypto::{Hash, HashAlgorithm, compute_hash};
use crate::error::Result;
use super::{
    NodeId, ConsensusConfig, ConsensusScore, ConsensusProof,
    storage_proof::{StorageProofManager, StorageMetrics},
    bandwidth_proof::{BandwidthProofManager, BandwidthMetrics},
    longevity_proof::{LongevityProofManager, LongevityMetrics},
};

/// Gestionnaire principal du consensus Proof of Archive
#[derive(Debug)]
pub struct ProofOfArchive {
    /// Configuration du consensus
    config: ConsensusConfig,
    /// Gestionnaire des preuves de stockage
    storage_manager: StorageProofManager,
    /// Gestionnaire des preuves de bande passante
    bandwidth_manager: BandwidthProofManager,
    /// Gestionnaire des preuves de longévité
    longevity_manager: LongevityProofManager,
    /// Cache des scores calculés
    score_cache: HashMap<NodeId, CachedScore>,
    /// Epoch actuel du consensus
    current_epoch: u64,
}

/// Score mis en cache avec timestamp
#[derive(Debug, Clone)]
struct CachedScore {
    score: ConsensusScore,
    expires_at: SystemTime,
}

impl ProofOfArchive {
    /// Crée une nouvelle instance du consensus PoA
    pub fn new(config: ConsensusConfig) -> Result<Self> {
        config.validate()?;

        Ok(Self {
            storage_manager: StorageProofManager::new(&config),
            bandwidth_manager: BandwidthProofManager::new(&config),
            longevity_manager: LongevityProofManager::new(&config),
            config,
            score_cache: HashMap::new(),
            current_epoch: 0,
        })
    }

    /// Calcule le score de consensus pour un nœud
    pub fn calculate_consensus_score(&mut self, node_id: &NodeId) -> Result<ConsensusScore> {
        // Vérifie le cache
        if let Some(cached) = self.score_cache.get(node_id) {
            if cached.expires_at > SystemTime::now() {
                return Ok(cached.score.clone());
            }
        }

        // Récupère les métriques pour chaque type de preuve
        let storage_metrics = self.storage_manager.get_node_metrics(node_id)?;
        let bandwidth_metrics = self.bandwidth_manager.get_node_metrics(node_id)?;
        let longevity_metrics = self.longevity_manager.get_node_metrics(node_id)?;

        // Calcule les scores individuels
        let storage_score = self.storage_manager.calculate_score(node_id, &storage_metrics)?;
        let bandwidth_score = self.bandwidth_manager.calculate_score(node_id, &bandwidth_metrics)?;
        let longevity_score = self.longevity_manager.calculate_score(node_id, &longevity_metrics)?;

        // Crée le score combiné
        let consensus_score = ConsensusScore::new(
            node_id.clone(),
            storage_score,
            bandwidth_score,
            longevity_score,
            &self.config,
        );

        // Met en cache le résultat
        self.cache_score(node_id.clone(), consensus_score.clone());

        Ok(consensus_score)
    }

    /// Calcule les scores pour tous les nœuds actifs
    pub fn calculate_all_scores(&mut self, active_nodes: &[NodeId]) -> Result<Vec<ConsensusScore>> {
        let mut scores = Vec::new();
        
        for node_id in active_nodes {
            match self.calculate_consensus_score(node_id) {
                Ok(score) => scores.push(score),
                Err(e) => {
                    log::warn!("Erreur lors du calcul du score pour le nœud {:?}: {}", node_id, e);
                    continue;
                }
            }
        }

        // Trie par score décroissant
        scores.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());

        Ok(scores)
    }

    /// Sélectionne les validateurs pour l'epoch actuel
    pub fn select_validators(&mut self, active_nodes: &[NodeId]) -> Result<Vec<NodeId>> {
        let scores = self.calculate_all_scores(active_nodes)?;
        
        let mut validators = Vec::new();
        for score in scores {
            if score.is_eligible_validator(&self.config) && validators.len() < self.config.validators_per_round {
                validators.push(score.node_id);
            }
        }

        // S'assure qu'on a au moins un validateur
        if validators.is_empty() && !active_nodes.is_empty() {
            validators.push(active_nodes[0].clone());
        }

        Ok(validators)
    }

    /// Génère un défi de consensus pour un nœud
    pub fn generate_consensus_challenge(&self, node_id: &NodeId) -> Result<ConsensusChallenge> {
        let storage_challenge = self.storage_manager.generate_challenge(node_id)?;
        let bandwidth_challenge = self.bandwidth_manager.generate_challenge(node_id)?;
        let longevity_challenge = self.longevity_manager.generate_challenge(node_id)?;

        let nonce = self.generate_nonce();
        let timestamp = chrono::Utc::now();

        let challenge = ConsensusChallenge {
            node_id: node_id.clone(),
            epoch: self.current_epoch,
            storage_challenge,
            bandwidth_challenge,
            longevity_challenge,
            nonce,
            timestamp,
            expires_at: timestamp + chrono::Duration::from_std(self.config.challenge_timeout).unwrap(),
        };

        Ok(challenge)
    }

    /// Vérifie une réponse à un défi de consensus
    pub fn verify_consensus_response(
        &self,
        challenge: &ConsensusChallenge,
        response: &ConsensusResponse,
    ) -> Result<bool> {
        // Vérifie que la réponse correspond au défi
        if response.challenge_id != challenge.id() {
            return Ok(false);
        }

        // Vérifie que la réponse n'est pas expirée
        if chrono::Utc::now() > challenge.expires_at {
            return Ok(false);
        }

        // Vérifie chaque type de preuve
        let storage_valid = self.storage_manager.verify_proof(
            &challenge.node_id,
            &response.storage_response,
        )?;

        let bandwidth_valid = self.bandwidth_manager.verify_proof(
            &challenge.node_id,
            &response.bandwidth_response,
        )?;

        let longevity_valid = self.longevity_manager.verify_proof(
            &challenge.node_id,
            &response.longevity_response,
        )?;

        Ok(storage_valid && bandwidth_valid && longevity_valid)
    }

    /// Avance à l'epoch suivant
    pub fn advance_epoch(&mut self) {
        self.current_epoch += 1;
        self.clear_expired_cache();
    }

    /// Obtient l'epoch actuel
    pub fn current_epoch(&self) -> u64 {
        self.current_epoch
    }

    /// Obtient la configuration du consensus
    pub fn config(&self) -> &ConsensusConfig {
        &self.config
    }

    /// Met à jour la configuration (requiert validation)
    pub fn update_config(&mut self, new_config: ConsensusConfig) -> Result<()> {
        new_config.validate()?;
        self.config = new_config;
        self.clear_cache();
        Ok(())
    }

    /// Obtient les statistiques du consensus
    pub fn get_statistics(&self) -> ConsensusStatistics {
        ConsensusStatistics {
            current_epoch: self.current_epoch,
            cached_scores: self.score_cache.len(),
            storage_nodes: self.storage_manager.active_nodes_count(),
            bandwidth_nodes: self.bandwidth_manager.active_nodes_count(),
            longevity_nodes: self.longevity_manager.active_nodes_count(),
        }
    }

    // Méthodes privées

    fn cache_score(&mut self, node_id: NodeId, score: ConsensusScore) {
        let expires_at = SystemTime::now() + Duration::from_secs(300); // 5 minutes
        self.score_cache.insert(node_id, CachedScore {
            score,
            expires_at,
        });
    }

    fn clear_expired_cache(&mut self) {
        let now = SystemTime::now();
        self.score_cache.retain(|_, cached| cached.expires_at > now);
    }

    fn clear_cache(&mut self) {
        self.score_cache.clear();
    }

    fn generate_nonce(&self) -> u64 {
        use rand::Rng;
        rand::thread_rng().gen()
    }
}

/// Défi de consensus combinant tous les types de preuves
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusChallenge {
    /// Nœud ciblé par le défi
    pub node_id: NodeId,
    /// Epoch du consensus
    pub epoch: u64,
    /// Défi de stockage
    pub storage_challenge: Vec<u8>,
    /// Défi de bande passante
    pub bandwidth_challenge: Vec<u8>,
    /// Défi de longévité
    pub longevity_challenge: Vec<u8>,
    /// Nonce aléatoire
    pub nonce: u64,
    /// Timestamp de création
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Expiration du défi
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl ConsensusChallenge {
    /// Calcule l'identifiant unique du défi
    pub fn id(&self) -> Hash {
        let mut data = Vec::new();
        data.extend_from_slice(self.node_id.hash().as_bytes());
        data.extend_from_slice(&self.epoch.to_le_bytes());
        data.extend_from_slice(&self.storage_challenge);
        data.extend_from_slice(&self.bandwidth_challenge);
        data.extend_from_slice(&self.longevity_challenge);
        data.extend_from_slice(&self.nonce.to_le_bytes());
        data.extend_from_slice(&self.timestamp.timestamp().to_le_bytes());

        compute_hash(&data, HashAlgorithm::Blake3)
    }

    /// Vérifie si le défi est encore valide
    pub fn is_valid(&self) -> bool {
        chrono::Utc::now() <= self.expires_at
    }
}

/// Réponse à un défi de consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResponse {
    /// Identifiant du défi
    pub challenge_id: Hash,
    /// Réponse au défi de stockage
    pub storage_response: Vec<u8>,
    /// Réponse au défi de bande passante
    pub bandwidth_response: Vec<u8>,
    /// Réponse au défi de longévité
    pub longevity_response: Vec<u8>,
    /// Timestamp de la réponse
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Statistiques du consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusStatistics {
    /// Epoch actuel
    pub current_epoch: u64,
    /// Nombre de scores en cache
    pub cached_scores: usize,
    /// Nombre de nœuds avec stockage actif
    pub storage_nodes: usize,
    /// Nombre de nœuds avec bande passante active
    pub bandwidth_nodes: usize,
    /// Nombre de nœuds avec bonus de longévité
    pub longevity_nodes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{generate_keypair, Hash};

    #[test]
    fn test_proof_of_archive_creation() {
        let config = ConsensusConfig::test_config();
        let poa = ProofOfArchive::new(config).unwrap();
        
        assert_eq!(poa.current_epoch(), 0);
        assert_eq!(poa.score_cache.len(), 0);
    }

    #[test]
    fn test_consensus_challenge_id() {
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        
        let challenge = ConsensusChallenge {
            node_id: node_id.clone(),
            epoch: 1,
            storage_challenge: vec![1, 2, 3],
            bandwidth_challenge: vec![4, 5, 6],
            longevity_challenge: vec![7, 8, 9],
            nonce: 12345,
            timestamp: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(30),
        };

        let id1 = challenge.id();
        let id2 = challenge.id();
        assert_eq!(id1, id2); // L'ID doit être déterministe

        assert!(challenge.is_valid());
    }

    #[test]
    fn test_epoch_advancement() {
        let config = ConsensusConfig::test_config();
        let mut poa = ProofOfArchive::new(config).unwrap();
        
        assert_eq!(poa.current_epoch(), 0);
        
        poa.advance_epoch();
        assert_eq!(poa.current_epoch(), 1);
        
        poa.advance_epoch();
        assert_eq!(poa.current_epoch(), 2);
    }

    #[test]
    fn test_config_update() {
        let config = ConsensusConfig::test_config();
        let mut poa = ProofOfArchive::new(config).unwrap();
        
        let mut new_config = ConsensusConfig::test_config();
        new_config.validators_per_round = 5;
        
        assert!(poa.update_config(new_config).is_ok());
        assert_eq!(poa.config().validators_per_round, 5);
        
        // Test avec config invalide
        let mut invalid_config = ConsensusConfig::test_config();
        invalid_config.storage_weight = 2.0; // Total > 1.0
        assert!(poa.update_config(invalid_config).is_err());
    }

    #[test]
    fn test_statistics() {
        let config = ConsensusConfig::test_config();
        let poa = ProofOfArchive::new(config).unwrap();
        
        let stats = poa.get_statistics();
        assert_eq!(stats.current_epoch, 0);
        assert_eq!(stats.cached_scores, 0);
    }
}