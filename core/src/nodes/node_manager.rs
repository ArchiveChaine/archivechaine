//! Gestionnaire central des nœuds pour ArchiveChain
//!
//! Le Node Manager orchestre tous les aspects de la gestion des nœuds :
//! - Orchestration des différents types de nœuds (Full Archive, Light Storage, Relay, Gateway)
//! - Gestion du cycle de vie des nœuds (création, démarrage, arrêt, redémarrage)
//! - Coordination avec les autres composants (consensus, storage, blockchain)
//! - Basculement automatique en cas de panne (failover)
//! - Monitoring et optimisation continue

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex};
use async_trait::async_trait;

use crate::crypto::{Hash, PublicKey, PrivateKey, generate_keypair};
use crate::consensus::{NodeId, ProofOfArchive, ConsensusConfig};
use crate::storage::{
    StorageManager, StorageConfig, StoragePolicy, ReplicationStrategy,
    AlertThresholds
};
use crate::blockchain::{Blockchain, BlockchainConfig};
use crate::error::Result;
use super::{
    Node, NodeType, NodeConfiguration, NetworkMessage,
    FullArchiveNode, FullArchiveConfig,
    LightStorageNode, LightStorageConfig,
    RelayNode, RelayNodeConfig,
    GatewayNode, GatewayNodeConfig,
    NodeHealth, HealthStatus,
    health_monitor::{HealthMonitor, HealthMonitorConfig},
    node_registry::{NodeRegistry, NodeRegistryConfig, NodeInfo},
};

/// Configuration du Node Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Configuration des nœuds Full Archive
    pub full_archive_config: FullArchiveConfig,
    /// Configuration des nœuds Light Storage
    pub light_storage_config: LightStorageConfig,
    /// Configuration des nœuds Relay
    pub relay_config: RelayNodeConfig,
    /// Configuration des nœuds Gateway
    pub gateway_config: GatewayNodeConfig,
    /// Configuration du consensus
    pub consensus_config: ConsensusConfig,
    /// Configuration du stockage
    pub storage_config: StorageConfig,
    /// Configuration de la blockchain
    pub blockchain_config: BlockchainConfig,
    /// Configuration du monitoring
    pub health_monitor_config: HealthMonitorConfig,
    /// Configuration du registre
    pub registry_config: NodeRegistryConfig,
    /// Configuration du clustering
    pub cluster_config: ClusterConfig,
}

/// Configuration du clustering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Nom du cluster
    pub cluster_name: String,
    /// Nœuds de bootstrap
    pub bootstrap_nodes: Vec<String>,
    /// Facteur de réplication par défaut
    pub default_replication_factor: u32,
    /// Stratégie de basculement
    pub failover_strategy: FailoverStrategy,
    /// Configuration de l'auto-scaling
    pub auto_scaling: AutoScalingConfig,
    /// Régions géographiques
    pub geographic_regions: Vec<String>,
}

/// Stratégies de basculement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailoverStrategy {
    /// Basculement automatique immédiat
    Automatic,
    /// Basculement manuel
    Manual,
    /// Basculement graduel
    Gradual,
    /// Pas de basculement
    None,
}

/// Configuration de l'auto-scaling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScalingConfig {
    /// Auto-scaling activé
    pub enabled: bool,
    /// Seuil de charge pour scale-up (%)
    pub scale_up_threshold: f64,
    /// Seuil de charge pour scale-down (%)
    pub scale_down_threshold: f64,
    /// Délai minimum entre les ajustements
    pub cooldown_period: Duration,
    /// Nombre minimum de nœuds
    pub min_nodes: u32,
    /// Nombre maximum de nœuds
    pub max_nodes: u32,
}

/// Statistiques du Node Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeManagerStats {
    /// Nombre de nœuds par type
    pub nodes_per_type: HashMap<String, u32>,
    /// Nœuds actifs
    pub active_nodes: u32,
    /// Nœuds en maintenance
    pub maintenance_nodes: u32,
    /// Nœuds défaillants
    pub failed_nodes: u32,
    /// Temps de fonctionnement du cluster
    pub cluster_uptime: Duration,
    /// Utilisation globale des ressources
    pub resource_utilization: ResourceUtilization,
    /// Événements récents
    pub recent_events: Vec<NodeEvent>,
}

