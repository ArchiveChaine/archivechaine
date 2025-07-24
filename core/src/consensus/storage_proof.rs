//! Proof of Storage pour ArchiveChain
//! 
//! Implémente un système de preuves cryptographiques pour vérifier que les nœuds
//! stockent effectivement les données qu'ils prétendent archiver

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime};
use crate::crypto::{Hash, HashAlgorithm, compute_hash, compute_combined_hash};
use crate::state::{MerkleTree, MerkleProof};
use crate::error::Result;
use super::{NodeId, ConsensusConfig, ConsensusProof};

/// Gestionnaire des preuves de stockage
#[derive(Debug)]
pub struct StorageProofManager {
    /// Configuration du consensus
    config: ConsensusConfig,
    /// Métriques de stockage par nœud
    node_metrics: HashMap<NodeId, StorageMetrics>,
    /// Défis actifs par nœud
    active_challenges: HashMap<NodeId, StorageChallenge>,
    /// Historique des preuves validées
    proof_history: HashMap<NodeId, Vec<ValidatedProof>>,
    /// Archives suivies pour les preuves
    tracked_archives: HashMap<Hash, ArchiveTrackingInfo>,
}

/// Métriques de stockage pour un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetrics {
    /// Nœud concerné
    pub node_id: NodeId,
    /// Taille totale stockée (bytes)
    pub total_stored_bytes: u64,
    /// Nombre d'archives stockées
    pub archive_count: u32,
    /// Taux de réussite des défis (0.0 - 1.0)
    pub challenge_success_rate: f64,
    /// Temps de réponse moyen aux défis (ms)
    pub avg_response_time_ms: u64,
    /// Dernière validation réussie
    pub last_successful_proof: Option<chrono::DateTime<chrono::Utc>>,
    /// Score de fiabilité (0.0 - 1.0)
    pub reliability_score: f64,
    /// Timestamp de dernière mise à jour
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Défi de preuve de stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageChallenge {
    /// Identifiant unique du défi
    pub challenge_id: Hash,
    /// Nœud ciblé
    pub node_id: NodeId,
    /// Archive à vérifier
    pub archive_hash: Hash,
    /// Positions des bytes à vérifier (échantillonnage aléatoire)
    pub sample_positions: Vec<u64>,
    /// Taille de chaque échantillon
    pub sample_size: u32,
    /// Nonce pour éviter la pré-computation
    pub nonce: u64,
    /// Timestamp de création
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Expiration du défi
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Algorithme de hash attendu
    pub hash_algorithm: HashAlgorithm,
}

/// Réponse à un défi de stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageChallengeResponse {
    /// Identifiant du défi
    pub challenge_id: Hash,
    /// Échantillons de données aux positions demandées
    pub data_samples: Vec<DataSample>,
    /// Hash combiné de tous les échantillons
    pub combined_hash: Hash,
    /// Preuve de Merkle pour la vérification
    pub merkle_proof: MerkleProof,
    /// Timestamp de la réponse
    pub responded_at: chrono::DateTime<chrono::Utc>,
}

/// Échantillon de données pour une position donnée
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSample {
    /// Position dans le fichier
    pub position: u64,
    /// Données à cette position
    pub data: Vec<u8>,
    /// Hash des données
    pub data_hash: Hash,
}

/// Preuve validée et stockée
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedProof {
    /// Hash du défi
    pub challenge_hash: Hash,
    /// Hash de l'archive prouvée
    pub archive_hash: Hash,
    /// Score obtenu (0.0 - 1.0)
    pub proof_score: f64,
    /// Temps de réponse (ms)
    pub response_time_ms: u64,
    /// Timestamp de validation
    pub validated_at: chrono::DateTime<chrono::Utc>,
}

/// Information de suivi d'une archive
#[derive(Debug, Clone)]
pub struct ArchiveTrackingInfo {
    /// Hash de l'archive
    pub archive_hash: Hash,
    /// Taille de l'archive
    pub size_bytes: u64,
    /// Nœuds qui prétendent stocker cette archive
    pub storage_nodes: HashSet<NodeId>,
    /// Nombre de vérifications réussies
    pub successful_verifications: u32,
    /// Dernière vérification
    pub last_verified: Option<chrono::DateTime<chrono::Utc>>,
}

