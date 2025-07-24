//! Gestionnaire principal du stockage distribué pour ArchiveChain
//! 
//! Coordonne tous les aspects du système de stockage :
//! - Orchestration de la réplication, distribution et découverte
//! - Interface unifiée pour les opérations de stockage
//! - Gestion des politiques et stratégies globales
//! - Monitoring et optimisation automatique

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, Mutex};
use crate::crypto::Hash;
use crate::consensus::NodeId;
use crate::error::Result;
use super::{
    ContentMetadata, StorageNodeInfo, StorageResult, StorageStatus, AvailabilityInfo,
    DistributedStorage, NodeType, StorageType, ReplicationStrategy, StorageMetrics,
    SearchQuery, SearchResults, ReplicationManager, DistributionManager, 
    ContentDiscovery, ArchiveStorage, BandwidthManager,
    // replication::{ReplicationManager, ReplicationConfig},
    // distribution::{DistributionManager, DistributionConfig},
    // discovery::{ContentDiscovery, DiscoveryConfig},
    // archive::{ArchiveStorage, ArchiveConfig},
    // bandwidth::{BandwidthManager, BandwidthConfig},
    // metrics::{MetricsConfig},
};

/// Configuration principale du gestionnaire de stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Configuration de réplication (temporaire)
    pub replication: (),
    /// Configuration de distribution géographique (temporaire)
    pub distribution: (),
    /// Configuration de découverte de contenu (temporaire)
    pub discovery: (),
    /// Configuration d'archivage (temporaire)
    pub archive: (),
    /// Configuration de bande passante (temporaire)
    pub bandwidth: (),
    /// Configuration des métriques (temporaire)
    pub metrics: (),
    /// Intervalle de synchronisation des nœuds
    pub node_sync_interval: Duration,
    /// Intervalle d'optimisation automatique
    pub optimization_interval: Duration,
    /// Seuil de redondance critique
    pub critical_redundancy_threshold: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            replication: (),
            distribution: (),
            discovery: (),
            archive: (),
            bandwidth: (),
            metrics: (),
            node_sync_interval: Duration::from_secs(60), // 1 minute
            optimization_interval: Duration::from_secs(3600), // 1 heure
            critical_redundancy_threshold: 2, // Moins de 2 répliques = critique
        }
    }
}

/// Statistiques globales du stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Nombre total de contenus stockés
    pub total_content_count: u64,
    /// Taille totale stockée (bytes)
    pub total_size_stored: u64,
    /// Nombre de nœuds actifs
    pub active_nodes: u32,
    /// Nombre total de répliques
    pub total_replicas: u64,
    /// Utilisation moyenne de la capacité
    pub average_capacity_usage: f64,
    /// Latence moyenne d'accès
    pub average_access_latency: Duration,
    /// Taux de disponibilité
    pub availability_rate: f64,
    /// Nombre de recherches par heure
    pub searches_per_hour: u64,
    /// Contenu le plus populaire
    pub top_content: Vec<(Hash, u64)>,
}

/// Politique de stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePolicy {
    // /// Stratégie de réplication par défaut
    // pub default_replication_strategy: ReplicationStrategy,
    /// Types de nœuds préférés par type de contenu
    pub node_preferences: HashMap<String, Vec<NodeType>>,
    /// Politiques de rétention
    pub retention_policies: Vec<RetentionPolicy>,
    /// Seuils d'alerte
    pub alert_thresholds: AlertThresholds,
}

/// Politique de rétention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Nom de la politique
    pub name: String,
    /// Filtre de contenu (regex sur content_type)
    pub content_filter: String,
    /// Durée de rétention minimale
    pub min_retention_duration: Duration,
    /// Action après expiration
    pub expiration_action: ExpirationAction,
}

/// Action à l'expiration d'une politique de rétention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExpirationAction {
    /// Déplacer vers stockage froid
    MoveToColdStorage,
    /// Réduire le nombre de répliques
    ReduceReplicas(u32),
    /// Supprimer complètement
    Delete,
    /// Demander confirmation
    RequestConfirmation,
}

/// Seuils d'alerte
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Seuil de capacité critique (%)
    pub critical_capacity_threshold: f64,
    /// Seuil de latence élevée (ms)
    pub high_latency_threshold: u32,
    /// Seuil de disponibilité faible (%)
    pub low_availability_threshold: f64,
    /// Seuil de répliques critiques
    pub critical_replicas_threshold: u32,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            critical_capacity_threshold: 90.0,
            high_latency_threshold: 1000, // 1 seconde
            low_availability_threshold: 95.0,
            critical_replicas_threshold: 2,
        }
    }
}

