//! Implémentation du Full Archive Node
//!
//! Les Full Archive Nodes sont le pilier du réseau ArchiveChain :
//! - Stockent des archives complètes (>10TB)
//! - Maintiennent une redondance élevée (5-15 copies)
//! - Participent activement au consensus PoA
//! - Fournissent un service haute disponibilité
//! - Synchronisation complète de la blockchain

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex};
use async_trait::async_trait;

use crate::crypto::{Hash, PublicKey, PrivateKey};
use crate::consensus::{NodeId, ConsensusScore, ProofOfArchive};
use crate::storage::{
    StorageManager, StorageNodeInfo, ContentMetadata, DistributedStorage,
    StorageType, NodeStatus
};
use crate::blockchain::Blockchain;
use crate::error::Result;
use super::{
    Node, NodeType, NodeConfiguration, NetworkMessage, MessageType,
    NodeHealth, NodeMetrics, GeneralNodeMetrics, HealthStatus
};

/// Configuration spécifique aux Full Archive Nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullArchiveConfig {
    /// Configuration générale du nœud
    pub node_config: NodeConfiguration,
    /// Capacité de stockage minimale (>10TB)
    pub min_storage_capacity: u64,
    /// Capacité de stockage maximale
    pub max_storage_capacity: u64,
    /// Facteur de réplication (5-15)
    pub replication_factor: u32,
    /// Bande passante dédiée (bytes/sec)
    pub dedicated_bandwidth: u64,
    /// Région géographique
    pub geographic_region: String,
    /// Interval de synchronisation blockchain
    pub blockchain_sync_interval: Duration,
    /// Interval de validation des archives
    pub archive_validation_interval: Duration,
    /// Seuil de stockage critique (%)
    pub critical_storage_threshold: f64,
    /// Configuration de sauvegarde
    pub backup_config: BackupConfiguration,
}

/// Configuration de sauvegarde
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfiguration {
    /// Sauvegarde automatique activée
    pub auto_backup_enabled: bool,
    /// Intervalle de sauvegarde
    pub backup_interval: Duration,
    /// Répertoire de sauvegarde
    pub backup_directory: String,
    /// Nombre de sauvegardes à conserver
    pub retention_count: u32,
    /// Compression des sauvegardes
    pub compress_backups: bool,
}

impl Default for FullArchiveConfig {
    fn default() -> Self {
        Self {
            node_config: NodeConfiguration {
                node_id: NodeId::from(Hash::zero()),
                node_type: NodeType::FullArchive {
                    storage_capacity: 20_000_000_000_000, // 20TB
                    replication_factor: 10,
                },
                region: "us-east-1".to_string(),
                listen_address: "0.0.0.0".to_string(),
                listen_port: 8080,
                bootstrap_nodes: Vec::new(),
                storage_config: None,
                network_config: super::NetworkConfiguration::default(),
                security_config: super::SecurityConfiguration::default(),
            },
            min_storage_capacity: 10_000_000_000_000, // 10TB minimum
            max_storage_capacity: u64::MAX,
            replication_factor: 10,
            dedicated_bandwidth: 1_000_000_000, // 1 GB/s
            geographic_region: "us-east-1".to_string(),
            blockchain_sync_interval: Duration::from_secs(30),
            archive_validation_interval: Duration::from_secs(3600), // 1 heure
            critical_storage_threshold: 85.0, // 85%
            backup_config: BackupConfiguration::default(),
        }
    }
}

impl Default for BackupConfiguration {
    fn default() -> Self {
        Self {
            auto_backup_enabled: true,
            backup_interval: Duration::from_secs(86400), // 24 heures
            backup_directory: "./backups".to_string(),
            retention_count: 7, // 7 sauvegardes
            compress_backups: true,
        }
    }
}

/// Capacités spécifiques d'un Full Archive Node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveNodeCapabilities {
    /// Capacité de stockage totale
    pub total_storage_capacity: u64,
    /// Espace utilisé
    pub used_storage: u64,
    /// Archives stockées
    pub archived_content_count: u64,
    /// Facteur de réplication maintenu
    pub maintained_replication_factor: u32,
    /// Support des types de stockage
    pub supported_storage_types: Vec<StorageType>,
    /// Participation au consensus
    pub consensus_participation: bool,
    /// Score de fiabilité
    pub reliability_score: f64,
}

