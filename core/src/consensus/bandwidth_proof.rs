//! Proof of Bandwidth pour ArchiveChain
//! 
//! Vérifie et mesure la capacité des nœuds à servir le contenu avec une bande passante suffisante

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime};
use crate::crypto::{Hash, HashAlgorithm, compute_hash};
use crate::error::Result;
use super::{NodeId, ConsensusConfig, ConsensusProof};

/// Gestionnaire des preuves de bande passante
#[derive(Debug)]
pub struct BandwidthProofManager {
    /// Configuration du consensus
    config: ConsensusConfig,
    /// Métriques de bande passante par nœud
    node_metrics: HashMap<NodeId, BandwidthMetrics>,
    /// Tests actifs de bande passante
    active_tests: HashMap<NodeId, BandwidthTest>,
    /// Historique des mesures de performance
    performance_history: HashMap<NodeId, VecDeque<PerformanceMeasurement>>,
    /// Requêtes de téléchargement en cours
    download_requests: HashMap<Hash, DownloadRequest>,
}

/// Métriques de bande passante pour un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthMetrics {
    /// Nœud concerné
    pub node_id: NodeId,
    /// Bande passante moyenne upload (bytes/sec)
    pub avg_upload_bandwidth: u64,
    /// Bande passante moyenne download (bytes/sec) 
    pub avg_download_bandwidth: u64,
    /// Latence moyenne (ms)
    pub avg_latency_ms: u64,
    /// Nombre de téléchargements servis
    pub downloads_served: u64,
    /// Bytes totaux servis
    pub total_bytes_served: u64,
    /// Taux de disponibilité (0.0 - 1.0)
    pub availability_rate: f64,
    /// Score de qualité de service (0.0 - 1.0)
    pub qos_score: f64,
    /// Dernière mesure de performance
    pub last_measurement: Option<chrono::DateTime<chrono::Utc>>,
    /// Timestamp de dernière mise à jour
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Test de bande passante pour un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthTest {
    /// Identifiant unique du test
    pub test_id: Hash,
    /// Nœud testé
    pub node_id: NodeId,
    /// Type de test
    pub test_type: BandwidthTestType,
    /// Taille des données de test (bytes)
    pub test_data_size: u64,
    /// Hash des données de test
    pub test_data_hash: Hash,
    /// Timestamp de début
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Timeout du test
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Nœuds pairs pour le test
    pub peer_nodes: Vec<NodeId>,
}

/// Types de tests de bande passante
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BandwidthTestType {
    /// Test d'upload vers le réseau
    Upload,
    /// Test de download depuis le réseau
    Download,
    /// Test bidirectionnel
    Bidirectional,
    /// Test de latence
    Latency,
}

/// Mesure de performance enregistrée
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMeasurement {
    /// Type de mesure
    pub measurement_type: BandwidthTestType,
    /// Bande passante mesurée (bytes/sec)
    pub bandwidth_bps: u64,
    /// Latence mesurée (ms)
    pub latency_ms: u64,
    /// Taille des données transférées
    pub data_size: u64,
    /// Durée du transfert (ms)
    pub transfer_duration_ms: u64,
    /// Timestamp de la mesure
    pub measured_at: chrono::DateTime<chrono::Utc>,
    /// Score de qualité (0.0 - 1.0)
    pub quality_score: f64,
}

/// Requête de téléchargement pour mesurer la performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRequest {
    /// Identifiant de la requête
    pub request_id: Hash,
    /// Archive demandée
    pub archive_hash: Hash,
    /// Nœud serveur
    pub server_node: NodeId,
    /// Nœud client
    pub client_node: NodeId,
    /// Taille attendue
    pub expected_size: u64,
    /// Timestamp de la requête
    pub requested_at: chrono::DateTime<chrono::Utc>,
    /// Timeout
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Réponse à un test de bande passante
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthTestResponse {
    /// Identifiant du test
    pub test_id: Hash,
    /// Résultats de performance
    pub performance_results: Vec<PerformanceMeasurement>,
    /// Données de preuve (échantillons des transferts)
    pub proof_data: Vec<TransferProof>,
    /// Timestamp de la réponse
    pub responded_at: chrono::DateTime<chrono::Utc>,
}

/// Preuve d'un transfert de données
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProof {
    /// Hash des données transférées
    pub data_hash: Hash,
    /// Taille transférée
    pub size_bytes: u64,
    /// Timestamp de début
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Timestamp de fin
    pub end_time: chrono::DateTime<chrono::Utc>,
    /// Nœud pair impliqué
    pub peer_node: NodeId,
    /// Direction du transfert
    pub direction: TransferDirection,
}