/// Gestionnaire principal de stockage
pub struct StorageManager {
    /// Configuration
    config: StorageConfig,
    /// Politique de stockage
    policy: StoragePolicy,
    /// Gestionnaire de réplication
    replication_manager: Arc<Mutex<ReplicationManager>>,
    /// Gestionnaire de distribution
    distribution_manager: Arc<Mutex<DistributionManager>>,
    /// Système de découverte
    discovery_system: Arc<Mutex<ContentDiscovery>>,
    /// Stockage d'archives
    archive_storage: Arc<Mutex<ArchiveStorage>>,
    /// Gestionnaire de bande passante
    bandwidth_manager: Arc<Mutex<BandwidthManager>>,
    /// Système de métriques
    metrics_system: Arc<Mutex<StorageMetrics>>,
    /// Nœuds de stockage disponibles
    available_nodes: Arc<RwLock<HashMap<NodeId, StorageNodeInfo>>>,
    /// Cache des métadonnées de contenu
    content_metadata_cache: Arc<RwLock<HashMap<Hash, ContentMetadata>>>,
    /// Dernière optimisation
    last_optimization: Mutex<SystemTime>,
}

impl StorageManager {
    /// Crée un nouveau gestionnaire de stockage
    pub async fn new(config: StorageConfig, policy: StoragePolicy) -> Result<Self> {
        let replication_manager = Arc::new(Mutex::new(
            ReplicationManager::new(config.replication.clone())
        ));
        
        let distribution_manager = Arc::new(Mutex::new(
            DistributionManager::new(config.distribution.clone())
        ));
        
        let discovery_system = Arc::new(Mutex::new(
            ContentDiscovery::new(config.discovery.clone())
        ));
        
        let archive_storage = Arc::new(Mutex::new(
            ArchiveStorage::new(config.archive.clone())?
        ));
        
        let bandwidth_manager = Arc::new(Mutex::new(
            BandwidthManager::new(config.bandwidth.clone())
        ));
        
        let metrics_system = Arc::new(Mutex::new(
            StorageMetrics::new(config.metrics.clone())
        ));

        Ok(Self {
            config,
            policy,
            replication_manager,
            distribution_manager,
            discovery_system,
            archive_storage,
            bandwidth_manager,
            metrics_system,
            available_nodes: Arc::new(RwLock::new(HashMap::new())),
            content_metadata_cache: Arc::new(RwLock::new(HashMap::new())),
            last_optimization: Mutex::new(SystemTime::now()),
        })
    }

    /// Met à jour la liste des nœuds disponibles
    pub async fn update_node_info(&self, node_id: NodeId, node_info: StorageNodeInfo) -> Result<()> {
        // Met à jour le cache des nœuds
        {
            let mut nodes = self.available_nodes.write().await;
            nodes.insert(node_id.clone(), node_info.clone());
        }

        // Met à jour les gestionnaires
        {
            let mut replication = self.replication_manager.lock().await;
            let nodes = self.available_nodes.read().await;
            replication.update_available_nodes(&nodes);
        }

        {
            let mut distribution = self.distribution_manager.lock().await;
            distribution.update_node_info(&*self.available_nodes.read().await);
        }

        Ok(())
    }

    /// Ajoute plusieurs nœuds en lot
    pub async fn add_nodes(&self, nodes: Vec<(NodeId, StorageNodeInfo)>) -> Result<()> {
        for (node_id, node_info) in nodes {
            self.update_node_info(node_id, node_info).await?;
        }
        Ok(())
    }

    /// Recherche du contenu
    pub async fn search_content(&self, query: SearchQuery) -> Result<SearchResults> {
        let discovery = self.discovery_system.lock().await;
        discovery.search(&query)
    }

    /// Obtient les contenus populaires
    pub async fn get_popular_content(&self, limit: usize) -> Result<Vec<(Hash, u64)>> {
        let discovery = self.discovery_system.lock().await;
        let popular_hashes = discovery.get_popular_content(limit);
        // Convert Vec<Hash> to Vec<(Hash, u64)> with dummy popularity scores
        let popular_with_scores = popular_hashes.into_iter()
            .enumerate()
            .map(|(i, hash)| (hash, (100 - i as u64).max(1)))
            .collect();
        Ok(popular_with_scores)
    }

