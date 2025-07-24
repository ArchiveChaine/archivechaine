//! Registre distribué des nœuds pour ArchiveChain
//!
//! Le Node Registry maintient un registre distribué de tous les nœuds actifs :
//! - Découverte automatique des nouveaux nœuds
//! - Métriques de performance et réputation des nœuds
//! - Distribution géographique optimale
//! - Index des capacités et spécialisations
//! - Système de heartbeat et timeout

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex};

use crate::crypto::{Hash, PublicKey};
use crate::consensus::NodeId;
use crate::error::Result;
use super::ApiType;

/// Configuration du Node Registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegistryConfig {
    /// Intervalle de heartbeat
    pub heartbeat_interval: Duration,
    /// Timeout avant considérer un nœud comme offline
    pub node_timeout: Duration,
    /// Intervalle de nettoyage des nœuds inactifs
    pub cleanup_interval: Duration,
    /// Découverte automatique activée
    pub auto_discovery_enabled: bool,
    /// Intervalle de découverte automatique
    pub discovery_interval: Duration,
    /// Nombre maximum de nœuds à découvrir par cycle
    pub max_discovery_per_cycle: u32,
    /// Persistance du registre
    pub persistence_enabled: bool,
    /// Chemin de sauvegarde du registre
    pub persistence_path: String,
    /// Synchronisation inter-registres
    pub registry_sync_enabled: bool,
    /// Autres registres à synchroniser
    pub peer_registries: Vec<String>,
}

/// Type de nœud pour le registre
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    /// Nœud d'archive complet
    FullArchive,
    /// Nœud de stockage léger
    LightStorage,
    /// Nœud de relais
    Relay,
    /// Nœud passerelle
    Gateway,
}

/// Statut d'un nœud dans le registre
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Actif et disponible
    Active,
    /// En cours de démarrage
    Starting,
    /// En maintenance
    Maintenance,
    /// Surchargé
    Overloaded,
    /// Offline (timeout)
    Offline,
    /// Banni du réseau
    Banned,
}

/// Informations complètes sur un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Identifiant unique du nœud
    pub node_id: NodeId,
    /// Type de nœud
    pub node_type: NodeType,
    /// Adresse réseau
    pub address: String,
    /// Région géographique
    pub region: String,
    /// Capacités du nœud
    pub capabilities: NodeCapabilities,
    /// Statut actuel
    pub status: NodeStatus,
    /// Date d'enregistrement
    pub registered_at: chrono::DateTime<chrono::Utc>,
    /// Dernier heartbeat
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    /// Métriques de performance
    pub performance_metrics: PerformanceMetrics,
}

/// Capacités d'un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// Capacité de stockage (bytes)
    pub storage_capacity: u64,
    /// Capacité de bande passante (bytes/sec)
    pub bandwidth_capacity: u64,
    /// Poids dans le consensus
    pub consensus_weight: f64,
    /// Endpoints API disponibles
    pub api_endpoints: Vec<ApiType>,
}

/// Métriques de performance d'un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Utilisation CPU (0.0-1.0)
    pub cpu_usage: f64,
    /// Utilisation mémoire (0.0-1.0)
    pub memory_usage: f64,
    /// Utilisation stockage (0.0-1.0)
    pub storage_usage: f64,
    /// Latence réseau moyenne
    pub network_latency: Duration,
    /// Temps de fonctionnement
    pub uptime: Duration,
}

/// Score de réputation d'un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationScore {
    /// Score global (0.0-1.0)
    pub overall_score: f64,
    /// Score de fiabilité
    pub reliability_score: f64,
    /// Score de performance
    pub performance_score: f64,
    /// Score de disponibilité
    pub availability_score: f64,
    /// Nombre d'interactions
    pub interaction_count: u64,
    /// Dernière mise à jour
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// Historique des scores
    pub score_history: Vec<HistoricalScore>,
}

/// Score historique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalScore {
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Score à ce moment
    pub score: f64,
    /// Métriques associées
    pub metrics: PerformanceMetrics,
}

