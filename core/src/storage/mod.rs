//! Module de stockage distribué pour ArchiveChain
//! 
//! Implémente le système de stockage et réplication selon les spécifications :
//! - Réplication adaptative basée sur la popularité
//! - Distribution géographique des données
//! - Redondance configurable (3-15 copies)
//! - Système de découverte de contenu (DHT)
//! - Gestion optimisée de la bande passante
//! - Métriques et monitoring en temps réel

pub mod manager;
// pub mod replication;
// pub mod distribution;
// pub mod discovery;
// pub mod archive;
// pub mod bandwidth;
// pub mod metrics;

// Re-exports publics
pub use manager::{
    StorageManager, StorageConfig, StorageStats, StoragePolicy,
    AlertThresholds, RetentionPolicy
};
// pub use replication::{
//     ReplicationStrategy, ReplicationManager, ContentImportance, 
//     ReplicationMetrics, AdaptiveReplication
// };
// pub use distribution::{
//     GeographicDistribution, Region, RegionInfo, PlacementStrategy,
//     DistributionManager, LatencyOptimizer
// };
// pub use discovery::{
//     ContentDiscovery, DistributedHashTable, ContentIndex, SearchCache,
//     PopularityTracker, SearchQuery, SearchResult, SearchResults
// };
// pub use archive::{
//     ArchiveStorage, CompressionConfig, EncryptionConfig, ChunkManager,
//     DeduplicationEngine, IntegrityChecker
// };
// pub use bandwidth::{
//     BandwidthManager, BandwidthLimits, PriorityQueues, QoSPolicies,
//     TransferManager, LoadBalancer
// };
// pub use metrics::{
//     StorageMetrics, PerformanceMetrics, HealthMetrics, AlertManager,
//     MetricsCollector, CapacityMonitor
// };



use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use crate::crypto::{Hash, PublicKey};
use crate::consensus::NodeId;
use crate::error::Result;

/// Importance temporaire du contenu (version simplifiée)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ContentImportance {
    Critical,
    High,
    Medium,
    Low,
}

/// Types de nœuds de stockage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    /// Nœud d'archive complet (stockage à long terme)
    FullArchive,
    /// Nœud de stockage léger (cache et réplication)
    LightStorage,
    /// Nœud spécialisé pour le contenu populaire
    HotStorage,
    /// Nœud de stockage froid (archivage long terme)
    ColdStorage,
}

/// Types de stockage par niveau de température
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageType {
    /// Stockage chaud : accès fréquent, SSD recommandé
    Hot,
    /// Stockage tiède : accès occasionnel, HDD acceptable
    Warm,
    /// Stockage froid : archivage long terme, tape/cloud
    Cold,
}

/// Stratégie de réplication simplifiée (temporaire)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationStrategy {
    /// Réplication basée sur la popularité
    PopularityBased { min_copies: u8, max_copies: u8 },
    /// Réplication fixe
    Fixed { copies: u8 },
    /// Réplication géographique
    Geographic { regions: Vec<String> },
}

impl ReplicationStrategy {
    /// Crée une stratégie basée sur les métadonnées
    pub fn from_metadata(metadata: &ContentMetadata) -> Self {
        match metadata.importance {
            ContentImportance::Critical => Self::Fixed { copies: 15 },
            ContentImportance::High => Self::PopularityBased { min_copies: 5, max_copies: 10 },
            ContentImportance::Medium => Self::PopularityBased { min_copies: 3, max_copies: 7 },
            ContentImportance::Low => Self::Fixed { copies: 3 },
        }
    }
}

/// Métriques de stockage simplifiées (temporaire)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetrics {
    /// Capacité totale de stockage
    pub total_capacity: u64,
    /// Espace utilisé
    pub used_capacity: u64,
    /// Nombre de fichiers stockés
    pub file_count: u64,
    /// Débit moyen
    pub average_throughput: f64,
}

/// Requête de recherche simplifiée (temporaire)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Termes de recherche
    pub terms: Vec<String>,
    /// Filtres
    pub filters: HashMap<String, String>,
    /// Limite de résultats
    pub limit: Option<usize>,
}