/// Statut d'un Full Archive Node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FullArchiveStatus {
    /// Initialisation en cours
    Initializing,
    /// Synchronisation avec le réseau
    Syncing,
    /// Opérationnel
    Operational,
    /// En maintenance
    Maintenance,
    /// Stockage critique
    StorageCritical,
    /// Panne détectée
    Failed,
    /// Arrêt en cours
    Stopping,
}

/// Métriques spécifiques aux Full Archive Nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullArchiveMetrics {
    /// Métriques générales
    pub general: GeneralNodeMetrics,
    /// Espace de stockage utilisé (bytes)
    pub storage_used: u64,
    /// Espace de stockage disponible (bytes)
    pub storage_available: u64,
    /// Nombre d'archives stockées
    pub archives_count: u64,
    /// Taux de réplication moyen
    pub average_replication_rate: f64,
    /// Débit de synchronisation (bytes/sec)
    pub sync_throughput: u64,
    /// Nombre de validations d'archives
    pub archive_validations: u64,
    /// Nombre d'échecs de validation
    pub validation_failures: u64,
    /// Score de consensus actuel
    pub consensus_score: f64,
    /// Temps de réponse moyen aux requêtes
    pub average_response_time: Duration,
    /// Nombre de sauvegardes réalisées
    pub backups_completed: u64,
}

impl NodeMetrics for FullArchiveMetrics {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn general_metrics(&self) -> GeneralNodeMetrics {
        self.general.clone()
    }
}

/// Full Archive Node - Nœud d'archive complet
pub struct FullArchiveNode {
    /// Configuration du nœud
    config: FullArchiveConfig,
    /// Identifiant du nœud
    node_id: NodeId,
    /// Clés cryptographiques
    keypair: (PublicKey, PrivateKey),
    /// Statut actuel
    status: Arc<RwLock<FullArchiveStatus>>,
    /// Gestionnaire de stockage
    storage_manager: Arc<Mutex<StorageManager>>,
    /// Instance blockchain
    blockchain: Arc<RwLock<Blockchain>>,
    /// Moteur de consensus
    consensus_engine: Arc<Mutex<ProofOfArchive>>,
    /// Archives stockées localement
    archived_content: Arc<RwLock<HashSet<Hash>>>,
    /// Cache des métadonnées d'archives
    metadata_cache: Arc<RwLock<HashMap<Hash, ContentMetadata>>>,
    /// Métriques de performance
    metrics: Arc<RwLock<FullArchiveMetrics>>,
    /// Connexions P2P actives
    peer_connections: Arc<RwLock<HashMap<NodeId, PeerConnectionInfo>>>,
    /// Heure de démarrage
    start_time: SystemTime,
    /// Dernière synchronisation
    last_sync: Arc<Mutex<SystemTime>>,
    /// Dernière sauvegarde
    last_backup: Arc<Mutex<SystemTime>>,
}

/// Informations de connexion P2P
#[derive(Debug, Clone)]
struct PeerConnectionInfo {
    /// Adresse du pair
    address: String,
    /// Timestamp de dernière activité
    last_activity: SystemTime,
    /// Latence mesurée
    latency: Duration,
    /// Statut de la connexion
    status: ConnectionStatus,
}

/// Statut d'une connexion
#[derive(Debug, Clone, PartialEq, Eq)]
enum ConnectionStatus {
    /// Connexion active
    Active,
    /// Connexion en attente
    Pending,
    /// Connexion fermée
    Closed,
    /// Erreur de connexion
    Error,
}