/// Utilisation des ressources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    /// CPU moyen (%)
    pub average_cpu: f64,
    /// Mémoire moyenne (%)
    pub average_memory: f64,
    /// Stockage moyen (%)
    pub average_storage: f64,
    /// Bande passante moyenne (bytes/sec)
    pub average_bandwidth: u64,
    /// Latence moyenne du réseau
    pub average_network_latency: Duration,
}

/// Événement de nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEvent {
    /// Timestamp de l'événement
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Nœud concerné
    pub node_id: NodeId,
    /// Type d'événement
    pub event_type: NodeEventType,
    /// Message descriptif
    pub message: String,
    /// Sévérité
    pub severity: EventSeverity,
}

/// Types d'événements de nœud
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeEventType {
    /// Nœud démarré
    NodeStarted,
    /// Nœud arrêté
    NodeStopped,
    /// Nœud redémarré
    NodeRestarted,
    /// Nœud en échec
    NodeFailed,
    /// Nœud récupéré
    NodeRecovered,
    /// Maintenance programmée
    MaintenanceStarted,
    /// Fin de maintenance
    MaintenanceCompleted,
    /// Mise à jour de configuration
    ConfigurationUpdated,
    /// Alerte de performance
    PerformanceAlert,
    /// Problème de connectivité
    ConnectivityIssue,
}

/// Sévérité des événements
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventSeverity {
    /// Information
    Info,
    /// Avertissement
    Warning,
    /// Erreur
    Error,
    /// Critique
    Critical,
}

/// Gestionnaire central des nœuds
pub struct NodeManager {
    /// Configuration
    config: NodeConfig,
    /// Nœuds gérés
    managed_nodes: Arc<RwLock<HashMap<NodeId, Box<dyn Node + Send + Sync>>>>,
    /// Registre des nœuds
    node_registry: Arc<Mutex<NodeRegistry>>,
    /// Moniteur de santé
    health_monitor: Arc<Mutex<HealthMonitor>>,
    /// Moteur de consensus
    consensus_engine: Arc<Mutex<ProofOfArchive>>,
    /// Gestionnaire de stockage
    storage_manager: Arc<Mutex<StorageManager>>,
    /// Blockchain
    blockchain: Arc<RwLock<Blockchain>>,
    /// Événements récents
    recent_events: Arc<RwLock<Vec<NodeEvent>>>,
    /// Statistiques
    stats: Arc<RwLock<NodeManagerStats>>,
    /// Heure de démarrage du cluster
    cluster_start_time: SystemTime,
    /// Tâches de maintenance en cours
    maintenance_tasks: Arc<Mutex<HashMap<NodeId, MaintenanceTask>>>,
}

/// Tâche de maintenance
#[derive(Debug, Clone)]
pub struct MaintenanceTask {
    /// Nœud concerné
    pub node_id: NodeId,
    /// Type de maintenance
    pub task_type: MaintenanceType,
    /// Heure de début
    pub started_at: SystemTime,
    /// Durée estimée
    pub estimated_duration: Duration,
    /// Statut
    pub status: MaintenanceStatus,
}

/// Types de maintenance
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaintenanceType {
    /// Redémarrage
    Restart,
    /// Mise à jour de configuration
    ConfigUpdate,
    /// Optimisation
    Optimization,
    /// Nettoyage
    Cleanup,
    /// Synchronisation
    Synchronization,
}

/// Statut de maintenance
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaintenanceStatus {
    /// En cours
    InProgress,
    /// Terminé avec succès
    Completed,
    /// Échoué
    Failed,
    /// Annulé
    Cancelled,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            full_archive_config: FullArchiveConfig::default(),
            light_storage_config: LightStorageConfig::default(),
            relay_config: RelayNodeConfig::default(),
            gateway_config: GatewayNodeConfig::default(),
            consensus_config: ConsensusConfig::default(),
            storage_config: StorageConfig::default(),
            blockchain_config: BlockchainConfig::default(),
            health_monitor_config: HealthMonitorConfig::default(),
            registry_config: NodeRegistryConfig::default(),
            cluster_config: ClusterConfig::default(),
        }
    }
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            cluster_name: "archivechain-cluster".to_string(),
            bootstrap_nodes: Vec::new(),
            default_replication_factor: 5,
            failover_strategy: FailoverStrategy::Automatic,
            auto_scaling: AutoScalingConfig::default(),
            geographic_regions: vec!["us-east-1".to_string(), "eu-west-1".to_string()],
        }
    }
}