/// Index géographique des nœuds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeographicIndex {
    /// Nœuds par région
    pub nodes_by_region: HashMap<String, Vec<NodeId>>,
    /// Latence inter-régions
    pub inter_region_latency: HashMap<(String, String), Duration>,
    /// Régions disponibles
    pub available_regions: HashSet<String>,
    /// Distribution recommandée
    pub recommended_distribution: HashMap<String, u32>,
}

/// Événement de découverte
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryEvent {
    /// Timestamp de l'événement
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Type d'événement
    pub event_type: DiscoveryEventType,
    /// Nœud concerné
    pub node_id: NodeId,
    /// Détails de l'événement
    pub details: String,
}

/// Types d'événements de découverte
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscoveryEventType {
    /// Nouveau nœud découvert
    NodeDiscovered,
    /// Nœud disparu
    NodeLost,
    /// Nœud mis à jour
    NodeUpdated,
    /// Heartbeat reçu
    HeartbeatReceived,
    /// Timeout détecté
    TimeoutDetected,
}

/// Registre distribué des nœuds
pub struct NodeRegistry {
    /// Configuration
    config: NodeRegistryConfig,
    /// Nœuds enregistrés
    registered_nodes: Arc<RwLock<HashMap<NodeId, NodeInfo>>>,
    /// Scores de réputation
    reputation_scores: Arc<RwLock<HashMap<NodeId, ReputationScore>>>,
    /// Index géographique
    geographic_index: Arc<RwLock<GeographicIndex>>,
    /// Événements de découverte récents
    discovery_events: Arc<RwLock<Vec<DiscoveryEvent>>>,
    /// Nœuds en cours de découverte
    discovery_queue: Arc<Mutex<HashSet<String>>>,
    /// Dernière synchronisation
    last_sync: Arc<Mutex<SystemTime>>,
    /// Statistiques du registre
    stats: Arc<RwLock<RegistryStats>>,
}

/// Statistiques du registre
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStats {
    /// Nombre total de nœuds
    pub total_nodes: u32,
    /// Nœuds actifs
    pub active_nodes: u32,
    /// Nœuds par type
    pub nodes_by_type: HashMap<NodeType, u32>,
    /// Nœuds par région
    pub nodes_by_region: HashMap<String, u32>,
    /// Score de réputation moyen
    pub average_reputation: f64,
    /// Temps de réponse moyen
    pub average_response_time: Duration,
    /// Événements de découverte (dernières 24h)
    pub recent_discovery_events: u32,
}

impl Default for NodeRegistryConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(30),
            node_timeout: Duration::from_secs(300), // 5 minutes
            cleanup_interval: Duration::from_secs(600), // 10 minutes
            auto_discovery_enabled: true,
            discovery_interval: Duration::from_secs(60),
            max_discovery_per_cycle: 10,
            persistence_enabled: true,
            persistence_path: "./registry.json".to_string(),
            registry_sync_enabled: true,
            peer_registries: Vec::new(),
        }
    }
}

impl NodeRegistry {
    /// Crée un nouveau registre de nœuds
    pub async fn new(config: NodeRegistryConfig) -> Result<Self> {
        let registry = Self {
            config,
            registered_nodes: Arc::new(RwLock::new(HashMap::new())),
            reputation_scores: Arc::new(RwLock::new(HashMap::new())),
            geographic_index: Arc::new(RwLock::new(GeographicIndex {
                nodes_by_region: HashMap::new(),
                inter_region_latency: HashMap::new(),
                available_regions: HashSet::new(),
                recommended_distribution: HashMap::new(),
            })),
            discovery_events: Arc::new(RwLock::new(Vec::new())),
            discovery_queue: Arc::new(Mutex::new(HashSet::new())),
            last_sync: Arc::new(Mutex::new(SystemTime::now())),
            stats: Arc::new(RwLock::new(RegistryStats {
                total_nodes: 0,
                active_nodes: 0,
                nodes_by_type: HashMap::new(),
                nodes_by_region: HashMap::new(),
                average_reputation: 0.0,
                average_response_time: Duration::ZERO,
                recent_discovery_events: 0,
            })),
        };

        // Charge les données persistées si disponibles
        if registry.config.persistence_enabled {
            registry.load_persisted_data().await?;
        }

        Ok(registry)
    }