/// Direction d'un transfert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferDirection {
    /// Upload vers le pair
    Upload,
    /// Download depuis le pair
    Download,
}

/// Score de bande passante calculé
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthScore {
    /// Score de capacité upload (0.0 - 1.0)
    pub upload_score: f64,
    /// Score de capacité download (0.0 - 1.0)
    pub download_score: f64,
    /// Score de latence (0.0 - 1.0)
    pub latency_score: f64,
    /// Score de disponibilité (0.0 - 1.0)
    pub availability_score: f64,
    /// Score combiné final (0.0 - 1.0)
    pub combined_score: f64,
}

impl BandwidthProofManager {
    /// Crée un nouveau gestionnaire de preuves de bande passante
    pub fn new(config: &ConsensusConfig) -> Self {
        Self {
            config: config.clone(),
            node_metrics: HashMap::new(),
            active_tests: HashMap::new(),
            performance_history: HashMap::new(),
            download_requests: HashMap::new(),
        }
    }

    /// Enregistre une nouvelle mesure de performance pour un nœud
    pub fn record_performance(&mut self, node_id: NodeId, measurement: PerformanceMeasurement) {
        // Met à jour les métriques du nœud
        let metrics = self.node_metrics.entry(node_id.clone()).or_insert_with(|| {
            BandwidthMetrics {
                node_id: node_id.clone(),
                avg_upload_bandwidth: 0,
                avg_download_bandwidth: 0,
                avg_latency_ms: 0,
                downloads_served: 0,
                total_bytes_served: 0,
                availability_rate: 1.0,
                qos_score: 1.0,
                last_measurement: None,
                updated_at: chrono::Utc::now(),
            }
        });

        // Met à jour l'historique
        let history = self.performance_history.entry(node_id.clone()).or_insert_with(VecDeque::new);
        history.push_back(measurement.clone());
        
        // Limite l'historique à 100 mesures
        if history.len() > 100 {
            history.pop_front();
        }

        // Recalcule les moyennes
        self.update_average_metrics(&node_id);
        
        metrics.last_measurement = Some(measurement.measured_at);
        metrics.updated_at = chrono::Utc::now();
    }

    /// Génère un test de bande passante pour un nœud
    pub fn generate_bandwidth_test(&mut self, node_id: &NodeId, test_type: BandwidthTestType) -> Result<BandwidthTest> {
        let test_id = Hash::from_bytes(&rand::random::<[u8; 32]>())?;
        
        // Taille de test basée sur le type
        let test_data_size = match test_type {
            BandwidthTestType::Latency => 1024, // 1KB pour la latence
            BandwidthTestType::Upload | BandwidthTestType::Download => 1024 * 1024, // 1MB
            BandwidthTestType::Bidirectional => 2 * 1024 * 1024, // 2MB
        };

        // Génère des données de test
        let test_data: Vec<u8> = (0..test_data_size).map(|_| rand::random::<u8>()).collect();
        let test_data_hash = compute_hash(&test_data, HashAlgorithm::Blake3);

        // Sélectionne des nœuds pairs pour le test
        let peer_nodes = self.select_peer_nodes_for_test(node_id, 3);

        let started_at = chrono::Utc::now();
        let expires_at = started_at + chrono::Duration::from_std(self.config.challenge_timeout)?;

        let test = BandwidthTest {
            test_id: test_id.clone(),
            node_id: node_id.clone(),
            test_type,
            test_data_size,
            test_data_hash,
            started_at,
            expires_at,
            peer_nodes,
        };

        // Stocke le test actif
        self.active_tests.insert(node_id.clone(), test.clone());

        Ok(test)
    }

    /// Vérifie une réponse à un test de bande passante
    pub fn verify_bandwidth_response(
        &mut self,
        test: &BandwidthTest,
        response: &BandwidthTestResponse,
    ) -> Result<bool> {
        // Vérifie que la réponse correspond au test
        if response.test_id != test.test_id {
            return Ok(false);
        }

        // Vérifie que la réponse n'est pas expirée
        if chrono::Utc::now() > test.expires_at {
            return Ok(false);
        }

        // Vérifie les preuves de transfert
        for proof in &response.proof_data {
            if !self.verify_transfer_proof(proof, test)? {
                return Ok(false);
            }
        }

        // Vérifie la cohérence des mesures de performance
        for measurement in &response.performance_results {
            if !self.verify_performance_measurement(measurement, test)? {
                return Ok(false);
            }
        }

        // Enregistre les mesures validées
        for measurement in &response.performance_results {
            self.record_performance(test.node_id.clone(), measurement.clone());
        }

        Ok(true)
    }