impl Default for AutoScalingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            scale_up_threshold: 80.0, // 80%
            scale_down_threshold: 30.0, // 30%
            cooldown_period: Duration::from_secs(300), // 5 minutes
            min_nodes: 3,
            max_nodes: 100,
        }
    }
}

impl NodeManager {
    /// Crée une nouvelle instance du Node Manager
    pub async fn new(config: NodeConfig) -> Result<Self> {
        // Valide la configuration
        config.validate()?;

        let cluster_start_time = SystemTime::now();

        // Initialise la blockchain
        let blockchain = Blockchain::new(config.blockchain_config.clone())?;

        // Initialise le moteur de consensus
        let consensus_engine = ProofOfArchive::new(config.consensus_config.clone())?;

        // Initialise le gestionnaire de stockage
        let storage_manager = StorageManager::new(
            config.storage_config.clone(),
            StoragePolicy {
                default_replication_strategy: ReplicationStrategy::fixed(
                    config.cluster_config.default_replication_factor,
                ),
                node_preferences: HashMap::new(),
                retention_policies: Vec::new(),
                alert_thresholds: AlertThresholds::default(),
            },
        ).await?;

        // Initialise le registre des nœuds
        let node_registry = NodeRegistry::new(config.registry_config.clone()).await?;

        // Initialise le moniteur de santé
        let health_monitor = HealthMonitor::new(config.health_monitor_config.clone()).await?;

        let initial_stats = NodeManagerStats {
            nodes_per_type: HashMap::new(),
            active_nodes: 0,
            maintenance_nodes: 0,
            failed_nodes: 0,
            cluster_uptime: Duration::ZERO,
            resource_utilization: ResourceUtilization {
                average_cpu: 0.0,
                average_memory: 0.0,
                average_storage: 0.0,
                average_bandwidth: 0,
                average_network_latency: Duration::ZERO,
            },
            recent_events: Vec::new(),
        };

        Ok(Self {
            config,
            managed_nodes: Arc::new(RwLock::new(HashMap::new())),
            node_registry: Arc::new(Mutex::new(node_registry)),
            health_monitor: Arc::new(Mutex::new(health_monitor)),
            consensus_engine: Arc::new(Mutex::new(consensus_engine)),
            storage_manager: Arc::new(Mutex::new(storage_manager)),
            blockchain: Arc::new(RwLock::new(blockchain)),
            recent_events: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(initial_stats)),
            cluster_start_time,
            maintenance_tasks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Crée et enregistre un nouveau nœud
    pub async fn create_node(&self, node_type: NodeType, custom_config: Option<NodeConfiguration>) -> Result<NodeId> {
        let keypair = generate_keypair()?;
        let node_id = NodeId::from_public_key(keypair.public_key());

        // Crée le nœud selon son type
        let node: Box<dyn Node + Send + Sync> = match node_type {
            NodeType::FullArchive { .. } => {
                let mut config = self.config.full_archive_config.clone();
                if let Some(custom) = custom_config {
                    config.node_config = custom;
                } else {
                    config.node_config.node_id = node_id.clone();
                    config.node_config.node_type = node_type.clone();
                }

                let storage_manager = {
                    let storage = self.storage_manager.lock().await;
                    // Créer une copie du storage manager pour le nœud
                    // Dans une vraie implémentation, on partagerait ou créerait une instance séparée
                    StorageManager::new(
                        self.config.storage_config.clone(),
                        StoragePolicy {
                            default_replication_strategy: ReplicationStrategy::fixed(
                                self.config.cluster_config.default_replication_factor,
                            ),
                            node_preferences: HashMap::new(),
                            retention_policies: Vec::new(),
                            alert_thresholds: AlertThresholds::default(),
                        },
                    ).await?
                };

                let blockchain = {
                    let bc = self.blockchain.read().await;
                    // Clone la blockchain - dans la réalité, on partagerait l'instance
                    Blockchain::new(self.config.blockchain_config.clone())?
                };

                let consensus_engine = ProofOfArchive::new(self.config.consensus_config.clone())?;

                Box::new(FullArchiveNode::new(
                    config,
                    keypair,
                    storage_manager,
                    blockchain,
                    consensus_engine,
                )?)
            },
            NodeType::LightStorage { .. } => {
                let mut config = self.config.light_storage_config.clone();
                if let Some(custom) = custom_config {
                    config.node_config = custom;
                } else {
                    config.node_config.node_id = node_id.clone();
                    config.node_config.node_type = node_type.clone();
                }

                let storage_manager = StorageManager::new(
                    self.config.storage_config.clone(),
                    StoragePolicy {
                        default_replication_strategy: ReplicationStrategy::fixed(
                            self.config.cluster_config.default_replication_factor,
                        ),
                        node_preferences: HashMap::new(),
                        retention_policies: Vec::new(),
                        alert_thresholds: AlertThresholds::default(),
                    },
                ).await?;

                Box::new(LightStorageNode::new(config, keypair, storage_manager)?)
            },
            NodeType::Relay { .. } => {
                let mut config = self.config.relay_config.clone();
                if let Some(custom) = custom_config {
                    config.node_config = custom;
                } else {
                    config.node_config.node_id = node_id.clone();
                    config.node_config.node_type = node_type.clone();
                }

                Box::new(RelayNode::new(config, keypair)?)
            },
            NodeType::Gateway { .. } => {
                let mut config = self.config.gateway_config.clone();
                if let Some(custom) = custom_config {
                    config.node_config = custom;
                } else {
                    config.node_config.node_id = node_id.clone();
                    config.node_config.node_type = node_type.clone();
                }

                Box::new(GatewayNode::new(config, keypair)?)
            },
        };

        // Enregistre le nœud
        {
            let mut nodes = self.managed_nodes.write().await;
            nodes.insert(node_id.clone(), node);
        }

        // Enregistre dans le registre
        {
            let mut registry = self.node_registry.lock().await;
            registry.register_node(NodeInfo {
                node_id: node_id.clone(),
                node_type: node_type.clone(),
                address: "127.0.0.1:8080".to_string(), // Exemple
                region: "us-east-1".to_string(),
                capabilities: super::node_registry::NodeCapabilities {
                    storage_capacity: match node_type {
                        NodeType::FullArchive { storage_capacity, .. } => storage_capacity,
                        NodeType::LightStorage { storage_capacity, .. } => storage_capacity,
                        _ => 0,
                    },
                    bandwidth_capacity: 1_000_000_000, // 1GB/s par défaut
                    consensus_weight: node_type.minimum_requirements().consensus_weight,
                    api_endpoints: Vec::new(),
                },
                status: super::node_registry::NodeStatus::Active,
                registered_at: chrono::Utc::now(),
                last_heartbeat: chrono::Utc::now(),
                performance_metrics: super::node_registry::PerformanceMetrics {
                    cpu_usage: 0.0,
                    memory_usage: 0.0,
                    storage_usage: 0.0,
                    network_latency: Duration::ZERO,
                    uptime: Duration::ZERO,
                },
            }).await?;
        }

        // Enregistre l'événement
        self.log_event(NodeEvent {
            timestamp: chrono::Utc::now(),
            node_id: node_id.clone(),
            event_type: NodeEventType::NodeStarted,
            message: format!("Nœud {:?} créé", node_type),
            severity: EventSeverity::Info,
        }).await;

        // Met à jour les statistiques
        self.update_stats().await?;

        Ok(node_id)
    }

    /// Démarre un nœud
    pub async fn start_node(&self, node_id: &NodeId) -> Result<()> {
        let mut nodes = self.managed_nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.start().await?;

            // Enregistre l'événement
            self.log_event(NodeEvent {
                timestamp: chrono::Utc::now(),
                node_id: node_id.clone(),
                event_type: NodeEventType::NodeStarted,
                message: "Nœud démarré avec succès".to_string(),
                severity: EventSeverity::Info,
            }).await;

            // Met à jour les statistiques
            self.update_stats().await?;

            Ok(())
        } else {
            Err(crate::error::CoreError::NotFound {
                message: format!("Nœud {:?} non trouvé", node_id),
            })
        }
    }

    /// Arrête un nœud
    pub async fn stop_node(&self, node_id: &NodeId) -> Result<()> {
        let mut nodes = self.managed_nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.stop().await?;

            // Enregistre l'événement
            self.log_event(NodeEvent {
                timestamp: chrono::Utc::now(),
                node_id: node_id.clone(),
                event_type: NodeEventType::NodeStopped,
                message: "Nœud arrêté".to_string(),
                severity: EventSeverity::Info,
            }).await;

            // Met à jour les statistiques
            self.update_stats().await?;

            Ok(())
        } else {
            Err(crate::error::CoreError::NotFound {
                message: format!("Nœud {:?} non trouvé", node_id),
            })
        }
    }