    /// Enregistre un nouveau nœud
    pub async fn register_node(&mut self, node_info: NodeInfo) -> Result<()> {
        let node_id = node_info.node_id.clone();
        
        // Enregistre le nœud
        {
            let mut nodes = self.registered_nodes.write().await;
            nodes.insert(node_id.clone(), node_info.clone());
        }

        // Initialise le score de réputation
        {
            let mut scores = self.reputation_scores.write().await;
            scores.insert(node_id.clone(), ReputationScore {
                overall_score: 0.5, // Score neutre initial
                reliability_score: 0.5,
                performance_score: 0.5,
                availability_score: 1.0, // Disponible au démarrage
                interaction_count: 0,
                last_updated: chrono::Utc::now(),
                score_history: Vec::new(),
            });
        }

        // Met à jour l'index géographique
        {
            let mut geo_index = self.geographic_index.write().await;
            geo_index.nodes_by_region
                .entry(node_info.region.clone())
                .or_insert_with(Vec::new)
                .push(node_id.clone());
            geo_index.available_regions.insert(node_info.region.clone());
        }

        // Enregistre l'événement
        self.record_discovery_event(DiscoveryEvent {
            timestamp: chrono::Utc::now(),
            event_type: DiscoveryEventType::NodeDiscovered,
            node_id: node_id.clone(),
            details: format!("Nouveau nœud {:?} enregistré", node_info.node_type),
        }).await;

        // Met à jour les statistiques
        self.update_stats().await;

        log::info!("Nœud {:?} enregistré avec succès", node_id);
        Ok(())
    }

    /// Supprime un nœud du registre
    pub async fn unregister_node(&mut self, node_id: &NodeId) -> Result<()> {
        let removed_node = {
            let mut nodes = self.registered_nodes.write().await;
            nodes.remove(node_id)
        };

        if let Some(node_info) = removed_node {
            // Supprime du score de réputation
            {
                let mut scores = self.reputation_scores.write().await;
                scores.remove(node_id);
            }

            // Met à jour l'index géographique
            {
                let mut geo_index = self.geographic_index.write().await;
                if let Some(region_nodes) = geo_index.nodes_by_region.get_mut(&node_info.region) {
                    region_nodes.retain(|id| id != node_id);
                    if region_nodes.is_empty() {
                        geo_index.nodes_by_region.remove(&node_info.region);
                        geo_index.available_regions.remove(&node_info.region);
                    }
                }
            }

            // Enregistre l'événement
            self.record_discovery_event(DiscoveryEvent {
                timestamp: chrono::Utc::now(),
                event_type: DiscoveryEventType::NodeLost,
                node_id: node_id.clone(),
                details: "Nœud supprimé du registre".to_string(),
            }).await;

            // Met à jour les statistiques
            self.update_stats().await;

            log::info!("Nœud {:?} supprimé du registre", node_id);
            Ok(())
        } else {
            Err(crate::error::CoreError::NotFound {
                message: format!("Nœud {:?} non trouvé dans le registre", node_id),
            })
        }
    }

    /// Met à jour les informations d'un nœud
    pub async fn update_node_info(&mut self, node_id: &NodeId, updated_info: NodeInfo) -> Result<()> {
        {
            let mut nodes = self.registered_nodes.write().await;
            if let Some(existing_info) = nodes.get_mut(node_id) {
                *existing_info = updated_info;
            } else {
                return Err(crate::error::CoreError::NotFound {
                    message: format!("Nœud {:?} non trouvé pour mise à jour", node_id),
                });
            }
        }

        // Enregistre l'événement
        self.record_discovery_event(DiscoveryEvent {
            timestamp: chrono::Utc::now(),
            event_type: DiscoveryEventType::NodeUpdated,
            node_id: node_id.clone(),
            details: "Informations du nœud mises à jour".to_string(),
        }).await;

        // Met à jour les statistiques
        self.update_stats().await;

        Ok(())
    }