/// Résultats de recherche simplifiés (temporaire)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// Métadonnées des contenus trouvés
    pub results: Vec<ContentMetadata>,
    /// Nombre total de résultats
    pub total_count: usize,
    /// Temps de recherche en millisecondes
    pub search_time_ms: u64,
}

/// Configuration globale du système de stockage
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct GlobalStorageConfig {
//     /// Configuration de réplication
//     pub replication: replication::ReplicationConfig,
//     /// Configuration de distribution géographique
//     pub distribution: distribution::DistributionConfig,
//     /// Configuration de découverte de contenu
//     pub discovery: discovery::DiscoveryConfig,
//     /// Configuration d'archivage
//     pub archive: archive::ArchiveConfig,
//     /// Configuration de bande passante
//     pub bandwidth: bandwidth::BandwidthConfig,
//     /// Configuration des métriques
//     pub metrics: metrics::MetricsConfig,
// }

// impl Default for GlobalStorageConfig {
//     fn default() -> Self {
//         Self {
//             replication: replication::ReplicationConfig::default(),
//             distribution: distribution::DistributionConfig::default(),
//             discovery: discovery::DiscoveryConfig::default(),
//             archive: archive::ArchiveConfig::default(),
//             bandwidth: bandwidth::BandwidthConfig::default(),
//             metrics: metrics::MetricsConfig::default(),
//         }
//     }
// }

/// Informations sur un nœud de stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageNodeInfo {
    /// Identifiant du nœud
    pub node_id: NodeId,
    /// Type de nœud
    pub node_type: NodeType,
    /// Région géographique
    pub region: String,
    /// Capacité totale en bytes
    pub total_capacity: u64,
    /// Espace utilisé en bytes
    pub used_capacity: u64,
    /// Types de stockage supportés
    pub supported_storage_types: Vec<StorageType>,
    /// Bande passante disponible (bytes/sec)
    pub available_bandwidth: u64,
    /// Latence moyenne (millisecondes)
    pub average_latency: u32,
    /// Score de fiabilité (0.0-1.0)
    pub reliability_score: f64,
    /// Timestamp de dernière mise à jour
    pub last_seen: chrono::DateTime<chrono::Utc>,
    /// Statut du nœud
    pub status: NodeStatus,
}

/// Statut d'un nœud de stockage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Nœud actif et disponible
    Active,
    /// Nœud en mode maintenance
    Maintenance,
    /// Nœud surchargé (capacité > 90%)
    Overloaded,
    /// Nœud hors ligne
    Offline,
    /// Nœud défaillant
    Failed,
}

impl StorageNodeInfo {
    /// Calcule le pourcentage d'utilisation de la capacité
    pub fn capacity_usage_percent(&self) -> f64 {
        if self.total_capacity == 0 {
            return 0.0;
        }
        (self.used_capacity as f64 / self.total_capacity as f64) * 100.0
    }

    /// Vérifie si le nœud est disponible pour recevoir du contenu
    pub fn is_available_for_storage(&self) -> bool {
        matches!(self.status, NodeStatus::Active) 
            && self.capacity_usage_percent() < 85.0
    }

    /// Calcule un score de performance global
    pub fn performance_score(&self) -> f64 {
        let capacity_factor = 1.0 - (self.capacity_usage_percent() / 100.0);
        let bandwidth_factor = (self.available_bandwidth as f64).min(1_000_000.0) / 1_000_000.0;
        let latency_factor = (1000.0 - self.average_latency as f64).max(0.0) / 1000.0;
        
        (capacity_factor * 0.4 + bandwidth_factor * 0.3 + latency_factor * 0.2 + self.reliability_score * 0.1)
    }
}

/// Interface principale pour le système de stockage distribué
pub trait DistributedStorage {
    /// Stocke du contenu avec réplication automatique
    async fn store_content(
        &mut self,
        content_hash: &Hash,
        data: &[u8],
        metadata: ContentMetadata,
    ) -> Result<StorageResult>;

    /// Récupère du contenu depuis le réseau
    async fn retrieve_content(&self, content_hash: &Hash) -> Result<Vec<u8>>;