    /// Redémarre un nœud
    pub async fn restart_node(&self, node_id: &NodeId) -> Result<()> {
        tracing::info!("Redémarrage du nœud {:?}", node_id);

        // Démarre une tâche de maintenance
        {
            let mut tasks = self.maintenance_tasks.lock().await;
            tasks.insert(node_id.clone(), MaintenanceTask {
                node_id: node_id.clone(),
                task_type: MaintenanceType::Restart,
                started_at: SystemTime::now(),
                estimated_duration: Duration::from_secs(30),
                status: MaintenanceStatus::InProgress,
            });
        }

        // Arrête puis redémarre
        self.stop_node(node_id).await?;
        tokio::time::sleep(Duration::from_secs(1)).await; // Délai pour arrêt propre
        self.start_node(node_id).await?;

        // Termine la tâche de maintenance
        {
            let mut tasks = self.maintenance_tasks.lock().await;
            if let Some(task) = tasks.get_mut(node_id) {
                task.status = MaintenanceStatus::Completed;
            }
        }

        // Enregistre l'événement
        self.log_event(NodeEvent {
            timestamp: chrono::Utc::now(),
            node_id: node_id.clone(),
            event_type: NodeEventType::NodeRestarted,
            message: "Nœud redémarré avec succès".to_string(),
            severity: EventSeverity::Info,
        }).await;

        Ok(())
    }