    /// Traite un heartbeat d'un nœud
    pub async fn process_heartbeat(&mut self, node_id: &NodeId, metrics: PerformanceMetrics) -> Result<()> {
        {
            let mut nodes = self.registered_nodes.write().await;
            if let Some(node_info) = nodes.get_mut(node_id) {
                node_info.last_heartbeat = chrono::Utc::now();
                node_info.performance_metrics = metrics.clone();
                
                // Met à jour le statut si nécessaire
                if node_info.status == NodeStatus::Offline {
                    node_info.status = NodeStatus::Active;
                }
            } else {
                return Err(crate::error::CoreError::NotFound {
                    message: format!("Nœud {:?} non trouvé pour heartbeat", node_id),
                });
            }
        }

        // Met à jour le score de réputation
        self.update_reputation_score(node_id, &metrics).await?;

        // Enregistre l'événement
        self.record_discovery_event(DiscoveryEvent {
            timestamp: chrono::Utc::now(),
            event_type: DiscoveryEventType::HeartbeatReceived,
            node_id: node_id.clone(),
            details: format!("Heartbeat reçu - CPU: {:.1}%", metrics.cpu_usage * 100.0),
        }).await;

        Ok(())
    }

    /// Met à jour le score de réputation d'un nœud
    pub async fn update_reputation_score(&mut self, node_id: &NodeId, metrics: &PerformanceMetrics) -> Result<()> {
        let mut scores = self.reputation_scores.write().await;
        let reputation = scores.entry(node_id.clone()).or_insert_with(|| ReputationScore {
            overall_score: 0.5,
            reliability_score: 0.5,
            performance_score: 0.5,
            availability_score: 1.0,
            interaction_count: 0,
            last_updated: chrono::Utc::now(),
            score_history: Vec::new(),
        });

        // Calcule le nouveau score de performance basé sur les métriques
        let performance_score = Self::calculate_performance_score(metrics);
        
        // Met à jour les scores avec un lissage exponentiel
        let alpha = 0.1; // Facteur de lissage
        reputation.performance_score = alpha * performance_score + (1.0 - alpha) * reputation.performance_score;
        
        // Le score de fiabilité augmente avec le temps de fonctionnement
        let reliability_factor = (metrics.uptime.as_secs() as f64 / 86400.0).min(1.0); // Max 1 jour
        reputation.reliability_score = alpha * reliability_factor + (1.0 - alpha) * reputation.reliability_score;

        // Score global combiné
        reputation.overall_score = (reputation.performance_score * 0.4 + 
                                   reputation.reliability_score * 0.3 + 
                                   reputation.availability_score * 0.3).min(1.0);

        // Enregistre dans l'historique
        reputation.score_history.push(HistoricalScore {
            timestamp: chrono::Utc::now(),
            score: reputation.overall_score,
            metrics: metrics.clone(),
        });

        // Garde seulement les 100 derniers scores
        if reputation.score_history.len() > 100 {
            reputation.score_history.remove(0);
        }

        reputation.interaction_count += 1;
        reputation.last_updated = chrono::Utc::now();

        Ok(())
    }

    /// Calcule le score de performance basé sur les métriques
    fn calculate_performance_score(metrics: &PerformanceMetrics) -> f64 {
        // Score basé sur l'utilisation des ressources (plus bas = mieux)
        let cpu_score = (1.0 - metrics.cpu_usage).max(0.0);
        let memory_score = (1.0 - metrics.memory_usage).max(0.0);
        let storage_score = (1.0 - metrics.storage_usage).max(0.0);
        
        // Score de latence (plus bas = mieux)
        let latency_score = if metrics.network_latency.as_millis() > 0 {
            (1000.0 / (metrics.network_latency.as_millis() as f64 + 1.0)).min(1.0)
        } else {
            1.0
        };

        // Score combiné
        (cpu_score * 0.3 + memory_score * 0.3 + storage_score * 0.2 + latency_score * 0.2).min(1.0)
    }

    /// Découvre automatiquement de nouveaux nœuds
    pub async fn auto_discover_nodes(&mut self) -> Result<u32> {
        if !self.config.auto_discovery_enabled {
            return Ok(0);
        }

        let mut discovered = 0;
        
        // Simulation de découverte automatique
        // Dans la réalité, on utiliserait mDNS, DHT, ou d'autres mécanismes
        
        // Pour cette implémentation, on simule la découverte
        // En production, cela impliquerait :
        // - Scan réseau local
        // - Interrogation de nœuds de bootstrap
        // - Annonces de découverte
        // - DHT lookups

        log::debug!("Découverte automatique terminée: {} nouveaux nœuds", discovered);
        Ok(discovered)
    }