impl StorageProofManager {
    /// Crée un nouveau gestionnaire de preuves de stockage
    pub fn new(config: &ConsensusConfig) -> Self {
        Self {
            config: config.clone(),
            node_metrics: HashMap::new(),
            active_challenges: HashMap::new(),
            proof_history: HashMap::new(),
            tracked_archives: HashMap::new(),
        }
    }

    /// Enregistre qu'un nœud stocke une archive
    pub fn register_storage(&mut self, node_id: NodeId, archive_hash: Hash, size_bytes: u64) {
        // Met à jour les métriques du nœud
        let metrics = self.node_metrics.entry(node_id.clone()).or_insert_with(|| {
            StorageMetrics {
                node_id: node_id.clone(),
                total_stored_bytes: 0,
                archive_count: 0,
                challenge_success_rate: 1.0,
                avg_response_time_ms: 0,
                last_successful_proof: None,
                reliability_score: 1.0,
                updated_at: chrono::Utc::now(),
            }
        });

        metrics.total_stored_bytes += size_bytes;
        metrics.archive_count += 1;
        metrics.updated_at = chrono::Utc::now();

        // Met à jour le suivi de l'archive
        let tracking = self.tracked_archives.entry(archive_hash.clone()).or_insert_with(|| {
            ArchiveTrackingInfo {
                archive_hash: archive_hash.clone(),
                size_bytes,
                storage_nodes: HashSet::new(),
                successful_verifications: 0,
                last_verified: None,
            }
        });

        tracking.storage_nodes.insert(node_id);
    }

    /// Génère un défi de stockage aléatoire pour un nœud
    pub fn generate_storage_challenge(&mut self, node_id: &NodeId) -> Result<StorageChallenge> {
        // Trouve une archive stockée par ce nœud
        let archive_hash = self.select_random_archive_for_node(node_id)?;
        let archive_info = self.tracked_archives.get(&archive_hash)
            .ok_or_else(|| crate::error::CoreError::Internal {
                message: "Archive introuvable pour le défi".to_string()
            })?;

        // Génère des positions aléatoires à échantillonner
        let sample_count = std::cmp::min(10, archive_info.size_bytes / 1024); // Max 10 échantillons
        let sample_positions = self.generate_random_positions(archive_info.size_bytes, sample_count as u32);

        let challenge_id = Hash::from_bytes(&rand::random::<[u8; 32]>())?;
        let nonce = rand::random::<u64>();
        let created_at = chrono::Utc::now();
        let expires_at = created_at + chrono::Duration::from_std(self.config.challenge_timeout)?;

        let challenge = StorageChallenge {
            challenge_id: challenge_id.clone(),
            node_id: node_id.clone(),
            archive_hash,
            sample_positions,
            sample_size: 1024, // 1KB par échantillon
            nonce,
            created_at,
            expires_at,
            hash_algorithm: HashAlgorithm::Blake3,
        };

        // Stocke le défi actif
        self.active_challenges.insert(node_id.clone(), challenge.clone());

        Ok(challenge)
    }

    /// Vérifie une réponse à un défi de stockage
    pub fn verify_storage_response(
        &mut self,
        challenge: &StorageChallenge,
        response: &StorageChallengeResponse,
    ) -> Result<bool> {
        // Vérifie que la réponse correspond au défi
        if response.challenge_id != challenge.challenge_id {
            return Ok(false);
        }

        // Vérifie que la réponse n'est pas expirée
        if chrono::Utc::now() > challenge.expires_at {
            return Ok(false);
        }

        // Vérifie que tous les échantillons sont présents
        if response.data_samples.len() != challenge.sample_positions.len() {
            return Ok(false);
        }

        // Vérifie chaque échantillon
        for (i, sample) in response.data_samples.iter().enumerate() {
            if sample.position != challenge.sample_positions[i] {
                return Ok(false);
            }

            // Vérifie le hash de l'échantillon
            let expected_hash = compute_hash(&sample.data, challenge.hash_algorithm);
            if expected_hash != sample.data_hash {
                return Ok(false);
            }
        }

        // Vérifie le hash combiné
        let sample_hashes: Vec<&[u8]> = response.data_samples
            .iter()
            .map(|s| s.data_hash.as_bytes())
            .collect();
        let expected_combined = compute_combined_hash(&sample_hashes, challenge.hash_algorithm);
        
        if expected_combined != response.combined_hash {
            return Ok(false);
        }

        // Vérifie la preuve de Merkle
        if !response.merkle_proof.verify(challenge.hash_algorithm) {
            return Ok(false);
        }

        // Met à jour les métriques du nœud
        self.update_node_metrics_after_challenge(&challenge.node_id, true, response.responded_at)?;

        // Enregistre la preuve validée
        self.record_validated_proof(challenge, response);

        Ok(true)
    }