    /// Vérifie et optimise automatiquement le système
    pub async fn auto_optimize(&self) -> Result<OptimizationReport> {
        let mut last_opt = self.last_optimization.lock().await;
        
        if last_opt.elapsed().unwrap_or(Duration::ZERO) < self.config.optimization_interval {
            return Ok(OptimizationReport::default());
        }

        let mut report = OptimizationReport::default();
        
        // Optimise la distribution géographique
        {
            let mut distribution = self.distribution_manager.lock().await;
            let _dist_result = distribution.optimize_distribution(&[])?;
            report.distribution_improvements = 0; // dummy value
        }

        // Réévalue les stratégies de réplication
        {
            let mut replication = self.replication_manager.lock().await;
            let discovery = self.discovery_system.lock().await;
            let popular_content = discovery.get_popular_content(1000);
            
            let _updated_content = replication.reevaluate_strategies(&popular_content)?;
            report.replication_updates = popular_content.len() as u32;
        }

        // Nettoie les caches
        {
            let mut discovery = self.discovery_system.lock().await;
            discovery.cleanup()?;
        }

        // Applique les politiques de rétention
        report.retention_actions = self.apply_retention_policies().await?;

        *last_opt = SystemTime::now();
        Ok(report)
    }

    /// Applique les politiques de rétention
    async fn apply_retention_policies(&self) -> Result<u32> {
        let mut actions_performed = 0;
        
        for policy in &self.policy.retention_policies {
            // Trouve le contenu concerné par cette politique
            // Implémentation simplifiée - dans la réalité, on filtrerait selon policy.content_filter
            let content_cache = self.content_metadata_cache.read().await;
            
            for (content_hash, metadata) in content_cache.iter() {
                let age = SystemTime::now().duration_since(
                    UNIX_EPOCH + Duration::from_secs(metadata.created_at.timestamp() as u64)
                ).unwrap_or(Duration::ZERO);

                if age > policy.min_retention_duration {
                    match &policy.expiration_action {
                        ExpirationAction::ReduceReplicas(target_replicas) => {
                            // Réduit le nombre de répliques
                            let mut replication = self.replication_manager.lock().await;
                            if let Some(strategy) = replication.get_strategy(content_hash) {
                                let mut new_strategy = strategy.clone();
                                let current_max = new_strategy.max_replicas();
                                new_strategy.set_max_replicas(current_max.min(*target_replicas));
                                replication.update_strategy(content_hash, new_strategy);
                                actions_performed += 1;
                            }
                        },
                        ExpirationAction::MoveToColdStorage => {
                            // Marque pour déplacement vers stockage froid
                            actions_performed += 1;
                        },
                        _ => {
                            // Autres actions...
                        }
                    }
                }
            }
        }

        Ok(actions_performed)
    }

    /// Vérifie les seuils d'alerte
    pub async fn check_alerts(&self) -> Result<Vec<StorageAlert>> {
        let mut alerts = Vec::new();
        let stats = self.get_storage_stats().await?;

        // Vérifie la capacité critique
        if stats.average_capacity_usage > self.policy.alert_thresholds.critical_capacity_threshold {
            alerts.push(StorageAlert {
                alert_type: AlertType::CriticalCapacity,
                message: format!(
                    "Capacité critique atteinte: {:.1}%",
                    stats.average_capacity_usage
                ),
                severity: AlertSeverity::Critical,
                timestamp: SystemTime::now(),
            });
        }

        // Vérifie la latence élevée
        if stats.average_access_latency.as_millis() > self.policy.alert_thresholds.high_latency_threshold as u128 {
            alerts.push(StorageAlert {
                alert_type: AlertType::HighLatency,
                message: format!(
                    "Latence élevée détectée: {}ms",
                    stats.average_access_latency.as_millis()
                ),
                severity: AlertSeverity::Warning,
                timestamp: SystemTime::now(),
            });
        }

        // Vérifie la disponibilité faible
        if stats.availability_rate < self.policy.alert_thresholds.low_availability_threshold {
            alerts.push(StorageAlert {
                alert_type: AlertType::LowAvailability,
                message: format!(
                    "Disponibilité faible: {:.1}%",
                    stats.availability_rate
                ),
                severity: AlertSeverity::Critical,
                timestamp: SystemTime::now(),
            });
        }

        Ok(alerts)
    }

