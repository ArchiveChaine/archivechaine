//! Module de gestion des nœuds distribués pour ArchiveChain
//!
//! Implémente les différents types de nœuds qui forment l'infrastructure distribuée :
//! - **Full Archive Nodes** : stockent des archives complètes (>10TB)
//! - **Light Storage Nodes** : stockage partiel avec spécialisation (1-10TB)  
//! - **Relay Nodes** : facilitent les communications sans stockage massif
//! - **Gateway Nodes** : interfaces publiques pour l'accès web
//!
//! # Architecture
//!
//! Le système de nœuds est conçu pour être :
//! - **Scalable** : support de milliers de nœuds
//! - **Résilient** : récupération automatique des pannes
//! - **Optimisé** : distribution géographique intelligente
//! - **Sécurisé** : authentification et chiffrement des communications
//!
//! # Exemple d'utilisation
//!
//! ```rust,no_run
//! use archivechain_core::nodes::{NodeManager, FullArchiveNode, NodeConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialise un gestionnaire de nœuds
//!     let config = NodeConfig::default();
//!     let mut node_manager = NodeManager::new(config).await?;
//!     
//!     // Démarre un nœud d'archive complet
//!     let archive_node = FullArchiveNode::new(/* config */)?;
//!     node_manager.register_node(archive_node).await?;
//!     
//!     // Lance le monitoring automatique
//!     node_manager.start_monitoring().await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod node_manager;
pub mod node_registry;
pub mod health_monitor;
pub mod full_archive;
pub mod light_storage;
pub mod relay;
pub mod gateway;

// Re-exports publics pour faciliter l'utilisation
pub use node_manager::{NodeManager, NodeConfig, NodeManagerStats};
pub use node_registry::{
    NodeRegistry, NodeRegistryConfig, NodeInfo, NodeCapabilities, 
    NodeStatus, GeographicIndex, ReputationScore
};
pub use health_monitor::{
    HealthMonitor, HealthMonitorConfig, NodeHealth, PerformanceMetrics,
    AlertSystem, AutoRecoverySystem, HealthStatus
};
pub use full_archive::{
    FullArchiveNode, FullArchiveConfig, ArchiveNodeCapabilities,
    FullArchiveMetrics, FullArchiveStatus
};
pub use light_storage::{
    LightStorageNode, LightStorageConfig, StorageSpecialization,
    ContentFilter, LightStorageMetrics, LightStorageStatus
};
pub use relay::{
    RelayNode, RelayNodeConfig, PeerConnection, MessageRouter,
    NetworkMetrics, RelayNodeStatus
};
pub use gateway::{
    GatewayNode, GatewayNodeConfig, ApiEndpoint, LoadBalancer,
    CacheLayer, RateLimiter, SecurityStack, GatewayMetrics
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use crate::crypto::{Hash, PublicKey};
use crate::consensus::NodeId;
use crate::storage::{
    NodeType as StorageNodeType, StorageNodeInfo
};
use crate::error::Result;

/// Trait principal définissant le comportement d'un nœud ArchiveChain
#[async_trait::async_trait]
pub trait Node: Send + Sync {
    /// Type de nœud
    fn node_type(&self) -> NodeType;
    
    /// Identifiant unique du nœud
    fn node_id(&self) -> &NodeId;
    
    /// Démarre le nœud
    async fn start(&mut self) -> Result<()>;
    
    /// Arrête le nœud proprement
    async fn stop(&mut self) -> Result<()>;
    
    /// Vérifie la santé du nœud
    async fn health_check(&self) -> Result<NodeHealth>;
    
    /// Obtient les métriques de performance
    async fn get_metrics(&self) -> Result<Box<dyn NodeMetrics>>;
    
    /// Traite un message réseau
    async fn handle_message(&mut self, message: NetworkMessage) -> Result<Option<NetworkMessage>>;
    
    /// Synchronise avec le réseau
    async fn sync_with_network(&mut self) -> Result<()>;
    
    /// Met à jour la configuration
    async fn update_config(&mut self, config: NodeConfiguration) -> Result<()>;
}

/// Types de nœuds supportés par ArchiveChain
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    /// Nœud d'archive complet (>10TB de stockage)
    FullArchive {
        /// Capacité de stockage en bytes
        storage_capacity: u64,
        /// Facteur de réplication
        replication_factor: u32,
    },
    /// Nœud de stockage léger (1-10TB avec spécialisation)
    LightStorage {
        /// Capacité de stockage en bytes
        storage_capacity: u64,
        /// Spécialisation du nœud
        specialization: StorageSpecialization,
    },
    /// Nœud de relais (facilite les communications)
    Relay {
        /// Capacité de bande passante (bytes/sec)
        bandwidth_capacity: u64,
        /// Nombre maximum de connexions simultanées
        max_connections: u32,
    },
    /// Nœud passerelle (interface publique)
    Gateway {
        /// APIs exposées
        exposed_apis: Vec<ApiType>,
        /// Limite de requêtes par seconde
        rate_limit: u32,
    },
}