    /// Nettoie les nœuds inactifs
    pub async fn cleanup_inactive_nodes(&mut self) -> Result<u32> {
        let mut removed_count = 0;
        let timeout_threshold = SystemTime::now() - self.config.node_timeout;
        let mut nodes_to_remove = Vec::new();

        // Identifie les nœuds à supprimer
        {
            let mut nodes = self.registered_nodes.write().await;
            for (node_id, node_info) in nodes.iter_mut() {
                let last_seen = node_info.last_heartbeat.timestamp() as u64;
                let last_seen_time = SystemTime::UNIX_EPOCH + Duration::from_secs(last_seen);
                
                if last_seen_time < timeout_threshold && node_info.status != NodeStatus::Banned {
                    node_info.status = NodeStatus::Offline;
                    
                    // Marque pour suppression après timeout prolongé
                    let extended_timeout = timeout_threshold - self.config.node_timeout;
                    if last_seen_time < extended_timeout {
                        nodes_to_remove.push(node_id.clone());
                    }
                }
            }
        }

        // Supprime les nœuds inactifs
        for node_id in nodes_to_remove {
            self.unregister_node(&node_id).await?;
            removed_count += 1;
        }

        if removed_count > 0 {
            log::info!("Nettoyage terminé: {} nœuds inactifs supprimés", removed_count);
        }

        Ok(removed_count)
    }

    /// Obtient les informations d'un nœud
    pub async fn get_node_info(&self, node_id: &NodeId) -> Result<Option<NodeInfo>> {
        let nodes = self.registered_nodes.read().await;
        Ok(nodes.get(node_id).cloned())
    }

    /// Obtient le score de réputation d'un nœud
    pub async fn get_reputation_score(&self, node_id: &NodeId) -> Option<ReputationScore> {
        let scores = self.reputation_scores.read().await;
        scores.get(node_id).cloned()
    }

    /// Liste tous les nœuds actifs
    pub async fn list_active_nodes(&self) -> Vec<NodeInfo> {
        let nodes = self.registered_nodes.read().await;
        nodes.values()
            .filter(|node| node.status == NodeStatus::Active)
            .cloned()
            .collect()
    }

    /// Liste les nœuds par type
    pub async fn list_nodes_by_type(&self, node_type: &NodeType) -> Vec<NodeInfo> {
        let nodes = self.registered_nodes.read().await;
        nodes.values()
            .filter(|node| &node.node_type == node_type)
            .cloned()
            .collect()
    }

    /// Liste les nœuds par région
    pub async fn list_nodes_by_region(&self, region: &str) -> Vec<NodeInfo> {
        let nodes = self.registered_nodes.read().await;
        nodes.values()
            .filter(|node| node.region == region)
            .cloned()
            .collect()
    }

    /// Obtient l'index géographique
    pub async fn get_geographic_index(&self) -> GeographicIndex {
        let geo_index = self.geographic_index.read().await;
        geo_index.clone()
    }

    /// Recommande des nœuds pour une opération
    pub async fn recommend_nodes(&self, criteria: NodeSelectionCriteria) -> Vec<NodeId> {
        let nodes = self.registered_nodes.read().await;
        let scores = self.reputation_scores.read().await;

        let mut candidates: Vec<_> = nodes.iter()
            .filter(|(_, node)| {
                // Filtre par type si spécifié
                if let Some(ref required_type) = criteria.node_type {
                    if &node.node_type != required_type {
                        return false;
                    }
                }

                // Filtre par région si spécifié
                if let Some(ref required_region) = criteria.region {
                    if &node.region != required_region {
                        return false;
                    }
                }

                // Filtre par statut
                node.status == NodeStatus::Active
            })
            .map(|(node_id, node)| {
                let reputation = scores.get(node_id)
                    .map(|s| s.overall_score)
                    .unwrap_or(0.5);
                (node_id.clone(), reputation)
            })
            .collect();

        // Trie par score de réputation décroissant
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Retourne les meilleurs candidats
        candidates.into_iter()
            .take(criteria.max_nodes.unwrap_or(10) as usize)
            .map(|(node_id, _)| node_id)
            .collect()
    }