    /// Effectue un health check sur tous les nœuds
    pub async fn health_check_all_nodes(&self) -> Result<HashMap<NodeId, NodeHealth>> {
        let mut health_results = HashMap::new();
        let nodes = self.managed_nodes.read().await;

        for (node_id, node) in nodes.iter() {
            match node.health_check().await {
                Ok(health) => {
                    health_results.insert(node_id.clone(), health);
                },
                Err(e) => {
                    tracing::error!("Erreur health check nœud {:?}: {}", node_id, e);
                    
                    // Enregistre l'événement d'erreur
                    self.log_event(NodeEvent {
                        timestamp: chrono::Utc::now(),
                        node_id: node_id.clone(),
                        event_type: NodeEventType::NodeFailed,
                        message: format!("Échec health check: {}", e),
                        severity: EventSeverity::Error,
                    }).await;
                }
            }
        }

        Ok(health_results)
    }

    /// Gère le basculement automatique
    pub async fn handle_node_failure(&self, failed_node_id: &NodeId) -> Result<()> {
        if self.config.cluster_config.failover_strategy != FailoverStrategy::Automatic {
            tracing::info!("Basculement automatique désactivé pour le nœud {:?}", failed_node_id);
            return Ok(());
        }

        tracing::warn!("Gestion de la panne du nœud {:?}", failed_node_id);

        // Tente de redémarrer le nœud
        match self.restart_node(failed_node_id).await {
            Ok(()) => {
                tracing::info!("Nœud {:?} redémarré avec succès", failed_node_id);
                
                self.log_event(NodeEvent {
                    timestamp: chrono::Utc::now(),
                    node_id: failed_node_id.clone(),
                    event_type: NodeEventType::NodeRecovered,
                    message: "Nœud récupéré après redémarrage".to_string(),
                    severity: EventSeverity::Info,
                }).await;
            },
            Err(e) => {
                tracing::error!("Échec du redémarrage du nœud {:?}: {}", failed_node_id, e);
                
                // Si le redémarrage échoue, créer un nouveau nœud de remplacement
                self.create_replacement_node(failed_node_id).await?;
            }
        }

        Ok(())
    }