impl NodeType {
    /// Retourne les capacités minimales requises pour ce type de nœud
    pub fn minimum_requirements(&self) -> NodeRequirements {
        match self {
            NodeType::FullArchive { .. } => NodeRequirements {
                min_storage: 10_000_000_000_000, // 10TB
                min_bandwidth: 100_000_000,      // 100 MB/s
                min_memory: 8_000_000_000,       // 8GB
                min_cpu_cores: 4,
                consensus_weight: 1.0,
            },
            NodeType::LightStorage { .. } => NodeRequirements {
                min_storage: 1_000_000_000_000,  // 1TB
                min_bandwidth: 50_000_000,       // 50 MB/s
                min_memory: 4_000_000_000,       // 4GB
                min_cpu_cores: 2,
                consensus_weight: 0.5,
            },
            NodeType::Relay { .. } => NodeRequirements {
                min_storage: 100_000_000_000,    // 100GB
                min_bandwidth: 1_000_000_000,    // 1 GB/s
                min_memory: 2_000_000_000,       // 2GB
                min_cpu_cores: 2,
                consensus_weight: 0.3,
            },
            NodeType::Gateway { .. } => NodeRequirements {
                min_storage: 500_000_000_000,    // 500GB
                min_bandwidth: 500_000_000,      // 500 MB/s
                min_memory: 8_000_000_000,       // 8GB
                min_cpu_cores: 4,
                consensus_weight: 0.1,
            },
        }
    }

    /// Vérifie si ce type de nœud peut participer au consensus
    pub fn can_participate_in_consensus(&self) -> bool {
        self.minimum_requirements().consensus_weight > 0.0
    }

    /// Convertit vers le type de stockage correspondant
    pub fn to_storage_node_type(&self) -> StorageNodeType {
        match self {
            NodeType::FullArchive { .. } => StorageNodeType::FullArchive,
            NodeType::LightStorage { .. } => StorageNodeType::LightStorage,
            NodeType::Relay { .. } => StorageNodeType::LightStorage, // Stockage minimal
            NodeType::Gateway { .. } => StorageNodeType::HotStorage, // Cache rapide
        }
    }
}

/// Configuration requise pour un type de nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRequirements {
    /// Stockage minimum en bytes
    pub min_storage: u64,
    /// Bande passante minimum en bytes/sec
    pub min_bandwidth: u64,
    /// Mémoire minimum en bytes
    pub min_memory: u64,
    /// Nombre minimum de cœurs CPU
    pub min_cpu_cores: u32,
    /// Poids dans le consensus (0.0-1.0)
    pub consensus_weight: f64,
}

/// Types d'API exposées par les Gateway Nodes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApiType {
    /// API REST HTTP
    Rest,
    /// API GraphQL
    GraphQL,
    /// WebSocket pour temps réel
    WebSocket,
    /// gRPC pour haute performance
    GRPC,
    /// Interface P2P
    P2P,
}

/// Configuration générale d'un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfiguration {
    /// Identifiant du nœud
    pub node_id: NodeId,
    /// Type et capacités du nœud
    pub node_type: NodeType,
    /// Région géographique
    pub region: String,
    /// Adresse réseau d'écoute
    pub listen_address: String,
    /// Port d'écoute
    pub listen_port: u16,
    /// Nœuds de bootstrap pour la découverte
    pub bootstrap_nodes: Vec<String>,
    /// Configuration du stockage
    pub storage_config: Option<StorageConfiguration>,
    /// Configuration réseau
    pub network_config: NetworkConfiguration,
    /// Configuration de sécurité
    pub security_config: SecurityConfiguration,
}

/// Configuration du stockage pour un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfiguration {
    /// Répertoire de base pour le stockage
    pub data_directory: String,
    /// Capacité maximale en bytes
    pub max_capacity: u64,
    /// Niveau de compression (0-9)
    pub compression_level: u8,
    /// Chiffrement activé
    pub encryption_enabled: bool,
    /// Politique de nettoyage automatique
    pub cleanup_policy: CleanupPolicy,
}

/// Configuration réseau
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfiguration {
    /// Timeout de connexion
    pub connection_timeout: Duration,
    /// Nombre maximum de connexions simultanées
    pub max_connections: u32,
    /// Intervalle de heartbeat
    pub heartbeat_interval: Duration,
    /// Taille du buffer de réception
    pub receive_buffer_size: usize,
    /// Taille du buffer d'envoi
    pub send_buffer_size: usize,
}

/// Configuration de sécurité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfiguration {
    /// Clé privée du nœud
    pub private_key_path: String,
    /// Certificat TLS
    pub tls_cert_path: Option<String>,
    /// Clé TLS
    pub tls_key_path: Option<String>,
    /// Autorités de certification de confiance
    pub trusted_ca_paths: Vec<String>,
    /// Chiffrement des communications requis
    pub require_encryption: bool,
}

/// Politique de nettoyage du stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CleanupPolicy {
    /// Pas de nettoyage automatique
    None,
    /// Nettoyage basé sur l'âge
    Age { max_age: Duration },
    /// Nettoyage basé sur la taille
    Size { max_size: u64 },
    /// Nettoyage basé sur l'utilisation (LRU)
    LeastRecentlyUsed { max_items: u64 },
}