    /// Obtient les métriques de stockage d'un nœud
    pub fn get_node_metrics(&self, node_id: &NodeId) -> Result<StorageMetrics> {
        self.node_metrics.get(node_id)
            .cloned()
            .ok_or_else(|| crate::error::CoreError::Internal {
                message: format!("Métriques introuvables pour le nœud {:?}", node_id)
            })
    }

    /// Calcule le score de stockage pour un nœud
    pub fn calculate_storage_score(&self, node_id: &NodeId) -> Result<f64> {
        let metrics = self.get_node_metrics(node_id)?;
        
        // Facteurs du score :
        // - Quantité de données stockées (normalisé)
        // - Taux de réussite des défis
        // - Score de fiabilité
        // - Récence de la dernière preuve

        let storage_factor = (metrics.total_stored_bytes as f64 / self.config.min_storage_proof as f64).min(1.0);
        let success_factor = metrics.challenge_success_rate;
        let reliability_factor = metrics.reliability_score;
        
        // Facteur de récence (pénalise les nœuds inactifs)
        let recency_factor = if let Some(last_proof) = metrics.last_successful_proof {
            let hours_since = chrono::Utc::now().signed_duration_since(last_proof).num_hours();
            (1.0 - (hours_since as f64 / 24.0)).max(0.1) // Décroît sur 24h, minimum 10%
        } else {
            0.5 // Score neutre pour les nouveaux nœuds
        };

        let score = storage_factor * 0.4 + success_factor * 0.3 + reliability_factor * 0.2 + recency_factor * 0.1;
        Ok(score.min(1.0))
    }

    /// Obtient le nombre de nœuds actifs avec stockage
    pub fn active_nodes_count(&self) -> usize {
        self.node_metrics.len()
    }

    /// Nettoie les défis expirés
    pub fn cleanup_expired_challenges(&mut self) {
        let now = chrono::Utc::now();
        self.active_challenges.retain(|_, challenge| challenge.expires_at > now);
    }

    // Méthodes privées

    fn select_random_archive_for_node(&self, node_id: &NodeId) -> Result<Hash> {
        let archives: Vec<&Hash> = self.tracked_archives
            .iter()
            .filter(|(_, info)| info.storage_nodes.contains(node_id))
            .map(|(hash, _)| hash)
            .collect();

        if archives.is_empty() {
            return Err(crate::error::CoreError::Internal {
                message: "Aucune archive trouvée pour ce nœud".to_string()
            });
        }

        let index = rand::random::<usize>() % archives.len();
        Ok(archives[index].clone())
    }

    fn generate_random_positions(&self, file_size: u64, count: u32) -> Vec<u64> {
        let mut positions = Vec::new();
        for _ in 0..count {
            positions.push(rand::random::<u64>() % file_size);
        }
        positions.sort();
        positions
    }