    /// Crée un nœud de remplacement
    async fn create_replacement_node(&self, failed_node_id: &NodeId) -> Result<NodeId> {
        // Récupère le type du nœud défaillant
        let node_type = {
            let registry = self.node_registry.lock().await;
            registry.get_node_info(failed_node_id).await?
                .map(|info| info.node_type.clone())
                .ok_or_else(|| crate::error::CoreError::NotFound {
                    message: format!("Informations du nœud {:?} non trouvées", failed_node_id),
                })?
        };

        // Convertit vers le bon type de nœud
        let replacement_node_type = match node_type {
            super::node_registry::NodeType::FullArchive => NodeType::FullArchive {
                storage_capacity: 20_000_000_000_000, // 20TB par défaut
                replication_factor: 10,
            },
            super::node_registry::NodeType::LightStorage => NodeType::LightStorage {
                storage_capacity: 5_000_000_000_000, // 5TB par défaut
                specialization: super::light_storage::StorageSpecialization::ContentType,
            },
            super::node_registry::NodeType::Relay => NodeType::Relay {
                bandwidth_capacity: 1_000_000_000, // 1GB/s
                max_connections: 1000,
            },
            super::node_registry::NodeType::Gateway => NodeType::Gateway {
                exposed_apis: vec![super::ApiType::Rest, super::ApiType::WebSocket],
                rate_limit: 1000,
            },
        };

        // Crée le nœud de remplacement
        let replacement_id = self.create_node(replacement_node_type, None).await?;
        
        // Démarre le nouveau nœud
        self.start_node(&replacement_id).await?;

        tracing::info!("Nœud de remplacement {:?} créé pour remplacer {:?}", replacement_id, failed_node_id);

        self.log_event(NodeEvent {
            timestamp: chrono::Utc::now(),
            node_id: replacement_id.clone(),
            event_type: NodeEventType::NodeStarted,
            message: format!("Nœud de remplacement créé pour {:?}", failed_node_id),
            severity: EventSeverity::Warning,
        }).await;

        Ok(replacement_id)
    }

    /// Met à jour les statistiques du cluster
    async fn update_stats(&self) -> Result<()> {
        let nodes = self.managed_nodes.read().await;
        let mut stats = self.stats.write().await;

        // Compte les nœuds par type
        let mut nodes_per_type = HashMap::new();
        let mut active_nodes = 0;
        let mut maintenance_nodes = 0;
        let mut failed_nodes = 0;

        for (node_id, node) in nodes.iter() {
            let node_type = format!("{:?}", node.node_type());
            *nodes_per_type.entry(node_type).or_insert(0) += 1;

            // Vérifie l'état de santé pour compter les statuts
            match node.health_check().await {
                Ok(health) => match health.status {
                    HealthStatus::Healthy => active_nodes += 1,
                    HealthStatus::Warning => active_nodes += 1, // Considéré comme actif
                    HealthStatus::Critical => failed_nodes += 1,
                    HealthStatus::Unresponsive => failed_nodes += 1,
                    HealthStatus::Recovering => active_nodes += 1, // En cours de récupération mais actif
                },
                Err(_) => failed_nodes += 1,
            }
        }

        // Compte les tâches de maintenance
        {
            let tasks = self.maintenance_tasks.lock().await;
            maintenance_nodes = tasks.values()
                .filter(|task| task.status == MaintenanceStatus::InProgress)
                .count() as u32;
        }

        stats.nodes_per_type = nodes_per_type;
        stats.active_nodes = active_nodes;
        stats.maintenance_nodes = maintenance_nodes;
        stats.failed_nodes = failed_nodes;
        stats.cluster_uptime = self.cluster_start_time.elapsed().unwrap_or(Duration::ZERO);

        // Met à jour les événements récents
        let events = self.recent_events.read().await;
        stats.recent_events = events.iter()
            .rev()
            .take(10) // Garde les 10 derniers événements
            .cloned()
            .collect();

        Ok(())
    }

    /// Enregistre un événement
    async fn log_event(&self, event: NodeEvent) {
        tracing::info!("Événement nœud: {:?} - {}", event.event_type, event.message);
        
        let mut events = self.recent_events.write().await;
        events.push(event);

        // Garde seulement les 100 derniers événements
        if events.len() > 100 {
            events.remove(0);
        }
    }

    /// Obtient les statistiques du cluster
    pub async fn get_cluster_stats(&self) -> NodeManagerStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Obtient les nœuds gérés
    pub async fn get_managed_nodes(&self) -> Vec<NodeId> {
        let nodes = self.managed_nodes.read().await;
        nodes.keys().cloned().collect()
    }

    /// Démarre tous les nœuds
    pub async fn start_all_nodes(&self) -> Result<()> {
        let node_ids: Vec<NodeId> = {
            let nodes = self.managed_nodes.read().await;
            nodes.keys().cloned().collect()
        };

        for node_id in node_ids {
            if let Err(e) = self.start_node(&node_id).await {
                tracing::error!("Erreur démarrage nœud {:?}: {}", node_id, e);
            }
        }

        Ok(())
    }