/// Message réseau échangé entre nœuds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    /// Identifiant unique du message
    pub message_id: Hash,
    /// Nœud expéditeur
    pub sender: NodeId,
    /// Nœud destinataire (None = broadcast)
    pub recipient: Option<NodeId>,
    /// Type de message
    pub message_type: MessageType,
    /// Contenu du message
    pub payload: Vec<u8>,
    /// Timestamp de création
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// TTL du message
    pub ttl: u32,
}

/// Types de messages réseau
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// Ping pour vérifier la connectivité
    Ping,
    /// Réponse au ping
    Pong,
    /// Découverte de nouveaux nœuds
    NodeDiscovery,
    /// Annonce d'un nouveau nœud
    NodeAnnouncement,
    /// Demande de synchronisation
    SyncRequest,
    /// Réponse de synchronisation
    SyncResponse,
    /// Défi de consensus
    ConsensusChallenge,
    /// Réponse au défi de consensus
    ConsensusResponse,
    /// Stockage de contenu
    ContentStore,
    /// Récupération de contenu
    ContentRetrieve,
    /// Métadonnées de contenu
    ContentMetadata,
    /// Erreur de traitement
    Error,
}

/// Trait pour les métriques d'un nœud
pub trait NodeMetrics: Send + Sync {
    /// Convertit en représentation sérialisable
    fn as_any(&self) -> &dyn std::any::Any;
    
    /// Métriques générales communes à tous les nœuds
    fn general_metrics(&self) -> GeneralNodeMetrics;
}

/// Métriques générales communes à tous les types de nœuds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralNodeMetrics {
    /// Uptime du nœud
    pub uptime: Duration,
    /// Utilisation CPU (0.0-1.0)
    pub cpu_usage: f64,
    /// Utilisation mémoire (0.0-1.0)
    pub memory_usage: f64,
    /// Utilisation du stockage (0.0-1.0)
    pub storage_usage: f64,
    /// Bande passante entrante (bytes/sec)
    pub bandwidth_in: u64,
    /// Bande passante sortante (bytes/sec)
    pub bandwidth_out: u64,
    /// Nombre de connexions actives
    pub active_connections: u32,
    /// Nombre de messages traités
    pub messages_processed: u64,
    /// Nombre d'erreurs
    pub error_count: u64,
    /// Latence moyenne de traitement
    pub average_latency: Duration,
}

impl Default for NetworkConfiguration {
    fn default() -> Self {
        Self {
            connection_timeout: Duration::from_secs(30),
            max_connections: 1000,
            heartbeat_interval: Duration::from_secs(30),
            receive_buffer_size: 64 * 1024, // 64KB
            send_buffer_size: 64 * 1024,    // 64KB
        }
    }
}

impl Default for SecurityConfiguration {
    fn default() -> Self {
        Self {
            private_key_path: "node.key".to_string(),
            tls_cert_path: None,
            tls_key_path: None,
            trusted_ca_paths: Vec::new(),
            require_encryption: false,
        }
    }
}

impl Default for StorageConfiguration {
    fn default() -> Self {
        Self {
            data_directory: "./data".to_string(),
            max_capacity: 1_000_000_000_000, // 1TB par défaut
            compression_level: 6,
            encryption_enabled: false,
            cleanup_policy: CleanupPolicy::Size { max_size: 900_000_000_000 }, // 90% de la capacité
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_type_requirements() {
        let full_archive = NodeType::FullArchive {
            storage_capacity: 20_000_000_000_000,
            replication_factor: 10,
        };
        
        let requirements = full_archive.minimum_requirements();
        assert_eq!(requirements.min_storage, 10_000_000_000_000);
        assert_eq!(requirements.consensus_weight, 1.0);
        assert!(full_archive.can_participate_in_consensus());
    }

    #[test]
    fn test_node_type_conversion() {
        let light_storage = NodeType::LightStorage {
            storage_capacity: 5_000_000_000_000,
            specialization: StorageSpecialization::Domain,
        };
        
        assert_eq!(light_storage.to_storage_node_type(), StorageNodeType::LightStorage);
    }

    #[test]
    fn test_network_message() {
        let msg = NetworkMessage {
            message_id: Hash::zero(),
            sender: NodeId::from(Hash::zero()),
            recipient: None,
            message_type: MessageType::Ping,
            payload: vec![1, 2, 3],
            timestamp: chrono::Utc::now(),
            ttl: 60,
        };
        
        assert_eq!(msg.message_type, MessageType::Ping);
        assert_eq!(msg.payload, vec![1, 2, 3]);
    }

    #[test]
    fn test_default_configurations() {
        let network_config = NetworkConfiguration::default();
        assert_eq!(network_config.connection_timeout, Duration::from_secs(30));
        assert_eq!(network_config.max_connections, 1000);
        
        let storage_config = StorageConfiguration::default();
        assert_eq!(storage_config.max_capacity, 1_000_000_000_000);
        assert_eq!(storage_config.compression_level, 6);
        
        let security_config = SecurityConfiguration::default();
        assert_eq!(security_config.private_key_path, "node.key");
        assert!(!security_config.require_encryption);
    }
}