    fn update_node_metrics_after_challenge(
        &mut self,
        node_id: &NodeId,
        success: bool,
        response_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        if let Some(metrics) = self.node_metrics.get_mut(node_id) {
            // Met à jour le taux de réussite
            let total_challenges = self.proof_history.get(node_id).map(|h| h.len()).unwrap_or(0) + 1;
            let successful_challenges = if success { 1 } else { 0 } + 
                self.proof_history.get(node_id)
                    .map(|h| h.iter().filter(|p| p.proof_score > 0.5).count())
                    .unwrap_or(0);
            
            metrics.challenge_success_rate = successful_challenges as f64 / total_challenges as f64;
            
            if success {
                metrics.last_successful_proof = Some(response_time);
                // Améliore légèrement le score de fiabilité
                metrics.reliability_score = (metrics.reliability_score * 0.9 + 0.1).min(1.0);
            } else {
                // Réduit le score de fiabilité en cas d'échec
                metrics.reliability_score = (metrics.reliability_score * 0.9).max(0.1);
            }

            metrics.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    fn record_validated_proof(&mut self, challenge: &StorageChallenge, response: &StorageChallengeResponse) {
        let proof_score = if response.data_samples.len() == challenge.sample_positions.len() {
            1.0
        } else {
            response.data_samples.len() as f64 / challenge.sample_positions.len() as f64
        };

        let response_time_ms = response.responded_at
            .signed_duration_since(challenge.created_at)
            .num_milliseconds() as u64;

        let validated_proof = ValidatedProof {
            challenge_hash: challenge.challenge_id.clone(),
            archive_hash: challenge.archive_hash.clone(),
            proof_score,
            response_time_ms,
            validated_at: chrono::Utc::now(),
        };

        self.proof_history
            .entry(challenge.node_id.clone())
            .or_insert_with(Vec::new)
            .push(validated_proof);

        // Limite l'historique pour éviter une croissance excessive
        if let Some(history) = self.proof_history.get_mut(&challenge.node_id) {
            if history.len() > 100 {
                history.drain(0..50); // Garde les 50 plus récentes
            }
        }
    }
}

impl ConsensusProof for StorageProofManager {
    type Metrics = StorageMetrics;

    fn calculate_score(&self, node_id: &NodeId, _metrics: &Self::Metrics) -> Result<f64> {
        self.calculate_storage_score(node_id)
    }

    fn verify_proof(&self, _node_id: &NodeId, proof_data: &[u8]) -> Result<bool> {
        // Désérialise la réponse et vérifie
        let response: StorageChallengeResponse = bincode::deserialize(proof_data)
            .map_err(|e| crate::error::CoreError::Internal {
                message: format!("Erreur de désérialisation: {}", e)
            })?;

        // Récupère le défi correspondant
        if let Some(challenge) = self.active_challenges.values()
            .find(|c| c.challenge_id == response.challenge_id) {
            // Note: Cette méthode devrait être mutable pour une implémentation complète
            // Pour l'instant, on fait juste une vérification basique
            Ok(response.data_samples.len() > 0)
        } else {
            Ok(false)
        }
    }

    fn generate_challenge(&self, node_id: &NodeId) -> Result<Vec<u8>> {
        // Note: Cette méthode devrait être mutable pour une implémentation complète
        // Pour l'instant, on génère un défi basique
        let challenge_data = format!("storage_challenge_{}", node_id.hash());
        Ok(challenge_data.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{generate_keypair, Hash};

    #[test]
    fn test_storage_proof_manager_creation() {
        let config = ConsensusConfig::test_config();
        let manager = StorageProofManager::new(&config);
        
        assert_eq!(manager.active_nodes_count(), 0);
    }

    #[test]
    fn test_register_storage() {
        let config = ConsensusConfig::test_config();
        let mut manager = StorageProofManager::new(&config);
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        let archive_hash = Hash::from_bytes(&[1; 32]).unwrap();
        
        manager.register_storage(node_id.clone(), archive_hash, 1024 * 1024);
        
        let metrics = manager.get_node_metrics(&node_id).unwrap();
        assert_eq!(metrics.total_stored_bytes, 1024 * 1024);
        assert_eq!(metrics.archive_count, 1);
    }

    #[test]
    fn test_storage_score_calculation() {
        let config = ConsensusConfig::test_config();
        let mut manager = StorageProofManager::new(&config);
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        let archive_hash = Hash::from_bytes(&[1; 32]).unwrap();
        
        manager.register_storage(node_id.clone(), archive_hash, 2048); // 2x le minimum
        
        let score = manager.calculate_storage_score(&node_id).unwrap();
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_challenge_generation() {
        let config = ConsensusConfig::test_config();
        let mut manager = StorageProofManager::new(&config);
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        let archive_hash = Hash::from_bytes(&[1; 32]).unwrap();
        
        manager.register_storage(node_id.clone(), archive_hash, 10240);
        
        let challenge = manager.generate_storage_challenge(&node_id).unwrap();
        assert_eq!(challenge.node_id, node_id);
        assert!(!challenge.sample_positions.is_empty());
        assert!(challenge.expires_at > challenge.created_at);
    }
}