    /// Met à jour la politique de stockage
    pub async fn update_policy(&mut self, new_policy: StoragePolicy) -> Result<()> {
        self.policy = new_policy;
        
        // Propage les changements aux gestionnaires
        // Ici on pourrait redéclencher une optimisation
        self.auto_optimize().await?;
        
        Ok(())
    }

    /// Obtient les métriques détaillées
    pub async fn get_detailed_metrics(&self) -> Result<DetailedMetrics> {
        let metrics = self.metrics_system.lock().await;
        let replication_metrics = {
            let replication = self.replication_manager.lock().await;
            replication.get_metrics().clone()
        };
        
        let distribution_stats = {
            let distribution = self.distribution_manager.lock().await;
            distribution.get_distribution_stats()
        };
        
        let discovery_stats = {
            let discovery = self.discovery_system.lock().await;
            discovery.get_stats()
        };

        Ok(DetailedMetrics {
            storage_metrics: metrics.get_current_metrics(),
            replication_metrics,
            distribution_stats,
            discovery_stats,
        })
    }

    /// Force une synchronisation complète
    pub async fn force_full_sync(&self) -> Result<SyncReport> {
        let mut report = SyncReport::default();
        
        // Synchronise tous les nœuds
        let nodes = self.available_nodes.read().await;
        report.nodes_synced = nodes.len() as u32;
        
        // Force une réévaluation complète
        let optimization = self.auto_optimize().await?;
        report.optimizations_applied = optimization.total_improvements();
        
        Ok(report)
    }
}

#[async_trait::async_trait]
#[async_trait::async_trait]
impl DistributedStorage for StorageManager {
    async fn store_content(
        &mut self,
        content_hash: &Hash,
        data: &[u8],
        metadata: ContentMetadata,
    ) -> Result<StorageResult> {
        let start_time = SystemTime::now();
        
        // Met en cache les métadonnées
        {
            let mut cache = self.content_metadata_cache.write().await;
            cache.insert(*content_hash, metadata.clone());
        }

        // Crée la stratégie de réplication
        let strategy = {
            let mut replication = self.replication_manager.lock().await;
            replication.create_strategy(*content_hash, &metadata)?
        };

        // Sélectionne les régions optimales
        let regions = {
            let distribution = self.distribution_manager.lock().await;
            distribution.select_optimal_regions(&metadata, None)?
        };

        // Calcule le nombre optimal de répliques
        let target_replicas = strategy.calculate_optimal_replicas(metadata.popularity);

        // Sélectionne les nœuds pour la réplication
        let selected_nodes = {
            let replication = self.replication_manager.lock().await;
            replication.select_nodes_for_replication(content_hash, target_replicas)?
        };

        // Stocke le contenu avec compression/chiffrement
        let stored_nodes = {
            let mut archive = self.archive_storage.lock().await;
            archive.store_content_optimized(data, &metadata, &selected_nodes).await?
        };

        // Met à jour le système de découverte
        {
            let mut discovery = self.discovery_system.lock().await;
            discovery.add_content(*content_hash, metadata.clone(), stored_nodes.clone());
        }

        // Enregistre les métriques
        {
            let mut metrics = self.metrics_system.lock().await;
            metrics.record_storage_operation(data.len() as u64, stored_nodes.len() as u32);
        }

        let storage_time = start_time.elapsed().unwrap_or(Duration::ZERO);
        let status = if stored_nodes.len() >= target_replicas as usize {
            StorageStatus::Success
        } else if stored_nodes.len() > 0 {
            StorageStatus::Partial
        } else {
            StorageStatus::Failed
        };

        Ok(StorageResult {
            content_hash: *content_hash,
            stored_nodes,
            replica_count: stored_nodes.len() as u32,
            storage_time,
            total_size_stored: data.len() as u64 * stored_nodes.len() as u64,
            status,
        })
    }

    async fn retrieve_content(&self, content_hash: &Hash) -> Result<Vec<u8>> {
        // Enregistre l'accès pour la popularité
        {
            let mut discovery = self.discovery_system.lock().await;
            discovery.record_content_access(*content_hash);
        }

        // Trouve les nœuds disponibles
        let availability = self.check_availability(content_hash).await?;
        
        if availability.nodes.is_empty() {
            return Err(crate::error::CoreError::Internal {
                message: "Contenu non trouvé".to_string(),
            });
        }

        // Sélectionne le nœud optimal (plus proche, moins chargé)
        let optimal_node = self.select_optimal_retrieval_node(&availability.nodes).await?;

        // Récupère le contenu
        let archive = self.archive_storage.lock().await;
        let data = archive.retrieve_content_from_node(content_hash, &optimal_node).await?;

        // Met à jour les métriques
        {
            let mut metrics = self.metrics_system.lock().await;
            metrics.record_retrieval_operation(data.len() as u64);
        }

        Ok(data)
    }