    /// Calcule le score de bande passante pour un nœud
    pub fn calculate_bandwidth_score(&self, node_id: &NodeId) -> Result<BandwidthScore> {
        let metrics = self.get_node_metrics(node_id)?;
        
        // Score d'upload (normalisé par rapport au seuil minimum)
        let upload_score = (metrics.avg_upload_bandwidth as f64 / self.config.min_bandwidth_threshold as f64).min(1.0);
        
        // Score de download
        let download_score = (metrics.avg_download_bandwidth as f64 / self.config.min_bandwidth_threshold as f64).min(1.0);
        
        // Score de latence (inversé : faible latence = bon score)
        let latency_score = if metrics.avg_latency_ms > 0 {
            (1000.0 / metrics.avg_latency_ms as f64).min(1.0)
        } else {
            1.0
        };
        
        // Score de disponibilité
        let availability_score = metrics.availability_rate;
        
        // Score combiné avec pondération
        let combined_score = upload_score * 0.3 + download_score * 0.3 + latency_score * 0.2 + availability_score * 0.2;

        Ok(BandwidthScore {
            upload_score,
            download_score,
            latency_score,
            availability_score,
            combined_score: combined_score.min(1.0),
        })
    }

    /// Obtient les métriques de bande passante d'un nœud
    pub fn get_node_metrics(&self, node_id: &NodeId) -> Result<BandwidthMetrics> {
        self.node_metrics.get(node_id)
            .cloned()
            .ok_or_else(|| crate::error::CoreError::Internal {
                message: format!("Métriques de bande passante introuvables pour le nœud {:?}", node_id)
            })
    }

    /// Enregistre un téléchargement servi par un nœud
    pub fn record_download_served(&mut self, node_id: &NodeId, bytes_served: u64) {
        if let Some(metrics) = self.node_metrics.get_mut(node_id) {
            metrics.downloads_served += 1;
            metrics.total_bytes_served += bytes_served;
            metrics.updated_at = chrono::Utc::now();
        }
    }

    /// Obtient le nombre de nœuds actifs avec bande passante
    pub fn active_nodes_count(&self) -> usize {
        self.node_metrics.len()
    }

    /// Nettoie les tests expirés
    pub fn cleanup_expired_tests(&mut self) {
        let now = chrono::Utc::now();
        self.active_tests.retain(|_, test| test.expires_at > now);
        self.download_requests.retain(|_, request| request.expires_at > now);
    }

    // Méthodes privées

    fn update_average_metrics(&mut self, node_id: &NodeId) {
        if let Some(history) = self.performance_history.get(node_id) {
            if let Some(metrics) = self.node_metrics.get_mut(node_id) {
                let upload_measurements: Vec<_> = history.iter()
                    .filter(|m| matches!(m.measurement_type, BandwidthTestType::Upload | BandwidthTestType::Bidirectional))
                    .collect();
                
                let download_measurements: Vec<_> = history.iter()
                    .filter(|m| matches!(m.measurement_type, BandwidthTestType::Download | BandwidthTestType::Bidirectional))
                    .collect();

                if !upload_measurements.is_empty() {
                    metrics.avg_upload_bandwidth = upload_measurements.iter()
                        .map(|m| m.bandwidth_bps)
                        .sum::<u64>() / upload_measurements.len() as u64;
                }

                if !download_measurements.is_empty() {
                    metrics.avg_download_bandwidth = download_measurements.iter()
                        .map(|m| m.bandwidth_bps)
                        .sum::<u64>() / download_measurements.len() as u64;
                }

                if !history.is_empty() {
                    metrics.avg_latency_ms = history.iter()
                        .map(|m| m.latency_ms)
                        .sum::<u64>() / history.len() as u64;

                    metrics.qos_score = history.iter()
                        .map(|m| m.quality_score)
                        .sum::<f64>() / history.len() as f64;
                }
            }
        }
    }

    fn select_peer_nodes_for_test(&self, _node_id: &NodeId, count: usize) -> Vec<NodeId> {
        // Sélectionne des nœuds aléatoires pour le test
        // Dans une implémentation réelle, on sélectionnerait des nœuds proches géographiquement
        self.node_metrics.keys()
            .take(count)
            .cloned()
            .collect()
    }

    fn verify_transfer_proof(&self, proof: &TransferProof, test: &BandwidthTest) -> Result<bool> {
        // Vérifie que le nœud pair est dans la liste du test
        if !test.peer_nodes.contains(&proof.peer_node) {
            return Ok(false);
        }

        // Vérifie que le timing est cohérent
        if proof.end_time <= proof.start_time {
            return Ok(false);
        }

        // Vérifie que les timestamps sont dans la fenêtre du test
        if proof.start_time < test.started_at || proof.end_time > test.expires_at {
            return Ok(false);
        }

        Ok(true)
    }