impl FullArchiveNode {
    /// Crée une nouvelle instance de Full Archive Node
    pub fn new(
        config: FullArchiveConfig,
        keypair: (PublicKey, PrivateKey),
        storage_manager: StorageManager,
        blockchain: Blockchain,
        consensus_engine: ProofOfArchive,
    ) -> Result<Self> {
        // Valide la configuration
        config.validate()?;

        let node_id = config.node_config.node_id.clone();
        let start_time = SystemTime::now();

        let initial_metrics = FullArchiveMetrics {
            general: GeneralNodeMetrics {
                uptime: Duration::ZERO,
                cpu_usage: 0.0,
                memory_usage: 0.0,
                storage_usage: 0.0,
                bandwidth_in: 0,
                bandwidth_out: 0,
                active_connections: 0,
                messages_processed: 0,
                error_count: 0,
                average_latency: Duration::ZERO,
            },
            storage_used: 0,
            storage_available: config.max_storage_capacity,
            archives_count: 0,
            average_replication_rate: 0.0,
            sync_throughput: 0,
            archive_validations: 0,
            validation_failures: 0,
            consensus_score: 0.0,
            average_response_time: Duration::ZERO,
            backups_completed: 0,
        };

        Ok(Self {
            config,
            node_id,
            keypair,
            status: Arc::new(RwLock::new(FullArchiveStatus::Initializing)),
            storage_manager: Arc::new(Mutex::new(storage_manager)),
            blockchain: Arc::new(RwLock::new(blockchain)),
            consensus_engine: Arc::new(Mutex::new(consensus_engine)),
            archived_content: Arc::new(RwLock::new(HashSet::new())),
            metadata_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(initial_metrics)),
            peer_connections: Arc::new(RwLock::new(HashMap::new())),
            start_time,
            last_sync: Arc::new(Mutex::new(start_time)),
            last_backup: Arc::new(Mutex::new(start_time)),
        })
    }

    /// Stocke du contenu avec réplication haute
    pub async fn store_archive(
        &mut self,
        content_hash: Hash,
        data: &[u8],
        metadata: ContentMetadata,
    ) -> Result<StorageResult> {
        // Vérifie l'espace disponible
        self.check_storage_capacity(data.len() as u64).await?;

        // Stocke via le gestionnaire de stockage
        let storage_result = {
            let mut storage = self.storage_manager.lock().await;
            storage.store_content(&content_hash, data, metadata.clone()).await?
        };

        // Met à jour le cache local
        {
            let mut archived = self.archived_content.write().await;
            archived.insert(content_hash);
        }

        {
            let mut cache = self.metadata_cache.write().await;
            cache.insert(content_hash, metadata);
        }

        // Met à jour les métriques
        self.update_storage_metrics().await?;

        Ok(storage_result)
    }

    /// Récupère du contenu archivé
    pub async fn retrieve_archive(&self, content_hash: &Hash) -> Result<Vec<u8>> {
        // Vérifie d'abord le cache local
        {
            let archived = self.archived_content.read().await;
            if !archived.contains(content_hash) {
                return Err(crate::error::CoreError::NotFound {
                    message: "Archive non trouvée sur ce nœud".to_string(),
                });
            }
        }

        // Récupère via le gestionnaire de stockage
        let storage = self.storage_manager.lock().await;
        let data = storage.retrieve_content(content_hash).await?;

        // Met à jour les métriques d'accès
        {
            let mut metrics = self.metrics.write().await;
            metrics.general.messages_processed += 1;
        }

        Ok(data)
    }

    /// Valide l'intégrité d'une archive
    pub async fn validate_archive(&self, content_hash: &Hash) -> Result<bool> {
        // Récupère les métadonnées
        let metadata = {
            let cache = self.metadata_cache.read().await;
            cache.get(content_hash).cloned()
        };

        let metadata = match metadata {
            Some(meta) => meta,
            None => return Ok(false),
        };

        // Récupère le contenu
        let data = self.retrieve_archive(content_hash).await?;

        // Valide le hash
        let computed_hash = crate::crypto::compute_hash(&data, crate::crypto::HashAlgorithm::Blake3);
        let is_valid = computed_hash == *content_hash;

        // Met à jour les métriques
        {
            let mut metrics = self.metrics.write().await;
            metrics.archive_validations += 1;
            if !is_valid {
                metrics.validation_failures += 1;
            }
        }

        Ok(is_valid)
    }

    /// Synchronise avec la blockchain
    pub async fn sync_blockchain(&mut self) -> Result<SyncResult> {
        {
            let mut status = self.status.write().await;
            *status = FullArchiveStatus::Syncing;
        }

        let sync_start = SystemTime::now();
        let mut blocks_synced = 0;
        let mut transactions_synced = 0;

        // Obtient la hauteur actuelle
        let current_height = {
            let blockchain = self.blockchain.read().await;
            blockchain.get_current_height()?
        };

        // Synchronise avec les pairs (simulation)
        // Dans la réalité, on interrogerait les autres nœuds
        blocks_synced = 0; // Pas de nouveaux blocs pour cet exemple
        transactions_synced = 0;

        // Met à jour le timestamp de dernière sync
        {
            let mut last_sync = self.last_sync.lock().await;
            *last_sync = SystemTime::now();
        }

        {
            let mut status = self.status.write().await;
            *status = FullArchiveStatus::Operational;
        }

        Ok(SyncResult {
            blocks_synced,
            transactions_synced,
            sync_duration: sync_start.elapsed().unwrap_or(Duration::ZERO),
            final_height: current_height,
        })
    }

    /// Effectue une sauvegarde
    pub async fn perform_backup(&mut self) -> Result<BackupResult> {
        if !self.config.backup_config.auto_backup_enabled {
            return Ok(BackupResult {
                success: false,
                backup_size: 0,
                backup_path: None,
                duration: Duration::ZERO,
            });
        }

        let backup_start = SystemTime::now();
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = format!("{}/backup_{}.tar", 
            self.config.backup_config.backup_directory, timestamp);

        // Simulation de sauvegarde
        // Dans la réalité, on compresserait et sauvegarderait les données
        let backup_size = 1024 * 1024 * 1024; // 1GB simulé

        {
            let mut last_backup = self.last_backup.lock().await;
            *last_backup = SystemTime::now();
        }

        {
            let mut metrics = self.metrics.write().await;
            metrics.backups_completed += 1;
        }

        Ok(BackupResult {
            success: true,
            backup_size,
            backup_path: Some(backup_path),
            duration: backup_start.elapsed().unwrap_or(Duration::ZERO),
        })
    }

    /// Vérifie la capacité de stockage
    async fn check_storage_capacity(&self, required_size: u64) -> Result<()> {
        let metrics = self.metrics.read().await;
        let available = metrics.storage_available;

        if required_size > available {
            return Err(crate::error::CoreError::InsufficientStorage {
                required: required_size,
                available,
            });
        }

        // Vérifie le seuil critique
        let usage_percent = (metrics.storage_used as f64 / 
            (metrics.storage_used + metrics.storage_available) as f64) * 100.0;

        if usage_percent > self.config.critical_storage_threshold {
            log::warn!(
                "Stockage critique atteint: {:.1}% (seuil: {:.1}%)",
                usage_percent,
                self.config.critical_storage_threshold
            );
        }

        Ok(())
    }

    /// Met à jour les métriques de stockage
    async fn update_storage_metrics(&self) -> Result<()> {
        let storage_stats = {
            let storage = self.storage_manager.lock().await;
            storage.get_storage_stats().await?
        };

        let archived_count = {
            let archived = self.archived_content.read().await;
            archived.len() as u64
        };

        {
            let mut metrics = self.metrics.write().await;
            metrics.storage_used = storage_stats.total_size_stored;
            metrics.archives_count = archived_count;
            metrics.average_replication_rate = storage_stats.total_replicas as f64 / archived_count.max(1) as f64;
            metrics.general.storage_usage = storage_stats.average_capacity_usage / 100.0;
        }

        Ok(())
    }

    /// Obtient les capacités du nœud
    pub async fn get_capabilities(&self) -> ArchiveNodeCapabilities {
        let metrics = self.metrics.read().await;
        
        ArchiveNodeCapabilities {
            total_storage_capacity: self.config.max_storage_capacity,
            used_storage: metrics.storage_used,
            archived_content_count: metrics.archives_count,
            maintained_replication_factor: self.config.replication_factor,
            supported_storage_types: vec![StorageType::Hot, StorageType::Warm, StorageType::Cold],
            consensus_participation: true,
            reliability_score: 0.95, // Score par défaut élevé pour Full Archive
        }
    }
}