    async fn check_availability(&self, content_hash: &Hash) -> Result<AvailabilityInfo> {
        let discovery = self.discovery_system.lock().await;
        
        // Recherche dans la DHT et l'index
        if let Some(entry) = discovery.dht.get(content_hash) {
            let available_nodes = entry.storage_nodes.clone();
            let regions = self.get_regions_for_nodes(&available_nodes).await;
            
            Ok(AvailabilityInfo {
                content_hash: *content_hash,
                available_replicas: available_nodes.len() as u32,
                nodes: available_nodes,
                regions,
                average_latency: Duration::from_millis(50), // À calculer selon les nœuds
                availability_score: 0.95, // À calculer selon la redondance
            })
        } else {
            Ok(AvailabilityInfo {
                content_hash: *content_hash,
                available_replicas: 0,
                nodes: Vec::new(),
                regions: Vec::new(),
                average_latency: Duration::ZERO,
                availability_score: 0.0,
            })
        }
    }

    async fn update_replication_strategy(
        &mut self,
        content_hash: &Hash,
        new_strategy: ReplicationStrategy,
    ) -> Result<()> {
        let mut replication = self.replication_manager.lock().await;
        replication.update_strategy(*content_hash, new_strategy);
        Ok(())
    }

    fn get_storage_stats(&self) -> StorageStats {
        // Version synchrone pour l'implémentation du trait
        // Dans la pratique, on utiliserait la version async
        StorageStats {
            total_content_count: 0,
            total_size_stored: 0,
            active_nodes: 0,
            total_replicas: 0,
            average_capacity_usage: 0.0,
            average_access_latency: Duration::ZERO,
            availability_rate: 0.0,
            searches_per_hour: 0,
            top_content: Vec::new(),
        }
    }
}

impl StorageManager {
    /// Version async des statistiques de stockage
    pub async fn get_storage_stats(&self) -> Result<StorageStats> {
        let nodes = self.available_nodes.read().await;
        let content_cache = self.content_metadata_cache.read().await;
        let discovery = self.discovery_system.lock().await;

        let active_nodes = nodes.values()
            .filter(|n| n.is_available_for_storage())
            .count() as u32;

        let total_capacity: u64 = nodes.values().map(|n| n.total_capacity).sum();
        let used_capacity: u64 = nodes.values().map(|n| n.used_capacity).sum();
        let average_capacity_usage = if total_capacity > 0 {
            (used_capacity as f64 / total_capacity as f64) * 100.0
        } else {
            0.0
        };

        let average_latency = if !nodes.is_empty() {
            let total_latency: u64 = nodes.values()
                .map(|n| n.average_latency as u64)
                .sum();
            Duration::from_millis(total_latency / nodes.len() as u64)
        } else {
            Duration::ZERO
        };

        let total_content_count = content_cache.len() as u64;
        let popular_hashes = discovery.get_popular_content(10);
        let top_content: Vec<(Hash, u64)> = popular_hashes.into_iter()
            .enumerate()
            .map(|(i, hash)| (hash, (100 - i as u64).max(1)))
            .collect();

        Ok(StorageStats {
            total_content_count,
            total_size_stored: content_cache.values().map(|m| m.size).sum(),
            active_nodes,
            total_replicas: 0, // À calculer depuis les données de réplication
            average_capacity_usage,
            average_access_latency: average_latency,
            availability_rate: 99.0, // À calculer selon la disponibilité réelle
            searches_per_hour: 0, // À tracker dans les métriques
            top_content,
        })
    }

    /// Sélectionne le nœud optimal pour récupérer du contenu
    async fn select_optimal_retrieval_node(&self, available_nodes: &[NodeId]) -> Result<NodeId> {
        let nodes = self.available_nodes.read().await;
        
        let mut best_node = None;
        let mut best_score = f64::MIN;

        for node_id in available_nodes {
            if let Some(node_info) = nodes.get(node_id) {
                let score = node_info.performance_score();
                if score > best_score {
                    best_score = score;
                    best_node = Some(node_id.clone());
                }
            }
        }

        best_node.ok_or_else(|| crate::error::CoreError::Internal {
            message: "Aucun nœud optimal trouvé".to_string(),
        })
    }