    /// Enregistre un événement de découverte
    async fn record_discovery_event(&self, event: DiscoveryEvent) {
        let mut events = self.discovery_events.write().await;
        events.push(event);

        // Garde seulement les 1000 derniers événements
        if events.len() > 1000 {
            events.remove(0);
        }
    }

    /// Met à jour les statistiques du registre
    async fn update_stats(&self) {
        let nodes = self.registered_nodes.read().await;
        let scores = self.reputation_scores.read().await;
        let mut stats = self.stats.write().await;

        stats.total_nodes = nodes.len() as u32;
        stats.active_nodes = nodes.values()
            .filter(|node| node.status == NodeStatus::Active)
            .count() as u32;

        // Compte par type
        let mut nodes_by_type = HashMap::new();
        for node in nodes.values() {
            *nodes_by_type.entry(node.node_type.clone()).or_insert(0) += 1;
        }
        stats.nodes_by_type = nodes_by_type;

        // Compte par région
        let mut nodes_by_region = HashMap::new();
        for node in nodes.values() {
            *nodes_by_region.entry(node.region.clone()).or_insert(0) += 1;
        }
        stats.nodes_by_region = nodes_by_region;

        // Score de réputation moyen
        if !scores.is_empty() {
            let total_score: f64 = scores.values().map(|s| s.overall_score).sum();
            stats.average_reputation = total_score / scores.len() as f64;
        }

        // Temps de réponse moyen
        if !nodes.is_empty() {
            let total_latency: Duration = nodes.values()
                .map(|n| n.performance_metrics.network_latency)
                .sum();
            stats.average_response_time = total_latency / nodes.len() as u32;
        }

        // Événements récents
        let events = self.discovery_events.read().await;
        let twenty_four_hours_ago = chrono::Utc::now() - chrono::Duration::hours(24);
        stats.recent_discovery_events = events.iter()
            .filter(|event| event.timestamp > twenty_four_hours_ago)
            .count() as u32;
    }

    /// Charge les données persistées
    async fn load_persisted_data(&self) -> Result<()> {
        // Simulation de chargement des données persistées
        // Dans la réalité, on chargerait depuis un fichier JSON ou une base de données
        log::debug!("Chargement des données persistées depuis {}", self.config.persistence_path);
        Ok(())
    }

    /// Obtient les statistiques du registre
    pub async fn get_stats(&self) -> RegistryStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
}

/// Critères de sélection de nœuds
#[derive(Debug, Clone)]
pub struct NodeSelectionCriteria {
    /// Type de nœud requis
    pub node_type: Option<NodeType>,
    /// Région requise
    pub region: Option<String>,
    /// Score de réputation minimum
    pub min_reputation: Option<f64>,
    /// Capacités minimales requises
    pub min_capabilities: Option<NodeCapabilities>,
    /// Nombre maximum de nœuds à retourner
    pub max_nodes: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_registry_creation() {
        let config = NodeRegistryConfig::default();
        let registry = NodeRegistry::new(config).await;
        assert!(registry.is_ok());
    }

    #[tokio::test]
    async fn test_node_registration() {
        let config = NodeRegistryConfig::default();
        let mut registry = NodeRegistry::new(config).await.unwrap();

        let node_info = NodeInfo {
            node_id: NodeId::from(Hash::zero()),
            node_type: NodeType::FullArchive,
            address: "127.0.0.1:8080".to_string(),
            region: "us-east-1".to_string(),
            capabilities: NodeCapabilities {
                storage_capacity: 1_000_000_000,
                bandwidth_capacity: 100_000_000,
                consensus_weight: 1.0,
                api_endpoints: vec![ApiType::Rest],
            },
            status: NodeStatus::Active,
            registered_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
            performance_metrics: PerformanceMetrics {
                cpu_usage: 0.5,
                memory_usage: 0.4,
                storage_usage: 0.3,
                network_latency: Duration::from_millis(50),
                uptime: Duration::from_secs(3600),
            },
        };

        let result = registry.register_node(node_info.clone()).await;
        assert!(result.is_ok());

        // Vérifie que le nœud est enregistré
        let retrieved = registry.get_node_info(&node_info.node_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().node_id, node_info.node_id);
    }