#[async_trait]
impl Node for FullArchiveNode {
    fn node_type(&self) -> NodeType {
        self.config.node_config.node_type.clone()
    }

    fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    async fn start(&mut self) -> Result<()> {
        log::info!("Démarrage du Full Archive Node: {:?}", self.node_id);

        // Initialise le stockage
        {
            let mut storage = self.storage_manager.lock().await;
            // Configuration du nœud de stockage
            let node_info = StorageNodeInfo {
                node_id: self.node_id.clone(),
                node_type: self.config.node_config.node_type.to_storage_node_type(),
                region: self.config.geographic_region.clone(),
                total_capacity: self.config.max_storage_capacity,
                used_capacity: 0,
                supported_storage_types: vec![StorageType::Hot, StorageType::Warm, StorageType::Cold],
                available_bandwidth: self.config.dedicated_bandwidth,
                average_latency: 50, // ms
                reliability_score: 0.95,
                last_seen: chrono::Utc::now(),
                status: NodeStatus::Active,
            };
            
            storage.update_node_info(self.node_id.clone(), node_info).await?;
        }

        // Synchronise avec la blockchain
        self.sync_blockchain().await?;

        {
            let mut status = self.status.write().await;
            *status = FullArchiveStatus::Operational;
        }

        log::info!("Full Archive Node démarré avec succès");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        log::info!("Arrêt du Full Archive Node: {:?}", self.node_id);

        {
            let mut status = self.status.write().await;
            *status = FullArchiveStatus::Stopping;
        }

        // Effectue une sauvegarde finale
        if self.config.backup_config.auto_backup_enabled {
            self.perform_backup().await?;
        }

        // Ferme les connexions
        {
            let mut connections = self.peer_connections.write().await;
            connections.clear();
        }

        log::info!("Full Archive Node arrêté");
        Ok(())
    }