    /// Obtient les régions pour une liste de nœuds
    async fn get_regions_for_nodes(&self, nodes: &[NodeId]) -> Vec<String> {
        let node_infos = self.available_nodes.read().await;
        nodes.iter()
            .filter_map(|node_id| node_infos.get(node_id))
            .map(|node| node.region.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }
}

/// Rapport d'optimisation
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OptimizationReport {
    /// Améliorations de distribution identifiées
    pub distribution_improvements: u32,
    /// Mises à jour de stratégies de réplication
    pub replication_updates: u32,
    /// Actions de politiques de rétention appliquées
    pub retention_actions: u32,
}

impl OptimizationReport {
    /// Retourne le nombre total d'améliorations
    pub fn total_improvements(&self) -> u32 {
        self.distribution_improvements + self.replication_updates + self.retention_actions
    }
}

/// Rapport de synchronisation
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SyncReport {
    /// Nombre de nœuds synchronisés
    pub nodes_synced: u32,
    /// Optimisations appliquées
    pub optimizations_applied: u32,
}

/// Alerte de stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageAlert {
    /// Type d'alerte
    pub alert_type: AlertType,
    /// Message d'alerte
    pub message: String,
    /// Sévérité
    pub severity: AlertSeverity,
    /// Timestamp
    pub timestamp: SystemTime,
}

/// Types d'alerte
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertType {
    /// Capacité critique
    CriticalCapacity,
    /// Latence élevée
    HighLatency,
    /// Disponibilité faible
    LowAvailability,
    /// Répliques critiques
    CriticalReplicas,
    /// Nœud hors ligne
    NodeOffline,
    /// Erreur de synchronisation
    SyncError,
}

/// Sévérité d'alerte
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Information
    Info,
    /// Avertissement
    Warning,
    /// Critique
    Critical,
}

/// Métriques détaillées
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedMetrics {
    /// Métriques de stockage simplifiées
    pub storage_metrics: StorageMetrics,
    /// Métriques de réplication
    pub replication_metrics: StorageMetrics,
    /// Statistiques de distribution
    pub distribution_stats: StorageMetrics,
    /// Statistiques de découverte
    pub discovery_stats: StorageMetrics,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Hash;

    #[tokio::test]
    async fn test_storage_manager_creation() {
        let config = StorageConfig::default();
        let policy = StoragePolicy {
            default_replication_strategy: ReplicationStrategy::from_metadata(
                &create_test_metadata(),
                &config.replication,
            ),
            node_preferences: HashMap::new(),
            retention_policies: Vec::new(),
            alert_thresholds: AlertThresholds::default(),
        };

        let manager = StorageManager::new(config, policy).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_node_management() {
        let config = StorageConfig::default();
        let policy = StoragePolicy {
            default_replication_strategy: ReplicationStrategy::from_metadata(
                &create_test_metadata(),
                &config.replication,
            ),
            node_preferences: HashMap::new(),
            retention_policies: Vec::new(),
            alert_thresholds: AlertThresholds::default(),
        };

        let manager = StorageManager::new(config, policy).await.unwrap();
        let node_id = NodeId::from(Hash::zero());
        let node_info = create_test_node_info();

        let result = manager.update_node_info(node_id.clone(), node_info).await;
        assert!(result.is_ok());

        let nodes = manager.available_nodes.read().await;
        assert!(nodes.contains_key(&node_id));
    }

    fn create_test_metadata() -> ContentMetadata {
        super::super::ContentMetadata {
            content_hash: Hash::zero(),
            size: 1024 * 1024,
            content_type: "text/html".to_string(),
            importance: super::super::replication::ContentImportance::Medium,
            popularity: 500,
            created_at: chrono::Utc::now(),
            preferred_regions: vec!["eu-west-1".to_string()],
            redundancy_level: 3,
            tags: vec!["test".to_string()],
        }
    }

    fn create_test_node_info() -> StorageNodeInfo {
        StorageNodeInfo {
            node_id: NodeId::from(Hash::zero()),
            node_type: NodeType::FullArchive,
            region: "eu-west-1".to_string(),
            total_capacity: 1_000_000_000,
            used_capacity: 500_000_000,
            supported_storage_types: vec![StorageType::Hot, StorageType::Warm],
            available_bandwidth: 1_000_000,
            average_latency: 50,
            reliability_score: 0.95,
            last_seen: chrono::Utc::now(),
            status: super::super::NodeStatus::Active,
        }
    }
}