    #[tokio::test]
    async fn test_heartbeat_processing() {
        let config = NodeRegistryConfig::default();
        let mut registry = NodeRegistry::new(config).await.unwrap();

        let node_info = NodeInfo {
            node_id: NodeId::from(Hash::zero()),
            node_type: NodeType::Relay,
            address: "127.0.0.1:8081".to_string(),
            region: "eu-west-1".to_string(),
            capabilities: NodeCapabilities {
                storage_capacity: 0,
                bandwidth_capacity: 1_000_000_000,
                consensus_weight: 0.3,
                api_endpoints: Vec::new(),
            },
            status: NodeStatus::Active,
            registered_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
            performance_metrics: PerformanceMetrics {
                cpu_usage: 0.2,
                memory_usage: 0.3,
                storage_usage: 0.0,
                network_latency: Duration::from_millis(20),
                uptime: Duration::from_secs(7200),
            },
        };

        registry.register_node(node_info.clone()).await.unwrap();

        // Traite un heartbeat
        let updated_metrics = PerformanceMetrics {
            cpu_usage: 0.4,
            memory_usage: 0.5,
            storage_usage: 0.0,
            network_latency: Duration::from_millis(30),
            uptime: Duration::from_secs(7260),
        };

        let result = registry.process_heartbeat(&node_info.node_id, updated_metrics).await;
        assert!(result.is_ok());

        // Vérifie la mise à jour du score de réputation
        let reputation = registry.get_reputation_score(&node_info.node_id).await;
        assert!(reputation.is_some());
        let score = reputation.unwrap();
        assert!(score.overall_score > 0.0);
        assert_eq!(score.interaction_count, 1);
    }

    #[tokio::test]
    async fn test_node_recommendation() {
        let config = NodeRegistryConfig::default();
        let mut registry = NodeRegistry::new(config).await.unwrap();

        // Ajoute plusieurs nœuds
        for i in 0..5 {
            let node_info = NodeInfo {
                node_id: NodeId::from(Hash::from_bytes(&[i as u8; 32]).unwrap()),
                node_type: NodeType::FullArchive,
                address: format!("127.0.0.{}:8080", i + 1),
                region: "us-east-1".to_string(),
                capabilities: NodeCapabilities {
                    storage_capacity: 1_000_000_000,
                    bandwidth_capacity: 100_000_000,
                    consensus_weight: 1.0,
                    api_endpoints: vec![ApiType::Rest],
                },
                status: NodeStatus::Active,
                registered_at: chrono::Utc::now(),
                last_heartbeat: chrono::Utc::now(),
                performance_metrics: PerformanceMetrics {
                    cpu_usage: 0.3,
                    memory_usage: 0.4,
                    storage_usage: 0.2,
                    network_latency: Duration::from_millis(40),
                    uptime: Duration::from_secs(3600),
                },
            };

            registry.register_node(node_info).await.unwrap();
        }

        // Teste la recommandation
        let criteria = NodeSelectionCriteria {
            node_type: Some(NodeType::FullArchive),
            region: Some("us-east-1".to_string()),
            min_reputation: None,
            min_capabilities: None,
            max_nodes: Some(3),
        };

        let recommendations = registry.recommend_nodes(criteria).await;
        assert_eq!(recommendations.len(), 3);
    }

    #[test]
    fn test_performance_score_calculation() {
        let metrics = PerformanceMetrics {
            cpu_usage: 0.3,
            memory_usage: 0.4,
            storage_usage: 0.2,
            network_latency: Duration::from_millis(50),
            uptime: Duration::from_secs(3600),
        };

        let score = NodeRegistry::calculate_performance_score(&metrics);
        assert!(score > 0.0 && score <= 1.0);
        
        // Score plus élevé pour de meilleures performances
        let better_metrics = PerformanceMetrics {
            cpu_usage: 0.1,
            memory_usage: 0.2,
            storage_usage: 0.1,
            network_latency: Duration::from_millis(10),
            uptime: Duration::from_secs(3600),
        };

        let better_score = NodeRegistry::calculate_performance_score(&better_metrics);
        assert!(better_score > score);
    }
}