    async fn health_check(&self) -> Result<NodeHealth> {
        let status = self.status.read().await;
        let metrics = self.metrics.read().await;

        let health_status = match *status {
            FullArchiveStatus::Operational => {
                if metrics.general.storage_usage > 0.9 {
                    HealthStatus::Warning
                } else {
                    HealthStatus::Healthy
                }
            },
            FullArchiveStatus::StorageCritical | FullArchiveStatus::Failed => HealthStatus::Critical,
            _ => HealthStatus::Warning,
        };

        Ok(NodeHealth {
            status: health_status,
            uptime: self.start_time.elapsed().unwrap_or(Duration::ZERO),
            cpu_usage: metrics.general.cpu_usage,
            memory_usage: metrics.general.memory_usage,
            storage_usage: metrics.general.storage_usage,
            network_latency: metrics.average_response_time,
            error_rate: if metrics.general.messages_processed > 0 {
                metrics.general.error_count as f64 / metrics.general.messages_processed as f64
            } else {
                0.0
            },
            last_check: SystemTime::now(),
        })
    }

    async fn get_metrics(&self) -> Result<Box<dyn NodeMetrics>> {
        let metrics = self.metrics.read().await;
        Ok(Box::new(metrics.clone()))
    }

    async fn handle_message(&mut self, message: NetworkMessage) -> Result<Option<NetworkMessage>> {
        {
            let mut metrics = self.metrics.write().await;
            metrics.general.messages_processed += 1;
        }

        match message.message_type {
            MessageType::Ping => {
                // Répond avec un Pong
                Ok(Some(NetworkMessage {
                    message_id: crate::crypto::compute_hash(
                        &message.message_id.as_bytes(),
                        crate::crypto::HashAlgorithm::Blake3
                    ),
                    sender: self.node_id.clone(),
                    recipient: Some(message.sender),
                    message_type: MessageType::Pong,
                    payload: Vec::new(),
                    timestamp: chrono::Utc::now(),
                    ttl: 60,
                }))
            },
            MessageType::ContentStore => {
                // Traite une demande de stockage
                // Dans la réalité, on déserialiserait le payload
                Ok(None)
            },
            MessageType::ContentRetrieve => {
                // Traite une demande de récupération
                // Dans la réalité, on récupérerait et retournerait le contenu
                Ok(None)
            },
            _ => {
                // Message non géré
                Ok(None)
            }
        }
    }

    async fn sync_with_network(&mut self) -> Result<()> {
        self.sync_blockchain().await?;
        Ok(())
    }

    async fn update_config(&mut self, config: super::NodeConfiguration) -> Result<()> {
        self.config.node_config = config;
        Ok(())
    }
}