    fn verify_performance_measurement(&self, measurement: &PerformanceMeasurement, test: &BandwidthTest) -> Result<bool> {
        // Vérifie que le type de mesure correspond au test
        if !self.measurement_type_matches_test(&measurement.measurement_type, &test.test_type) {
            return Ok(false);
        }

        // Vérifie que la bande passante est réaliste (pas plus de 10 Gbps)
        if measurement.bandwidth_bps > 10_000_000_000 {
            return Ok(false);
        }

        // Vérifie que la durée est cohérente avec la taille
        let expected_min_duration = (measurement.data_size * 1000) / (measurement.bandwidth_bps + 1);
        if measurement.transfer_duration_ms < expected_min_duration {
            return Ok(false);
        }

        Ok(true)
    }

    fn measurement_type_matches_test(&self, measurement_type: &BandwidthTestType, test_type: &BandwidthTestType) -> bool {
        match (measurement_type, test_type) {
            (BandwidthTestType::Upload, BandwidthTestType::Upload) => true,
            (BandwidthTestType::Download, BandwidthTestType::Download) => true,
            (BandwidthTestType::Latency, BandwidthTestType::Latency) => true,
            (_, BandwidthTestType::Bidirectional) => true,
            _ => false,
        }
    }
}

impl ConsensusProof for BandwidthProofManager {
    type Metrics = BandwidthMetrics;

    fn calculate_score(&self, node_id: &NodeId, _metrics: &Self::Metrics) -> Result<f64> {
        let score = self.calculate_bandwidth_score(node_id)?;
        Ok(score.combined_score)
    }

    fn verify_proof(&self, _node_id: &NodeId, proof_data: &[u8]) -> Result<bool> {
        // Désérialise la réponse et vérifie
        let response: BandwidthTestResponse = bincode::deserialize(proof_data)
            .map_err(|e| crate::error::CoreError::Internal {
                message: format!("Erreur de désérialisation: {}", e)
            })?;

        // Vérification basique
        Ok(!response.performance_results.is_empty() && !response.proof_data.is_empty())
    }

    fn generate_challenge(&self, node_id: &NodeId) -> Result<Vec<u8>> {
        // Génère un défi basique pour les tests
        let challenge_data = format!("bandwidth_test_{}", node_id.hash());
        Ok(challenge_data.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{generate_keypair, Hash};

    #[test]
    fn test_bandwidth_proof_manager_creation() {
        let config = ConsensusConfig::test_config();
        let manager = BandwidthProofManager::new(&config);
        
        assert_eq!(manager.active_nodes_count(), 0);
    }

    #[test]
    fn test_performance_recording() {
        let config = ConsensusConfig::test_config();
        let mut manager = BandwidthProofManager::new(&config);
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        
        let measurement = PerformanceMeasurement {
            measurement_type: BandwidthTestType::Upload,
            bandwidth_bps: 1024 * 1024, // 1 Mbps
            latency_ms: 50,
            data_size: 1024 * 1024,
            transfer_duration_ms: 1000,
            measured_at: chrono::Utc::now(),
            quality_score: 0.8,
        };
        
        manager.record_performance(node_id.clone(), measurement);
        
        let metrics = manager.get_node_metrics(&node_id).unwrap();
        assert_eq!(metrics.avg_upload_bandwidth, 1024 * 1024);
        assert_eq!(metrics.avg_latency_ms, 50);
    }

    #[test]
    fn test_bandwidth_test_generation() {
        let config = ConsensusConfig::test_config();
        let mut manager = BandwidthProofManager::new(&config);
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        
        let test = manager.generate_bandwidth_test(&node_id, BandwidthTestType::Upload).unwrap();
        
        assert_eq!(test.node_id, node_id);
        assert_eq!(test.test_data_size, 1024 * 1024);
        assert!(test.expires_at > test.started_at);
    }

    #[test]
    fn test_bandwidth_score_calculation() {
        let config = ConsensusConfig::test_config();
        let mut manager = BandwidthProofManager::new(&config);
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        
        // Ajoute des mesures de performance
        let upload_measurement = PerformanceMeasurement {
            measurement_type: BandwidthTestType::Upload,
            bandwidth_bps: 2 * 1024 * 1024, // 2 Mbps (2x le minimum de test)
            latency_ms: 25,
            data_size: 1024 * 1024,
            transfer_duration_ms: 500,
            measured_at: chrono::Utc::now(),
            quality_score: 0.9,
        };
        
        manager.record_performance(node_id.clone(), upload_measurement);
        
        let score = manager.calculate_bandwidth_score(&node_id).unwrap();
        assert!(score.combined_score > 0.0);
        assert!(score.combined_score <= 1.0);
        assert!(score.upload_score > 0.0);
    }
}