    /// Vérifie la disponibilité du contenu
    async fn check_availability(&self, content_hash: &Hash) -> Result<AvailabilityInfo>;

    /// Met à jour la stratégie de réplication
    async fn update_replication_strategy(
        &mut self,
        content_hash: &Hash,
        new_strategy: ReplicationStrategy,
    ) -> Result<()>;

    /// Obtient les statistiques du stockage
    fn get_storage_stats(&self) -> StorageStats;
}

/// Métadonnées de contenu pour le stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMetadata {
    /// Hash du contenu
    pub content_hash: Hash,
    /// Taille du contenu en bytes
    pub size: u64,
    /// Type MIME du contenu
    pub content_type: String,
    /// Titre du contenu
    pub title: Option<String>,
    /// Description du contenu
    pub description: Option<String>,
    /// Importance du contenu
    pub importance: ContentImportance,
    /// Popularité actuelle (accès/jour)
    pub popularity: u64,
    /// Timestamp de création
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Régions préférées pour le stockage
    pub preferred_regions: Vec<String>,
    /// Niveau de redondance souhaité
    pub redundancy_level: u8,
    /// Tags pour la recherche
    pub tags: Vec<String>,
}

/// Résultat d'une opération de stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageResult {
    /// Hash du contenu stocké
    pub content_hash: Hash,
    /// Nœuds où le contenu a été stocké
    pub stored_nodes: Vec<NodeId>,
    /// Nombre de répliques créées
    pub replica_count: u32,
    /// Temps pris pour l'opération
    pub storage_time: Duration,
    /// Taille totale stockée (avec réplication)
    pub total_size_stored: u64,
    /// Statut de l'opération
    pub status: StorageStatus,
}

/// Statut d'une opération de stockage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageStatus {
    /// Stockage réussi avec redondance complète
    Success,
    /// Stockage partiel (moins de répliques que souhaité)
    Partial,
    /// Stockage échoué
    Failed,
    /// En cours de stockage
    Pending,
}

/// Informations sur la disponibilité du contenu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityInfo {
    /// Hash du contenu
    pub content_hash: Hash,
    /// Nombre de répliques disponibles
    pub available_replicas: u32,
    /// Nœuds contenant le contenu
    pub nodes: Vec<NodeId>,
    /// Régions où le contenu est disponible
    pub regions: Vec<String>,
    /// Latence moyenne d'accès
    pub average_latency: Duration,
    /// Score de disponibilité (0.0-1.0)
    pub availability_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_node_info() {
        let node_info = StorageNodeInfo {
            node_id: NodeId::from(Hash::zero()),
            node_type: NodeType::FullArchive,
            region: "eu-west-1".to_string(),
            total_capacity: 1_000_000_000, // 1GB
            used_capacity: 500_000_000,    // 500MB
            supported_storage_types: vec![StorageType::Hot, StorageType::Warm],
            available_bandwidth: 1_000_000, // 1MB/s
            average_latency: 50,
            reliability_score: 0.95,
            last_seen: chrono::Utc::now(),
            status: NodeStatus::Active,
        };

        assert_eq!(node_info.capacity_usage_percent(), 50.0);
        assert!(node_info.is_available_for_storage());
        assert!(node_info.performance_score() > 0.0);
    }

    #[test]
    fn test_storage_types() {
        assert_eq!(NodeType::FullArchive, NodeType::FullArchive);
        assert_ne!(StorageType::Hot, StorageType::Cold);
    }

    #[test]
    fn test_content_metadata() {
        let metadata = ContentMetadata {
            content_hash: Hash::zero(),
            size: 1024,
            content_type: "text/html".to_string(),
            importance: ContentImportance::High,
            popularity: 1500,
            created_at: chrono::Utc::now(),
            preferred_regions: vec!["us-east-1".to_string()],
            redundancy_level: 5,
            tags: vec!["web".to_string(), "archive".to_string()],
        };

        assert_eq!(metadata.size, 1024);
        assert_eq!(metadata.redundancy_level, 5);
    }
}