/// Résultat d'une opération de stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageResult {
    /// Hash du contenu stocké
    pub content_hash: Hash,
    /// Nombre de répliques créées
    pub replicas_created: u32,
    /// Nœuds où le contenu est stocké
    pub storage_nodes: Vec<NodeId>,
    /// Temps de stockage
    pub storage_duration: Duration,
    /// Succès de l'opération
    pub success: bool,
}

/// Résultat d'une synchronisation blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Nombre de blocs synchronisés
    pub blocks_synced: u64,
    /// Nombre de transactions synchronisées
    pub transactions_synced: u64,
    /// Durée de la synchronisation
    pub sync_duration: Duration,
    /// Hauteur finale de la blockchain
    pub final_height: u64,
}

/// Résultat d'une sauvegarde
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
    /// Succès de la sauvegarde
    pub success: bool,
    /// Taille de la sauvegarde (bytes)
    pub backup_size: u64,
    /// Chemin de la sauvegarde
    pub backup_path: Option<String>,
    /// Durée de la sauvegarde
    pub duration: Duration,
}

impl FullArchiveConfig {
    /// Valide la configuration
    pub fn validate(&self) -> Result<()> {
        if self.min_storage_capacity < 10_000_000_000_000 {
            return Err(crate::error::CoreError::Validation {
                message: "Capacité minimale pour Full Archive Node: 10TB".to_string(),
            });
        }

        if self.replication_factor < 5 || self.replication_factor > 15 {
            return Err(crate::error::CoreError::Validation {
                message: "Facteur de réplication doit être entre 5 et 15".to_string(),
            });
        }

        if self.critical_storage_threshold < 50.0 || self.critical_storage_threshold > 95.0 {
            return Err(crate::error::CoreError::Validation {
                message: "Seuil critique doit être entre 50% et 95%".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::generate_keypair;
    use crate::storage::StorageConfig;
    use crate::consensus::ConsensusConfig;
    use crate::blockchain::BlockchainConfig;

    #[test]
    fn test_full_archive_config_validation() {
        let mut config = FullArchiveConfig::default();
        assert!(config.validate().is_ok());

        // Test capacité insuffisante
        config.min_storage_capacity = 1_000_000_000; // 1GB
        assert!(config.validate().is_err());

        // Test facteur de réplication invalide
        config.min_storage_capacity = 15_000_000_000_000; // 15TB
        config.replication_factor = 20; // Trop élevé
        assert!(config.validate().is_err());

        config.replication_factor = 8; // Valide
        assert!(config.validate().is_ok());
    }

    #[tokio::test]
    async fn test_full_archive_node_creation() {
        let config = FullArchiveConfig::default();
        let keypair = generate_keypair().unwrap();
        
        let storage_config = StorageConfig::default();
        let storage_manager = StorageManager::new(
            storage_config,
            crate::storage::manager::StoragePolicy {
                default_replication_strategy: crate::storage::replication::ReplicationStrategy::Fixed { 
                    replica_count: 3 
                },
                node_preferences: HashMap::new(),
                retention_policies: Vec::new(),
                alert_thresholds: crate::storage::manager::AlertThresholds::default(),
            }
        ).await.unwrap();

        let blockchain_config = BlockchainConfig::default();
        let blockchain = crate::blockchain::Blockchain::new(blockchain_config).unwrap();

        let consensus_config = ConsensusConfig::default();
        let consensus_engine = ProofOfArchive::new(consensus_config).unwrap();

        let node = FullArchiveNode::new(
            config,
            keypair,
            storage_manager,
            blockchain,
            consensus_engine,
        );

        assert!(node.is_ok());
    }

    #[test]
    fn test_archive_node_capabilities() {
        let capabilities = ArchiveNodeCapabilities {
            total_storage_capacity: 20_000_000_000_000,
            used_storage: 5_000_000_000_000,
            archived_content_count: 1000,
            maintained_replication_factor: 10,
            supported_storage_types: vec![StorageType::Hot, StorageType::Warm, StorageType::Cold],
            consensus_participation: true,
            reliability_score: 0.95,
        };

        assert_eq!(capabilities.total_storage_capacity, 20_000_000_000_000);
        assert_eq!(capabilities.maintained_replication_factor, 10);
        assert!(capabilities.consensus_participation);
    }
}