    /// Arrête tous les nœuds
    pub async fn stop_all_nodes(&self) -> Result<()> {
        let node_ids: Vec<NodeId> = {
            let nodes = self.managed_nodes.read().await;
            nodes.keys().cloned().collect()
        };

        for node_id in node_ids {
            if let Err(e) = self.stop_node(&node_id).await {
                tracing::error!("Erreur arrêt nœud {:?}: {}", node_id, e);
            }
        }

        Ok(())
    }
}

impl NodeConfig {
    /// Valide la configuration
    pub fn validate(&self) -> Result<()> {
        // Valide les configurations individuelles
        self.consensus_config.validate()?;
        
        // Valide la configuration du cluster
        if self.cluster_config.cluster_name.is_empty() {
            return Err(crate::error::CoreError::Validation {
                message: "Le nom du cluster ne peut pas être vide".to_string(),
            });
        }

        if self.cluster_config.default_replication_factor < 3 {
            return Err(crate::error::CoreError::Validation {
                message: "Le facteur de réplication doit être au minimum 3".to_string(),
            });
        }

        // Valide l'auto-scaling
        let auto_scaling = &self.cluster_config.auto_scaling;
        if auto_scaling.enabled {
            if auto_scaling.scale_up_threshold <= auto_scaling.scale_down_threshold {
                return Err(crate::error::CoreError::Validation {
                    message: "Seuil de scale-up doit être supérieur au seuil de scale-down".to_string(),
                });
            }

            if auto_scaling.min_nodes >= auto_scaling.max_nodes {
                return Err(crate::error::CoreError::Validation {
                    message: "min_nodes doit être inférieur à max_nodes".to_string(),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_config_validation() {
        let mut config = NodeConfig::default();
        assert!(config.validate().is_ok());

        // Test nom de cluster vide
        config.cluster_config.cluster_name.clear();
        assert!(config.validate().is_err());

        // Test facteur de réplication trop faible
        config.cluster_config.cluster_name = "test".to_string();
        config.cluster_config.default_replication_factor = 2;
        assert!(config.validate().is_err());

        // Test auto-scaling mal configuré
        config.cluster_config.default_replication_factor = 5;
        config.cluster_config.auto_scaling.enabled = true;
        config.cluster_config.auto_scaling.scale_up_threshold = 50.0;
        config.cluster_config.auto_scaling.scale_down_threshold = 60.0; // Inversé
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_node_manager_creation() {
        let config = NodeConfig::default();
        let node_manager = NodeManager::new(config).await;
        assert!(node_manager.is_ok());
    }

    #[tokio::test]
    async fn test_node_creation_and_management() {
        let config = NodeConfig::default();
        let node_manager = NodeManager::new(config).await.unwrap();

        // Crée un nœud Full Archive
        let node_type = NodeType::FullArchive {
            storage_capacity: 20_000_000_000_000,
            replication_factor: 10,
        };

        let node_id = node_manager.create_node(node_type, None).await.unwrap();
        assert!(!node_id.hash().is_zero());

        // Vérifie que le nœud est enregistré
        let managed_nodes = node_manager.get_managed_nodes().await;
        assert!(managed_nodes.contains(&node_id));

        // Démarre le nœud
        assert!(node_manager.start_node(&node_id).await.is_ok());

        // Effectue un health check
        let health_results = node_manager.health_check_all_nodes().await.unwrap();
        assert!(health_results.contains_key(&node_id));

        // Arrête le nœud
        assert!(node_manager.stop_node(&node_id).await.is_ok());
    }

    #[test]
    fn test_maintenance_task() {
        let task = MaintenanceTask {
            node_id: NodeId::from(Hash::zero()),
            task_type: MaintenanceType::Restart,
            started_at: SystemTime::now(),
            estimated_duration: Duration::from_secs(30),
            status: MaintenanceStatus::InProgress,
        };

        assert_eq!(task.task_type, MaintenanceType::Restart);
        assert_eq!(task.status, MaintenanceStatus::InProgress);
    }

    #[test]
    fn test_node_event() {
        let event = NodeEvent {
            timestamp: chrono::Utc::now(),
            node_id: NodeId::from(Hash::zero()),
            event_type: NodeEventType::NodeStarted,
            message: "Test event".to_string(),
            severity: EventSeverity::Info,
        };

        assert_eq!(event.event_type, NodeEventType::NodeStarted);
        assert_eq!(event.severity, EventSeverity::Info);
    }
}